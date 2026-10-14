//! Ruby's own suites pass whatever signal dispositions the process that runs them has.
//!
//! A tool without a terminal starts its children with SIGINT ignored, and a shell does the
//! same with SIGINT and SIGQUIT for a command it starts in the background. An ignored
//! disposition is inherited through every child process. `test_should_propagate_signaled`
//! starts a child interpreter that is to be ended by SIGINT, and `test_status_kill` one that
//! is to be ended by SIGQUIT.
//!
//! One test per file: the test changes a disposition of the whole process.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::time::Duration;

use common::suites::{self, Suites};

/// How long one test process may run.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(300);

#[test]
fn upstream_suite_passes_with_sigint_and_sigquit_ignored() {
    let suites = Suites::build("upstream_signals", PROCESS_TIMEOUT, false);
    let skip = suites::allowed_tests();

    // SAFETY: `SIG_IGN` installs no handler, and this process has one test, so nothing else
    // depends on the dispositions of SIGINT and SIGQUIT.
    let before = unsafe {
        [
            libc::signal(libc::SIGINT, libc::SIG_IGN),
            libc::signal(libc::SIGQUIT, libc::SIG_IGN),
        ]
    };
    assert!(
        !before.contains(&libc::SIG_ERR),
        "SIGINT and SIGQUIT can be ignored"
    );
    let results = [
        suites.run_unit_file("ruby/test_beginendblock.rb", &skip),
        suites.run_unit_file("ruby/test_process.rb", &skip),
    ];
    // SAFETY: puts back the dispositions that `signal` returned above.
    unsafe {
        libc::signal(libc::SIGINT, before[0]);
        libc::signal(libc::SIGQUIT, before[1]);
    }

    for result in results {
        let totals =
            result.unwrap_or_else(|problem| panic!("with SIGINT and SIGQUIT ignored:\n{problem}"));
        assert!(totals.tests > 0);
    }
}
