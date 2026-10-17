//! Ruby's own test suites, run against the interpreter exactly as it was built for vkrgss.
//!
//! A wrong compiler flag does not fail the build: without `-fwrapv` the interpreter compiles
//! and then fails its overflow checks. Only these suites show it. A real `ruby` executable is
//! linked from the archives of the build script, because the suites start child interpreters
//! and expect a normal process. `upstream.allow` names what is left out, each with its reason.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use common::suites::{self, Suites};

/// What `ruby -v` must print, which names the platform of the target.
#[cfg(target_arch = "x86_64")]
const VERSION_LINE: &str = "ruby 1.9.2p0 (2010-08-18 revision 29036) [x86_64-linux]";
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
const VERSION_LINE: &str = "ruby 1.9.2p0 (2010-08-18 revision 29036) [aarch64-linux]";
#[cfg(target_os = "android")]
const VERSION_LINE: &str = "ruby 1.9.2p0 (2010-08-18 revision 29036) [aarch64-linux-android]";
#[cfg(target_os = "macos")]
const VERSION_LINE: &str = "ruby 1.9.2p0 (2010-08-18 revision 29036) [aarch64-darwin]";

/// How long one test process may run. The slowest file takes a few seconds.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(300);

fn built() -> &'static Suites {
    static BUILT: OnceLock<Suites> = OnceLock::new();
    BUILT.get_or_init(|| Suites::build("upstream", PROCESS_TIMEOUT, false))
}

#[test]
fn upstream_executable_reports_its_version() {
    let output = common::run(Command::new(&built().ruby).arg("-v"));
    assert_eq!(output.trim_end(), VERSION_LINE);
}

#[test]
fn bootstrap_tests_pass() {
    let skip = suites::allowed_tests();
    let files: Vec<String> = suites::test_files("bootstraptest")
        .into_iter()
        .filter(|file| !skip.contains(&format!("bootstraptest/{file}")))
        .collect();
    assert!(!files.is_empty(), "no file of bootstraptest is selected");
    let mut assertions = 0;
    let mut problems = Vec::new();
    for file in &files {
        match built().run_bootstrap_file(file) {
            Ok(count) => assertions += count,
            Err(problem) => problems.push(problem),
        }
    }
    eprintln!(
        "bootstraptest: {} files, {assertions} assertions",
        files.len()
    );
    assert!(
        problems.is_empty(),
        "{} file(s) of bootstraptest did not pass:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn upstream_core_suite_passes() {
    let skip = suites::allowed_tests();
    let files = suites::selected_core_files(&skip);
    let totals = built().run_unit_files(&files, &skip);
    eprintln!("test/ruby: {} files, {totals:?}", files.len());
}

/// Every entry of the lists says why, and names a file that exists.
#[test]
fn every_allowed_entry_has_a_reason() {
    for list in [suites::ALLOW_LIST, suites::ANDROID_ALLOW_LIST] {
        check_allow_list(list);
    }
}

fn check_allow_list(list: &str) {
    let allowed = suites::parse_allow_list(list);
    assert!(!allowed.is_empty(), "the list is empty");
    for entry in &allowed {
        assert!(!entry.reason.is_empty(), "{} has no reason", entry.test);
        let file = entry.test.split('#').next().expect("a file");
        let path = match file.strip_prefix("ruby/") {
            Some(_) => common::source_dir().join("test").join(file),
            None => common::source_dir().join(file),
        };
        assert!(
            file.starts_with("ruby/") || file.starts_with("bootstraptest/"),
            "{} names neither test/ruby nor bootstraptest",
            entry.test
        );
        assert!(path.is_file(), "{} names no vendored file", entry.test);
    }
}

#[test]
fn an_entry_without_a_reason_is_seen() {
    let allowed = suites::parse_allow_list("# Why.\nruby/test_a.rb\n\nruby/test_b.rb#test_c\n");
    assert_eq!(allowed.len(), 2);
    assert_eq!(allowed[0].reason, "Why.");
    assert!(allowed[1].reason.is_empty());
}

/// The suites must not write into the vendored tree: it is hashed by the manifest test.
#[test]
fn upstream_scratch_directory_is_outside_the_vendored_tree() {
    let scratch: &Path = &built().scratch;
    assert!(!scratch.starts_with(common::source_dir()));
}
