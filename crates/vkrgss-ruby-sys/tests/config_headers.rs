//! The configuration headers of aarch64 are those of x86_64 with the platform strings of
//! aarch64. Below their leading comments the two differ only in the definitions that name the
//! platform, so that they cannot drift apart. Those of Android are those of aarch64 without
//! the definitions of what bionic lacks and with the platform strings of Android.
//!
//! The header of Ruby for aarch64-apple-darwin is the one `configure` wrote on the Mac, which
//! differs from those of Linux in more than the platform: its platform strings are checked.
//! Its header of Oniguruma is that of aarch64 Linux.

mod common;

use std::fs;

const X86_64: &str = "x86_64-unknown-linux-gnu";
const AARCH64: &str = "aarch64-unknown-linux-gnu";
const ANDROID: &str = "aarch64-linux-android";
const DARWIN: &str = "aarch64-apple-darwin";

/// The definitions that name the platform in the header of the interpreter.
const PLATFORM: &[&str] = &[
    "RUBY_PLATFORM",
    "RUBY_ARCHLIB",
    "RUBY_SITE_ARCHLIB",
    "RUBY_VENDOR_ARCHLIB",
];

fn read(target: &str, header: &str) -> String {
    let path = common::manifest_dir()
        .join("config")
        .join(target)
        .join(header);
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

/// Asserts that, below the leading comments, the header of aarch64 is that of x86_64 with
/// `x86_64-linux` replaced by `aarch64-linux` in the definitions `platform` names, in that
/// order, and nowhere else, and that its leading comment names aarch64.
fn assert_only_the_platform_differs(header: &str, platform: &[&str]) {
    let x86_64 = read(X86_64, header);
    let aarch64 = read(AARCH64, header);
    let (_, x86_64) = split_comments(&x86_64);
    let (comment, aarch64) = split_comments(&aarch64);
    assert!(
        comment.contains("aarch64"),
        "{header}: the leading comment does not name aarch64"
    );
    assert_eq!(
        x86_64.lines().count(),
        aarch64.lines().count(),
        "{header}: the two have a different number of lines"
    );
    let mut differing = Vec::new();
    for (x86_64_line, aarch64_line) in x86_64.lines().zip(aarch64.lines()) {
        if x86_64_line == aarch64_line {
            continue;
        }
        let name = x86_64_line
            .strip_prefix("#define ")
            .and_then(|definition| definition.split_once(' '))
            .map(|(name, _)| name);
        assert!(
            name.is_some_and(|name| platform.contains(&name))
                && x86_64_line.replace("x86_64-linux", "aarch64-linux") == aarch64_line,
            "{header}: `{x86_64_line}` differs from `{aarch64_line}`"
        );
        differing.extend(name);
    }
    assert_eq!(differing, platform, "{header}: the platform strings");
}

/// The name of a definition line.
fn defined_name(line: &str) -> Option<&str> {
    line.strip_prefix("#define ")
        .map(|definition| definition.split([' ', '(']).next().unwrap_or(definition))
}

/// Asserts that, below the leading comments, the header of Android is that of aarch64
/// without the definitions `removed` names, each of which the header of aarch64 has, and with
/// `aarch64-linux` replaced by `aarch64-linux-android` in the definitions `platform` names,
/// and that its leading comment names Android.
fn assert_only_bionic_differs(header: &str, removed: &[&str], platform: &[&str]) {
    let aarch64 = read(AARCH64, header);
    let android = read(ANDROID, header);
    let (_, aarch64) = split_comments(&aarch64);
    let (comment, android) = split_comments(&android);
    assert!(
        comment.contains("aarch64-linux-android"),
        "{header}: the leading comment does not name aarch64-linux-android"
    );
    for name in removed {
        assert!(
            aarch64.lines().any(|line| defined_name(line) == Some(name)),
            "{header}: the header of aarch64 does not define {name}"
        );
    }
    let expected: Vec<String> = aarch64
        .lines()
        .filter(|line| !defined_name(line).is_some_and(|name| removed.contains(&name)))
        .map(|line| match defined_name(line) {
            Some(name) if platform.contains(&name) => {
                line.replace("aarch64-linux", "aarch64-linux-android")
            }
            _ => line.to_owned(),
        })
        .collect();
    let actual: Vec<&str> = android.lines().collect();
    assert_eq!(actual, expected, "{header}: the header of Android");
}

#[test]
fn the_ruby_header_of_aarch64_differs_only_in_the_platform_strings() {
    assert_only_the_platform_differs("config.h", PLATFORM);
}

/// bionic has no `crypt(3)` and no libcrypt, no `eaccess`, `lchmod` or `group_member`, and an
/// opaque `FILE`.
#[test]
fn the_ruby_header_of_android_lacks_only_what_bionic_lacks() {
    assert_only_bionic_differs(
        "config.h",
        &[
            "HAVE_LIBCRYPT",
            "HAVE_CRYPT",
            "HAVE_EACCESS",
            "HAVE_LCHMOD",
            "HAVE_GROUP_MEMBER",
            "FILE_READPTR",
            "FILE_READEND",
        ],
        PLATFORM,
    );
}

#[test]
fn the_oniguruma_header_of_android_differs_only_in_its_comment() {
    assert_only_bionic_differs("oniguruma/config.h", &[], &[]);
}

#[test]
fn the_oniguruma_header_of_aarch64_differs_only_in_its_comment() {
    assert_only_the_platform_differs("oniguruma/config.h", &[]);
}

#[test]
fn the_ruby_header_of_darwin_names_aarch64_darwin() {
    let header = read(DARWIN, "config.h");
    let (comment, _) = split_comments(&header);
    assert!(
        comment.contains("aarch64-darwin"),
        "the leading comment does not name aarch64-darwin"
    );
    let platform: Vec<&str> = header
        .lines()
        .filter(|line| line.contains("ARCHLIB ") || line.contains("RUBY_PLATFORM "))
        .collect();
    assert_eq!(
        platform,
        [
            "#define RUBY_PLATFORM \"aarch64-darwin\"",
            "#define RUBY_ARCHLIB \"/usr/local/lib/ruby/1.8/aarch64-darwin\"",
            "#define RUBY_SITE_ARCHLIB \"/usr/local/lib/ruby/site_ruby/1.8/aarch64-darwin\"",
            "#define RUBY_VENDOR_ARCHLIB \"/usr/local/lib/ruby/vendor_ruby/1.8/aarch64-darwin\"",
        ]
    );
}

#[test]
fn the_oniguruma_header_of_darwin_is_that_of_aarch64_linux() {
    let linux = read(AARCH64, "oniguruma/config.h");
    let darwin = read(DARWIN, "oniguruma/config.h");
    let (comment, darwin) = split_comments(&darwin);
    assert!(
        comment.contains("aarch64-apple-darwin"),
        "the leading comment does not name aarch64-apple-darwin"
    );
    assert_eq!(split_comments(&linux).1, darwin);
}
