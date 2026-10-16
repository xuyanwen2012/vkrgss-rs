//! The configuration header of aarch64 is that of x86_64 with the platform string of aarch64.
//! Below their leading comments the two differ only in `RUBY_PLATFORM`, so that they cannot
//! drift apart.

use std::fs;
use std::path::Path;

const X86_64: &str = "x86_64-unknown-linux-gnu";
const AARCH64: &str = "aarch64-unknown-linux-gnu";

fn read(target: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("config")
        .join(target)
        .join("ruby/config.h");
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The leading comments of a header, and the rest.
fn split_comments(header: &str) -> (&str, &str) {
    let mut rest = header.trim_start();
    while let Some(comment) = rest.strip_prefix("/*") {
        let end = comment.find("*/").expect("the comment ends");
        rest = comment[end + 2..].trim_start();
    }
    (&header[..header.len() - rest.len()], rest)
}

#[test]
fn the_header_of_aarch64_differs_only_in_ruby_platform() {
    let x86_64 = read(X86_64);
    let aarch64 = read(AARCH64);
    let (_, x86_64) = split_comments(&x86_64);
    let (comment, aarch64) = split_comments(&aarch64);
    assert!(
        comment.contains("aarch64"),
        "the leading comment does not name aarch64"
    );
    assert_eq!(x86_64.lines().count(), aarch64.lines().count());
    let differing: Vec<(&str, &str)> = x86_64
        .lines()
        .zip(aarch64.lines())
        .filter(|(x86_64_line, aarch64_line)| x86_64_line != aarch64_line)
        .collect();
    assert_eq!(
        differing,
        [(
            "#define RUBY_PLATFORM \"x86_64-linux\"",
            "#define RUBY_PLATFORM \"aarch64-linux\""
        )]
    );
}
