//! Ruby's own suites pass whatever signal dispositions the process that runs them has.
//!
//! A tool without a terminal starts its children with SIGINT ignored, and an ignored
//! disposition is inherited through every child process. `test_should_propagate_signaled`
//! starts a child interpreter that sends itself SIGINT and expects it to be interrupted.
//!
//! One test per file: the test changes a disposition of the whole process.

mod common;

use std::time::Duration;

use common::Suites;

/// How long one test process may run.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(120);

#[test]
fn upstream_suite_passes_with_sigint_ignored() {
    let scratch = common::scratch_dir("upstream_signals");
    let ruby = common::build_ruby_executable(&scratch);
    let suites = Suites {
        scratch,
        ruby,
        env: Vec::new(),
        timeout: PROCESS_TIMEOUT,
    };

    // SAFETY: `SIG_IGN` installs no handler, and this process has one test, so nothing else
    // depends on the disposition of SIGINT.
    let before = unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
    assert_ne!(before, libc::SIG_ERR, "SIGINT can be ignored");
    let result = suites.run_unit_file("ruby", "test_beginendblock.rb");
    // SAFETY: puts back the disposition that `signal` returned above.
    unsafe { libc::signal(libc::SIGINT, before) };

    let totals = result.unwrap_or_else(|problem| panic!("with SIGINT ignored:\n{problem}"));
    assert!(totals.tests > 0, "test_beginendblock.rb ran no test");
}
