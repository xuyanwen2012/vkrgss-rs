//! Ruby's own test suites, run against the interpreter exactly as it was built for vkrgss.
//!
//! A wrong compiler flag does not fail the build: without `-fwrapv`, for one, the interpreter
//! compiles and then computes `2**64` as 0. Only these suites show it, so they are neither
//! optional nor filtered. A real `ruby` executable is linked from the archives of the build
//! script, because the suites start child interpreters and expect a normal process.

mod common;

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use common::Suites;

/// What `ruby -v` must print, which names the platform of the target.
#[cfg(target_arch = "x86_64")]
const VERSION_LINE: &str = "ruby 1.8.7 (2013-06-27 patchlevel 374) [x86_64-linux]";
#[cfg(target_arch = "aarch64")]
const VERSION_LINE: &str = "ruby 1.8.7 (2013-06-27 patchlevel 374) [aarch64-linux]";

/// How long one test process may run. The slowest file takes a few seconds.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(120);

fn built() -> &'static Suites {
    static BUILT: OnceLock<Suites> = OnceLock::new();
    BUILT.get_or_init(|| {
        let scratch = common::scratch_dir("upstream");
        let ruby = common::build_ruby_executable(&scratch);
        Suites {
            scratch,
            ruby,
            env: Vec::new(),
            timeout: PROCESS_TIMEOUT,
        }
    })
}

#[test]
fn upstream_executable_reports_its_version() {
    let output = common::run(Command::new(&built().ruby).arg("-v"));
    assert_eq!(output.trim_end(), VERSION_LINE);
}

#[test]
fn upstream_core_suite_passes() {
    let totals = built().run_unit_suite("ruby");
    eprintln!("test/ruby: {totals:?}");
    assert_eq!(
        (totals.tests, totals.failures, totals.errors),
        (common::CORE_TESTS, 0, 0),
        "test/ruby reported {totals:?}"
    );
}

#[test]
fn upstream_zlib_suite_passes() {
    let totals = built().run_unit_suite("zlib");
    eprintln!("test/zlib: {totals:?}");
    assert_eq!(
        (totals.tests, totals.failures, totals.errors),
        (common::ZLIB_TESTS, 0, 0),
        "test/zlib reported {totals:?}"
    );
}

#[test]
fn upstream_sample_test_passes() {
    built().run_sample();
}

/// The suites must not write into the vendored tree: it is hashed by the manifest test.
#[test]
fn upstream_scratch_directory_is_outside_the_vendored_tree() {
    let scratch: &Path = &built().scratch;
    assert!(!scratch.starts_with(common::source_dir()));
}
