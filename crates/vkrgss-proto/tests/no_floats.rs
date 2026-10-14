//! No floating-point type may appear in the protocol, so two backends can never disagree
//! through float rounding.
//!
//! The check reads the crate's source: outside comments, the words `f32` and `f64` must not
//! occur anywhere in `vkrgss-proto`. That is stricter than inspecting message fields and covers
//! every type reachable from a message, since all of them are defined in this crate.

use std::fs;
use std::path::{Path, PathBuf};

const FLOAT_TYPES: [&str; 2] = ["f32", "f64"];

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// The identifier-like words of a line, ignoring anything after `//`.
fn code_words(line: &str) -> impl Iterator<Item = &str> {
    let code = line.split("//").next().unwrap_or("");
    code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
}

fn float_uses(source: &str) -> Vec<(usize, &str)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| code_words(line).any(|word| FLOAT_TYPES.contains(&word)))
        .map(|(index, line)| (index + 1, line))
        .collect()
}

#[test]
fn protocol_source_has_no_float_types() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(
        files.len() >= 8,
        "expected the protocol sources, found {files:?}"
    );

    let mut report = String::new();
    for file in &files {
        let source = fs::read_to_string(file).unwrap();
        for (line_number, line) in float_uses(&source) {
            report.push_str(&format!(
                "{}:{line_number}: {}\n",
                file.display(),
                line.trim()
            ));
        }
    }
    assert!(
        report.is_empty(),
        "floating-point types in the protocol:\n{report}"
    );
}

#[test]
fn the_check_finds_float_fields() {
    assert_eq!(float_uses("pub struct P {\n    pub x: f32,\n}").len(), 1);
    assert_eq!(float_uses("pub zoom: Option<f64>,").len(), 1);
    assert_eq!(
        float_uses("let x = 1.0_f32;").len(),
        0,
        "suffix is one word, `0_f32`"
    );
    assert_eq!(float_uses("fn scale(v: f64) -> i32").len(), 1);
}

#[test]
fn the_check_ignores_comments_and_longer_words() {
    assert_eq!(
        float_uses("/// Never an f32.\npub x: i32, // not f64").len(),
        0
    );
    assert_eq!(float_uses("pub buf32: u32, pub f320: u8").len(), 0);
}
