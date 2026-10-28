//! `docs/architecture.md` against the workspace: its crate table names exactly the members of
//! the workspace, its list of where `unsafe` compiles agrees with the `unsafe_code` attributes
//! of the crates and their modules, and every sentence of the table or of the top doc comment
//! of an interpreter crate that names targets names exactly those of the crate's list.
//!
//! The parsing is simple on purpose: a change of the document's format that it does not
//! understand fails the test instead of passing it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The heading of the table of crates.
const TABLE_HEADER: &str = "| Crate | Responsibility | Side |";

/// The starts of the three items of the list of where `unsafe` compiles.
const FORBIDDEN: &str = "- Forbidden";
const DENIED: &str = "- Denied";
const ALLOWED: &str = "- Allowed";

/// The interpreter crates and the file, below the crate's directory, that holds the `TARGETS`
/// they are built for. `vkrgss-ruby-sys` keeps the list in its build script.
const INTERPRETERS: [(&str, &str); 2] = [
    ("vkrgss-ruby-sys", "build.rs"),
    ("vkrgss-ruby192-sys", "targets.rs"),
];

/// What the attributes of a crate say of `unsafe`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Unsafe {
    /// `#![forbid(unsafe_code)]` at the root.
    Forbidden,
    /// `#![deny(unsafe_code)]` at the root, and the modules that allow it.
    Denied(BTreeSet<String>),
    /// No attribute at the root.
    Allowed,
}

/// What the check compares the document with.
struct Workspace {
    /// The names of the packages of the workspace.
    members: BTreeSet<String>,
    /// What each package's attributes say of `unsafe`.
    unsafe_code: BTreeMap<String, Unsafe>,
    /// The targets of each interpreter crate, and the top doc comment of its root.
    interpreters: BTreeMap<String, (BTreeSet<String>, String)>,
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}

/// The workspace as `cargo metadata` and the sources describe it.
fn workspace() -> Workspace {
    let root = repository();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(&root)
        .output()
        .expect("cargo metadata starts");
    assert!(output.status.success(), "cargo metadata: {}", output.status);
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut workspace = Workspace {
        members: BTreeSet::new(),
        unsafe_code: BTreeMap::new(),
        interpreters: BTreeMap::new(),
    };
    for package in metadata["packages"].as_array().unwrap() {
        let name = package["name"].as_str().unwrap().to_owned();
        let roots: Vec<PathBuf> = package["targets"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|target| {
                target["kind"].as_array().unwrap().iter().any(|kind| {
                    ["lib", "bin", "cdylib", "staticlib"].contains(&kind.as_str().unwrap())
                })
            })
            .map(|target| PathBuf::from(target["src_path"].as_str().unwrap()))
            .collect();
        let [crate_root] = &roots[..] else {
            panic!("{name}: the check knows one root per package, not {roots:?}");
        };
        workspace
            .unsafe_code
            .insert(name.clone(), unsafe_of(crate_root));
        if let Some((_, file)) = INTERPRETERS
            .iter()
            .find(|(crate_name, _)| *crate_name == name)
        {
            let manifest = PathBuf::from(package["manifest_path"].as_str().unwrap());
            let list = manifest.parent().unwrap().join(file);
            let targets = targets_of(&read(&list), &list);
            let doc = top_doc(&read(crate_root));
            workspace.interpreters.insert(name.clone(), (targets, doc));
        }
        workspace.members.insert(name);
    }
    for (name, _) in INTERPRETERS {
        assert!(
            workspace.interpreters.contains_key(name),
            "{name} is not in the workspace"
        );
    }
    workspace
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// What the attributes of the crate whose root is `crate_root` say of `unsafe`: the attribute
/// at the root, and for a crate that denies it the modules that allow it, those whose `mod`
/// line in the root follows `#[allow(unsafe_code)]` and those whose file below the root's
/// directory holds such an attribute.
fn unsafe_of(crate_root: &Path) -> Unsafe {
    let text = read(crate_root);
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    if lines.contains(&"#![forbid(unsafe_code)]") {
        return Unsafe::Forbidden;
    }
    if !lines.contains(&"#![deny(unsafe_code)]") {
        return Unsafe::Allowed;
    }
    let mut modules = BTreeSet::new();
    for (index, _) in lines
        .iter()
        .enumerate()
        .filter(|(_, line)| **line == "#[allow(unsafe_code)]")
    {
        let item = lines[index + 1..]
            .iter()
            .find(|line| !line.starts_with("#["))
            .unwrap();
        let module = item
            .trim_start_matches("pub ")
            .trim_start_matches("pub(crate) ")
            .strip_prefix("mod ")
            .and_then(|rest| rest.strip_suffix(';'))
            .unwrap_or_else(|| {
                panic!(
                    "{}: `#[allow(unsafe_code)]` before `{item}`, which the check does not \
                     know: it knows it before a `mod` line",
                    crate_root.display()
                )
            });
        modules.insert(module.to_owned());
    }
    let dir = crate_root.parent().unwrap();
    for file in rust_files(dir) {
        if file == crate_root || !read(&file).contains("#[allow(unsafe_code)]") {
            continue;
        }
        let relative = file.strip_prefix(dir).unwrap().with_extension("");
        let parts: Vec<String> = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .filter(|part| part != "mod")
            .collect();
        modules.insert(parts.join("::"));
    }
    Unsafe::Denied(modules)
}

/// The Rust files below `dir`, recursively.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

/// The strings of the `TARGETS` array of the Rust source `text`.
fn targets_of(text: &str, file: &Path) -> BTreeSet<String> {
    let start = text
        .find("TARGETS: &[&str] = &[")
        .unwrap_or_else(|| panic!("{}: no `TARGETS: &[&str] = &[`", file.display()));
    let rest = &text[start..];
    let list = &rest[rest.find("= &[").unwrap() + 4..rest.find("];").unwrap()];
    let targets: BTreeSet<String> = list
        .split(',')
        .map(|item| item.trim().trim_matches('"').to_owned())
        .filter(|item| !item.is_empty())
        .collect();
    assert!(
        !targets.is_empty(),
        "{}: `TARGETS` is empty",
        file.display()
    );
    targets
}

/// The top doc comment of the Rust source `text`, its `//!` lines without the marker.
fn top_doc(text: &str) -> String {
    text.lines()
        .filter_map(|line| line.strip_prefix("//!"))
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The rows of the table of crates in `document`, each as the crate and the whole row.
fn table(document: &str) -> Vec<(String, String)> {
    let mut lines = document.lines().skip_while(|line| *line != TABLE_HEADER);
    assert!(
        lines.next().is_some(),
        "docs/architecture.md: no table with the heading `{TABLE_HEADER}`"
    );
    lines
        .skip(1)
        .take_while(|line| line.starts_with('|'))
        .map(|row| {
            let first = row.split('|').nth(1).unwrap().trim().trim_matches('`');
            (first.to_owned(), row.to_owned())
        })
        .collect()
}

/// The item of the list in `document` that starts with `start`, its lines joined.
fn item(document: &str, start: &str) -> String {
    let mut lines = document.lines().skip_while(|line| !line.starts_with(start));
    let first = lines.next().unwrap_or_else(|| {
        panic!("docs/architecture.md: no item `{start}` in the list of where `unsafe` compiles")
    });
    let mut text = first.to_owned();
    for line in lines.take_while(|line| line.starts_with("  ")) {
        text.push(' ');
        text.push_str(line.trim());
    }
    text
}

/// The words in backticks of `text`, crates and modules, without the attributes.
fn quoted(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .filter(|word| !word.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// Whether `word` has the form of the name of a crate of the workspace.
fn is_crate(word: &str) -> bool {
    word.starts_with("vkrgss") || word == "xtask"
}

/// What the list of where `unsafe` compiles in `document` says of each crate.
fn unsafe_in(document: &str) -> BTreeMap<String, Unsafe> {
    let mut stated = BTreeMap::new();
    for (start, unsafe_code) in [(FORBIDDEN, Unsafe::Forbidden), (ALLOWED, Unsafe::Allowed)] {
        for word in quoted(&item(document, start)) {
            if is_crate(&word) {
                stated.insert(word, unsafe_code.clone());
            }
        }
    }
    let denied = item(document, DENIED);
    let (_, list) = denied
        .split_once(':')
        .expect("the item `- Denied` lists its crates after a colon");
    for part in list.split(';') {
        let words = quoted(part);
        let Some((name, modules)) = words.split_first() else {
            continue;
        };
        assert!(is_crate(name), "`{name}` of `- Denied` is not a crate");
        let modules = modules.iter().filter(|word| !is_crate(word)).cloned();
        stated.insert(name.clone(), Unsafe::Denied(modules.collect()));
    }
    stated
}

/// The target triples named in `sentence`.
fn triples(sentence: &str) -> BTreeSet<String> {
    sentence
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .filter(|word| {
            word.matches('-').count() >= 2
                && ["x86_64-", "aarch64-", "i686-", "armv7-", "riscv64"]
                    .iter()
                    .any(|arch| word.starts_with(arch))
        })
        .map(str::to_owned)
        .collect()
}

/// The sentences of `text`: what ends at a full stop before white space or at the end.
fn sentences(text: &str) -> Vec<String> {
    let text = text.replace('\n', " ");
    let mut sentences = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for (index, &byte) in bytes.iter().enumerate() {
        let ends = byte == b'.' && bytes.get(index + 1).is_none_or(u8::is_ascii_whitespace);
        if ends || index + 1 == bytes.len() {
            sentences.push(text[start..=index].trim().to_owned());
            start = index + 1;
        }
    }
    sentences
}

/// Every way in which `document` disagrees with `workspace`.
fn problems(document: &str, workspace: &Workspace) -> Vec<String> {
    let mut problems = Vec::new();
    let rows = table(document);
    let listed: BTreeSet<String> = rows.iter().map(|(name, _)| name.clone()).collect();
    for name in workspace.members.difference(&listed) {
        problems.push(format!("the crate table lacks `{name}`, which is missing"));
    }
    for name in listed.difference(&workspace.members) {
        problems.push(format!(
            "the crate table names `{name}`, which is not a member"
        ));
    }

    let stated = unsafe_in(document);
    for (name, actual) in &workspace.unsafe_code {
        match (stated.get(name), actual) {
            (None, _) => problems.push(format!("the list of `unsafe` lacks `{name}`")),
            (Some(Unsafe::Denied(listed)), Unsafe::Denied(modules)) => {
                for module in modules.difference(listed) {
                    problems.push(format!(
                        "the list of `unsafe` omits the module `{module}` of `{name}`, which \
                         allows `unsafe_code`"
                    ));
                }
                for module in listed.difference(modules) {
                    problems.push(format!(
                        "the list of `unsafe` names the module `{module}` of `{name}`, which \
                         does not allow `unsafe_code`"
                    ));
                }
            }
            (Some(stated), actual) if stated != actual => problems.push(format!(
                "the list of `unsafe` says {stated:?} of `{name}`, its attributes {actual:?}"
            )),
            _ => {}
        }
    }
    for name in stated
        .keys()
        .filter(|name| !workspace.members.contains(*name))
    {
        problems.push(format!(
            "the list of `unsafe` names `{name}`, which is not a member"
        ));
    }

    for (name, (targets, doc)) in &workspace.interpreters {
        let row = rows
            .iter()
            .find(|(crate_name, _)| crate_name == name)
            .map(|(_, row)| row.as_str())
            .unwrap_or_default();
        for (place, text) in [
            ("its row of the crate table", row),
            ("its top doc comment", doc),
        ] {
            for sentence in sentences(text) {
                let named = triples(&sentence);
                if !named.is_empty() && &named != targets {
                    problems.push(format!(
                        "`{name}`, {place}: \"{sentence}\" names {named:?}, its list holds \
                         {targets:?}"
                    ));
                }
            }
        }
    }
    problems
}

fn architecture() -> String {
    read(&repository().join("docs/architecture.md"))
}

#[test]
fn the_architecture_document_matches_the_workspace() {
    let problems = problems(&architecture(), &workspace());
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn a_crate_missing_from_the_table_is_reported() {
    let document: String = architecture()
        .lines()
        .filter(|line| !line.starts_with("| `vkrgss-audio` |"))
        .map(|line| format!("{line}\n"))
        .collect();
    let problems = problems(&document, &workspace());
    assert_eq!(
        problems,
        ["the crate table lacks `vkrgss-audio`, which is missing"]
    );
}

#[test]
fn a_module_allowed_unsafe_but_not_listed_is_reported() {
    let workspace = workspace();
    let Some(Unsafe::Denied(modules)) = workspace.unsafe_code.get("vkrgss-guest") else {
        panic!("vkrgss-guest denies unsafe_code at its root");
    };
    assert!(modules.contains("seccomp"), "{modules:?}");
    let document = architecture().replacen("`seccomp`, ", "", 1);
    assert_ne!(document, architecture(), "the list names `seccomp`");
    let problems = problems(&document, &workspace);
    assert_eq!(
        problems,
        [
            "the list of `unsafe` omits the module `seccomp` of `vkrgss-guest`, which allows \
             `unsafe_code`"
        ]
    );
}

#[test]
fn a_sentence_that_names_other_targets_is_reported() {
    let workspace = workspace();
    let row = "| `vkrgss-ruby192-sys` | Ruby 1.9.2; empty on every target but \
               `x86_64-unknown-linux-gnu` | guest |";
    let document: String = architecture()
        .lines()
        .map(|line| {
            if line.starts_with("| `vkrgss-ruby192-sys` |") {
                format!("{row}\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    let problems = problems(&document, &workspace);
    assert_eq!(problems.len(), 1, "{problems:#?}");
    assert!(
        problems[0].contains("x86_64-unknown-linux-gnu"),
        "{problems:#?}"
    );
}
