//! The vendored trees must match their manifests and their patch logs.
//!
//! `MANIFEST.sha256` holds the upstream hash of every vendored file. A file may differ from it
//! only when an entry of `PATCHES.md` names the file, and then it must match the patched hash
//! of that entry. Files may be neither added nor removed without a record. A tree without a
//! `PATCHES.md` has no patches.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Files that vkxp adds next to the release and that the manifest therefore does not list.
const OWN_FILES: &[&str] = &["MANIFEST.sha256", "PATCHES.md", "README.vkxp.md"];

/// Directory of the patch files, which vkxp adds as well.
const PATCH_DIR: &str = "patches";

fn vendored_root(tree: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party")
        .join(tree)
}

fn sha256_hex(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Parses the `sha256sum` format: 64 hex digits, two spaces, the path.
fn parse_manifest(text: &str) -> BTreeMap<String, String> {
    let mut entries = BTreeMap::new();
    for line in text.lines() {
        let (hash, path) = line
            .split_once("  ")
            .unwrap_or_else(|| panic!("malformed manifest line: {line:?}"));
        assert!(
            hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
            "malformed hash in manifest line: {line:?}"
        );
        let previous = entries.insert(path.to_owned(), hash.to_owned());
        assert!(previous.is_none(), "{path} is listed twice in the manifest");
    }
    entries
}

/// One entry of `PATCHES.md`.
#[derive(Debug, Default)]
struct Patch {
    title: String,
    file: Option<String>,
    patch: Option<String>,
    patched_sha256: Option<String>,
    has_reason: bool,
}

/// The value of a `- Name: value` field line, with the backticks around it removed.
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let value = line
        .strip_prefix("- ")?
        .strip_prefix(name)?
        .strip_prefix(':')?;
    Some(value.trim().trim_matches('`'))
}

/// Parses the patch log: every `## ` heading starts an entry made of field lines.
fn parse_patches(text: &str) -> Vec<Patch> {
    let mut patches: Vec<Patch> = Vec::new();
    for line in text.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            patches.push(Patch {
                title: title.to_owned(),
                ..Patch::default()
            });
            continue;
        }
        let Some(current) = patches.last_mut() else {
            continue;
        };
        if let Some(value) = field(line, "File") {
            current.file = Some(value.to_owned());
        } else if let Some(value) = field(line, "Patch") {
            current.patch = Some(value.to_owned());
        } else if let Some(value) = field(line, "Patched SHA-256") {
            current.patched_sha256 = Some(value.to_owned());
        } else if let Some(value) = field(line, "Reason") {
            current.has_reason = !value.is_empty();
        }
    }
    patches
}

/// Every file below `dir`, as a path relative to `root` with `/` separators.
fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_files(root, &path, out);
        } else {
            let relative = path.strip_prefix(root).expect("path below the root");
            let parts: Vec<_> = relative
                .components()
                .map(|part| part.as_os_str().to_str().expect("UTF-8 path"))
                .collect();
            out.push(parts.join("/"));
        }
    }
}

/// Checks one tree below `third_party` against its records.
fn check_tree(tree: &str) {
    let root = vendored_root(tree);
    let manifest = parse_manifest(
        &fs::read_to_string(root.join("MANIFEST.sha256")).expect("MANIFEST.sha256 is readable"),
    );
    let patch_log = root.join("PATCHES.md");
    let patches = if patch_log.is_file() {
        parse_patches(&fs::read_to_string(patch_log).expect("PATCHES.md is readable"))
    } else {
        Vec::new()
    };

    let mut problems: Vec<String> = Vec::new();

    // The patch log: complete entries, existing patch files, one patched hash per file.
    let mut patched: BTreeMap<String, String> = BTreeMap::new();
    let mut patch_files: Vec<String> = Vec::new();
    for entry in &patches {
        let (Some(file), Some(patch), Some(hash)) =
            (&entry.file, &entry.patch, &entry.patched_sha256)
        else {
            problems.push(format!(
                "PATCHES.md entry `{}` lacks a File, Patch or Patched SHA-256 field",
                entry.title
            ));
            continue;
        };
        if !entry.has_reason {
            problems.push(format!(
                "PATCHES.md entry `{}` gives no Reason",
                entry.title
            ));
        }
        if !manifest.contains_key(file) {
            problems.push(format!(
                "PATCHES.md entry `{}` patches {file}, which the manifest does not list",
                entry.title
            ));
        }
        if !root.join(patch).is_file() {
            problems.push(format!(
                "PATCHES.md entry `{}` names the patch file {patch}, which does not exist",
                entry.title
            ));
        }
        patch_files.push(patch.clone());
        if let Some(other) = patched.insert(file.clone(), hash.to_ascii_lowercase())
            && other != hash.to_ascii_lowercase()
        {
            problems.push(format!(
                "PATCHES.md records two different patched hashes for {file}"
            ));
        }
    }

    // The tree: every file is listed, and has the upstream hash or its recorded patched hash.
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files);
    files.sort();
    for file in &files {
        if OWN_FILES.contains(&file.as_str()) {
            continue;
        }
        if file.starts_with(&format!("{PATCH_DIR}/")) {
            if !patch_files.contains(file) {
                problems.push(format!("{file}: patch file that no PATCHES.md entry names"));
            }
            continue;
        }
        let Some(upstream) = manifest.get(file) else {
            problems.push(format!(
                "{file}: present in the tree but not in the manifest"
            ));
            continue;
        };
        let actual = sha256_hex(&root.join(file));
        match patched.get(file) {
            Some(expected) if &actual == expected => {}
            Some(_) if &actual == upstream => problems.push(format!(
                "{file}: PATCHES.md records a patch, but the file is still the upstream one"
            )),
            Some(expected) => problems.push(format!(
                "{file}: hash {actual} is not the patched hash {expected} that PATCHES.md records"
            )),
            None if &actual == upstream => {}
            None => problems.push(format!(
                "{file}: hash {actual} differs from the manifest's {upstream} and no patch is \
                 recorded for it"
            )),
        }
    }
    for file in manifest.keys() {
        if !files.contains(file) {
            problems.push(format!(
                "{file}: listed in the manifest but missing from the tree"
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "the vendored tree {tree} does not match its records:\n  {}",
        problems.join("\n  ")
    );
}

#[test]
fn manifest_and_patch_log_describe_the_vendored_tree() {
    check_tree("ruby-1.8.7");
}

#[test]
fn manifest_describes_the_vendored_engine() {
    check_tree("oniguruma-5.9.6");
}
