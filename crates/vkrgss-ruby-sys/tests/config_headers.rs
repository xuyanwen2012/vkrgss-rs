//! The configuration headers of aarch64 are those of x86_64 with the platform strings of
//! aarch64. Below their leading comments the two differ only in the definitions that name the
//! platform, so that they cannot drift apart.

use std::fs;
use std::path::Path;

const X86_64: &str = "x86_64-unknown-linux-gnu";
const AARCH64: &str = "aarch64-unknown-linux-gnu";

fn read(target: &str, header: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
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

#[test]
fn the_ruby_header_of_aarch64_differs_only_in_the_platform_strings() {
    assert_only_the_platform_differs(
        "config.h",
        &[
            "RUBY_PLATFORM",
            "RUBY_ARCHLIB",
            "RUBY_SITE_ARCHLIB",
            "RUBY_VENDOR_ARCHLIB",
        ],
    );
}

#[test]
fn the_oniguruma_header_of_aarch64_differs_only_in_its_comment() {
    assert_only_the_platform_differs("oniguruma/config.h", &[]);
}
