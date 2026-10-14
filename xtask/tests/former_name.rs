//! The former name of the project, vkxp, stays only where `former-name.allow` says why: the
//! requirement "The former name only where it is kept" of `workspace-structure`.
//!
//! Every tracked file outside `openspec/` is checked: its path, and each line of its text
//! when it is text. `openspec/` is left out because its archived changes are history, its
//! main specs change only through deltas, and its open changes may describe the rename.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex_lite::Regex;

/// The name that is checked for, in any case.
const FORMER: &str = "vkxp";

/// One rule of the allow-list.
#[derive(Debug)]
enum Rule {
    /// A tracked path that may hold the name.
    Path(String),
    /// What the lines of the files whose paths match the glob may hold.
    Text(String, Regex),
}

/// Reads the allow-list: entries separated by blank lines, each a reason in comment lines
/// followed by rules. An entry of a reason alone is a comment. The error names the line.
fn parse(text: &str) -> Result<Vec<Rule>, String> {
    let mut rules = Vec::new();
    let mut reason = false;
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let line = line.trim_end();
        if line.is_empty() {
            reason = false;
        } else if line.starts_with('#') {
            reason = true;
        } else if !reason {
            return Err(format!("line {number}: a rule without a reason: {line}"));
        } else if let Some(glob) = line.strip_prefix("path ") {
            rules.push(Rule::Path(glob.to_owned()));
        } else if let Some(rest) = line.strip_prefix("text ") {
            let (glob, pattern) = rest
                .split_once(' ')
                .ok_or_else(|| format!("line {number}: `text` needs a glob and a regex"))?;
            let regex = Regex::new(pattern).map_err(|error| format!("line {number}: {error}"))?;
            rules.push(Rule::Text(glob.to_owned(), regex));
        } else {
            return Err(format!("line {number}: neither `path` nor `text`: {line}"));
        }
    }
    Ok(rules)
}

/// Whether `path` matches `glob`, in which `**` stands for any part of a path and `*` and `?`
/// for any part and any one character of a name.
fn matches(glob: &str, path: &str) -> bool {
    fn from(glob: &[u8], path: &[u8]) -> bool {
        match glob {
            [] => path.is_empty(),
            [b'*', b'*', rest @ ..] => (0..=path.len()).any(|at| from(rest, &path[at..])),
            [b'*', rest @ ..] => {
                for at in 0..=path.len() {
                    if from(rest, &path[at..]) {
                        return true;
                    }
                    if path.get(at) == Some(&b'/') {
                        break;
                    }
                }
                false
            }
            [b'?', rest @ ..] => {
                matches!(path.first(), Some(byte) if *byte != b'/') && from(rest, &path[1..])
            }
            [byte, rest @ ..] => path.first() == Some(byte) && from(rest, &path[1..]),
        }
    }
    from(glob.as_bytes(), path.as_bytes())
}

/// Whether `text` names the former name in any case.
fn names_former(text: &str) -> bool {
    text.to_ascii_lowercase().contains(FORMER)
}

/// What the allow-list does not allow in the tracked file `path` with `text`, if it is text:
/// the path, and `path:line` for each line, numbered from 1.
fn check(rules: &[Rule], path: &str, text: Option<&str>) -> Vec<String> {
    let mut reported = Vec::new();
    let path_allowed = rules
        .iter()
        .any(|rule| matches!(rule, Rule::Path(glob) if matches(glob, path)));
    if names_former(path) && !path_allowed {
        reported.push(path.to_owned());
    }
    let regexes: Vec<&Regex> = rules
        .iter()
        .filter_map(|rule| match rule {
            Rule::Text(glob, regex) if matches(glob, path) => Some(regex),
            _ => None,
        })
        .collect();
    for (index, line) in text.into_iter().flat_map(str::lines).enumerate() {
        if !names_former(line) {
            continue;
        }
        let rest = regexes.iter().fold(line.to_owned(), |rest, regex| {
            regex.replace_all(&rest, "").into_owned()
        });
        if names_former(&rest) {
            reported.push(format!("{path}:{}", index + 1));
        }
    }
    reported
}

/// The root of the repository.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The committed allow-list.
fn allow_list() -> Vec<Rule> {
    let file = root().join("xtask/tests/former-name.allow");
    let text = fs::read_to_string(&file).unwrap_or_else(|error| panic!("{file:?}: {error}"));
    parse(&text).unwrap_or_else(|error| panic!("former-name.allow: {error}"))
}

/// Scenario "The repository".
#[test]
fn the_former_name_stays_only_where_the_allow_list_keeps_it() {
    let root = root();
    let listed = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(&root)
        .output()
        .expect("git runs");
    assert!(listed.status.success(), "git ls-files failed");
    let listed = String::from_utf8(listed.stdout).expect("the tracked paths are UTF-8");
    let rules = allow_list();
    let mut reported = Vec::new();
    for path in listed.split('\0').filter(|path| !path.is_empty()) {
        if path.starts_with("openspec/") {
            continue;
        }
        // A file the checkout lacks, or a binary file, has its path checked alone.
        let bytes = fs::read(root.join(path)).unwrap_or_default();
        let text = String::from_utf8(bytes)
            .ok()
            .filter(|text| !text.contains('\0'));
        reported.extend(check(&rules, path, text.as_deref()));
    }
    assert!(
        reported.is_empty(),
        "the former name `{FORMER}` appears where xtask/tests/former-name.allow does not keep \
         it; write the new name, vkrgss, or add an entry with its reason:\n{}",
        reported.join("\n")
    );
}

/// Scenario "A new mention".
#[test]
fn a_line_that_names_the_former_name_is_reported_with_its_number() {
    let text = "# Running\n\nline 3\nline 4\nline 5\nline 6\nThe program vkxp runs a game.\n";
    assert_eq!(
        check(&allow_list(), "docs/running.md", Some(text)),
        ["docs/running.md:7"]
    );
    let text = "Run VKXP or Vkxp.\n";
    assert_eq!(
        check(&allow_list(), "docs/building.md", Some(text)),
        ["docs/building.md:1"]
    );
}

/// Scenario "A path".
#[test]
fn a_path_that_holds_the_former_name_is_reported() {
    let text = "//! The core.\n";
    assert_eq!(
        check(&allow_list(), "crates/vkxp-core/src/lib.rs", Some(text)),
        ["crates/vkxp-core/src/lib.rs"]
    );
}

/// Scenario "An identifier of the vendored interpreters".
#[test]
fn an_identifier_of_the_vendored_interpreters_is_not_reported() {
    let text = "    pub fn ruby_vkxp_stack_size(size: usize);\n";
    let path = "crates/vkrgss-ruby-sys/src/ffi.rs";
    assert!(check(&allow_list(), path, Some(text)).is_empty());
}

/// Scenario "A vendored source".
#[test]
fn a_vendored_source_is_not_reported() {
    let text = "/* vkxp: keep the counter. */\n";
    assert!(check(&allow_list(), "third_party/ruby-1.8.7/gc.c", Some(text)).is_empty());
}

/// Scenario "The document of a vendored tree".
#[test]
fn the_document_of_a_vendored_tree_under_the_former_name_is_reported() {
    let path = "third_party/ruby-1.8.7/README.vkxp.md";
    assert_eq!(check(&allow_list(), path, Some("# Ruby 1.8.7\n")), [path]);
}

/// Scenario "Other variables" of the requirement "Environment variables under the former
/// name": the variables of the former name that the allow-list allows in the sources of the
/// program are the four a player sets; the other `VKXP_` names it allows are macros of the
/// vendored sources.
#[test]
fn the_allow_list_allows_only_the_four_variables_a_player_sets() {
    let vendored_macros = [
        "VKXP_ADDRESS_SANITIZER",
        "VKXP_FIRST_PROVISIONAL",
        "VKXP_GC_STRESS_DUE",
        "VKXP_LAST_MEASURED_DEFAULT",
        "VKXP_STACK_ADDRESSABLE",
    ];
    let names = Regex::new(r"VKXP_[A-Z0-9_]+").unwrap();
    let alternatives = Regex::new(r"VKXP_\(([A-Z0-9_|]+)\)").unwrap();
    let mut allowed = BTreeSet::new();
    for rule in allow_list() {
        let Rule::Text(glob, regex) = rule else {
            continue;
        };
        if !(glob.starts_with("crates/") || glob.starts_with("xtask/src") || glob == "**") {
            continue;
        }
        let source = regex.as_str();
        for found in alternatives.captures_iter(source) {
            allowed.extend(found[1].split('|').map(|name| format!("VKXP_{name}")));
        }
        allowed.extend(
            names
                .find_iter(source)
                .map(|found| found.as_str().to_owned()),
        );
    }
    let allowed: BTreeSet<&str> = allowed
        .iter()
        .map(String::as_str)
        .filter(|name| !vendored_macros.contains(name))
        .collect();
    let four = [
        "VKXP_GUEST",
        "VKXP_LOG",
        "VKXP_PRESENT_TIMING",
        "VKXP_SANDBOX",
    ];
    assert_eq!(allowed, BTreeSet::from(four));

    // Another variable of the former name in the sources of the program is reported.
    let line = "    let device = std::env::var(\"VKXP_TEST_DEVICE\");\n";
    assert_eq!(
        check(&allow_list(), "crates/vkrgss/src/main.rs", Some(line)),
        ["crates/vkrgss/src/main.rs:1"]
    );
}

#[test]
fn an_entry_without_a_reason_is_an_error() {
    assert!(parse("# Why.\ntext README.md vkxp\n\npath a/vkxp.md\n").is_err());
    assert!(parse("# Why.\npath a/vkxp.md\ntext b.md vkxp\n").is_ok());
    assert!(parse("# Why.\nkeep a.md\n").is_err());
}

#[test]
fn globs_match_parts_of_paths() {
    assert!(matches("third_party/**", "third_party/ruby-1.8.7/gc.c"));
    assert!(matches("**", "README.md"));
    assert!(matches(
        "crates/*/src/lib.rs",
        "crates/vkrgss-core/src/lib.rs"
    ));
    assert!(!matches("crates/*/lib.rs", "crates/vkrgss-core/src/lib.rs"));
    assert!(matches("docs/?ecisions.md", "docs/decisions.md"));
    assert!(!matches("README.md", "docs/README.md"));
}
