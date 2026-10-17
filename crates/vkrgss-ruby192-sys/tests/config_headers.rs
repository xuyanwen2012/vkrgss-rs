//! The configuration header of aarch64 is that of x86_64 with the platform string of aarch64.
//! Below their leading comments the two differ only in `RUBY_PLATFORM`, so that they cannot
//! drift apart. That of Android is that of aarch64 without the definitions of what bionic
//! lacks and with the platform string of Android.
//!
//! The header for aarch64-apple-darwin is the one `configure` wrote on the Mac, which differs
//! from those of Linux in more than the platform: its platform string and the hand edits it
//! shares with them are checked.

use std::fs;
use std::path::PathBuf;

const X86_64: &str = "x86_64-unknown-linux-gnu";
const AARCH64: &str = "aarch64-unknown-linux-gnu";
const ANDROID: &str = "aarch64-linux-android";
const DARWIN: &str = "aarch64-apple-darwin";

/// The directory of this crate, or where the tests run on a phone (`cargo xtask
/// android-test`), the value of `CARGO_MANIFEST_DIR` when it is set.
fn manifest_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

fn read(target: &str) -> String {
    let path = manifest_dir()
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

/// The name of a definition line.
fn defined_name(line: &str) -> Option<&str> {
    line.strip_prefix("#define ")
        .map(|definition| definition.split([' ', '(']).next().unwrap_or(definition))
}

/// bionic has no `crypt(3)` and no libcrypt, no `eaccess` or `lchmod`, and an opaque `FILE`.
#[test]
fn the_header_of_android_lacks_only_what_bionic_lacks() {
    const REMOVED: &[&str] = &[
        "HAVE_LIBCRYPT",
        "HAVE_CRYPT",
        "HAVE_EACCESS",
        "HAVE_LCHMOD",
        "FILE_READPTR",
        "FILE_READEND",
    ];
    let aarch64 = read(AARCH64);
    let android = read(ANDROID);
    let (_, aarch64) = split_comments(&aarch64);
    let (comment, android) = split_comments(&android);
    assert!(
        comment.contains("aarch64-linux-android"),
        "the leading comment does not name aarch64-linux-android"
    );
    for name in REMOVED {
        assert!(
            aarch64.lines().any(|line| defined_name(line) == Some(name)),
            "the header of aarch64 does not define {name}"
        );
    }
    let expected: Vec<String> = aarch64
        .lines()
        .filter(|line| !defined_name(line).is_some_and(|name| REMOVED.contains(&name)))
        .map(|line| match defined_name(line) {
            Some("RUBY_PLATFORM") => line.replace("aarch64-linux", "aarch64-linux-android"),
            _ => line.to_owned(),
        })
        .collect();
    let actual: Vec<&str> = android.lines().collect();
    assert_eq!(actual, expected, "the header of Android");
}

#[test]
fn the_header_of_darwin_names_aarch64_darwin_and_has_the_hand_edits() {
    let header = read(DARWIN);
    let (comment, definitions) = split_comments(&header);
    assert!(
        comment.contains("aarch64-darwin"),
        "the leading comment does not name aarch64-darwin"
    );
    let lines: Vec<&str> = definitions.lines().collect();
    assert!(lines.contains(&"#define RUBY_PLATFORM \"aarch64-darwin\""));
    assert!(lines.contains(&"#define RUBY_EXEC_PREFIX \"/nonexistent-vkrgss-ruby192\""));
    assert!(!definitions.contains("STACK_END_ADDRESS"));
}
