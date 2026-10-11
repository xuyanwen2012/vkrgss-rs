//! Ruby's own suites pass whatever search path the process that runs them has.
//!
//! Ruby 1.8.7 warns about a directory in `PATH` that everybody may write whenever it starts a
//! program, and `test_beginendblock.rb` compares the error stream of a child with an exact
//! text. The harness gives the suites a search path of its own.
//!
//! One test per file: the test changes the environment of the whole process.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use common::Suites;

/// How long one test process may run.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(120);

#[test]
fn upstream_suite_passes_with_a_world_writable_directory_in_the_path() {
    let scratch = common::scratch_dir("upstream_path");
    let ruby = common::build_ruby_executable(&scratch);
    let suites = Suites {
        scratch: scratch.clone(),
        ruby,
        env: Vec::new(),
        timeout: PROCESS_TIMEOUT,
    };

    let writable = scratch.join("writable");
    fs::create_dir(&writable).expect("scratch directory");
    fs::set_permissions(&writable, fs::Permissions::from_mode(0o777)).expect("mode 0777");
    let before = std::env::var_os("PATH").unwrap_or_default();
    let mut search = vec![writable];
    search.extend(std::env::split_paths(&before));
    let search = std::env::join_paths(search).expect("a joinable PATH");
    // SAFETY: this process has one test, so no other thread reads or writes the environment.
    unsafe { std::env::set_var("PATH", search) };
    let result = suites.run_unit_file("ruby", "test_beginendblock.rb");
    // SAFETY: as above; puts back the value read before.
    unsafe { std::env::set_var("PATH", before) };

    let totals = result
        .unwrap_or_else(|problem| panic!("with a world-writable directory in PATH:\n{problem}"));
    assert!(totals.tests > 0, "test_beginendblock.rb ran no test");
}
