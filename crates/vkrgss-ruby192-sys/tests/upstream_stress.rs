//! The selection of `test/ruby` again, with `GC.stress = true`: the collector runs at every
//! allocation, so a value that C code keeps alive only through a pointer into it is lost at
//! once instead of on some other machine. Two such faults of the release were found this way
//! and are patched (see `PATCHES.md` of the vendored tree).
//!
//! The files whose stressed run took under a minute in the feasibility study of 2026-10-13
//! run with the workspace tests. The slow ones are an ignored test, which is the step
//! `suites-1.9.2` of `cargo xtask stress`:
//!
//! ```text
//! cargo test -p vkrgss-ruby192-sys --test upstream_stress -- --ignored
//! ```
//!
//! What neither run includes is named in `xtask/src/stress.skip` under that step, with its
//! reason, beside what `upstream.allow` leaves out of every run.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::time::Duration;

use common::suites::{self, Suites};

/// The files of `test/ruby` whose stressed run took a minute or more in the study, with the
/// seconds it took there, ten files at a time. `test_io_m17n.rb` (61 s) is not selected.
const SLOW_UNDER_STRESS: &[(&str, u32)] = &[
    ("ruby/test_array.rb", 170),
    ("ruby/test_assignment.rb", 733),
    ("ruby/test_fiber.rb", 440),
    ("ruby/test_literal.rb", 652),
    ("ruby/test_process.rb", 73),
    ("ruby/test_sprintf_comb.rb", 72),
    ("ruby/test_system.rb", 821),
];

/// The list of `cargo xtask stress`, of which the lines of the step below belong here.
const STRESS_SKIP_LIST: &str = include_str!("../../../xtask/src/stress.skip");

/// The step of `cargo xtask stress` that runs the slow files.
const STRESS_STEP: &str = "suites-1.9.2 ";

/// How long one quick file may run. The slowest took 32 s in the study.
const QUICK_TIMEOUT: Duration = Duration::from_secs(600);

/// How long one slow file may run.
const SLOW_TIMEOUT: Duration = Duration::from_secs(3600);

/// What no stressed run includes: the entries of `xtask/src/stress.skip` for the step, as
/// `ruby/test_x.rb` or `ruby/test_x.rb#test_y`.
fn left_out_under_stress() -> Vec<String> {
    STRESS_SKIP_LIST
        .lines()
        .filter_map(|line| line.trim().strip_prefix(STRESS_STEP))
        .map(|test| test.trim().to_owned())
        .collect()
}

/// What a stressed run leaves out: the list of every run and that of the stressed runs.
fn skip() -> Vec<String> {
    let mut skip = suites::allowed_tests();
    skip.extend(left_out_under_stress());
    skip
}

fn is_slow(file: &str) -> bool {
    SLOW_UNDER_STRESS.iter().any(|(slow, _)| *slow == file)
}

#[test]
fn quick_files_pass_under_collector_stress() {
    let skip = skip();
    let files: Vec<String> = suites::selected_core_files(&skip)
        .into_iter()
        .filter(|file| !is_slow(file))
        .collect();
    let suites = Suites::build("upstream_stress", QUICK_TIMEOUT, true);
    let totals = suites.run_unit_files(&files, &skip);
    eprintln!(
        "test/ruby under GC.stress: {} files, {totals:?}",
        files.len()
    );
}

#[test]
#[ignore = "slow: run by `cargo xtask stress` as its step `suites-1.9.2`"]
fn slow_files_pass_under_collector_stress() {
    let skip = skip();
    let files: Vec<String> = suites::selected_core_files(&skip)
        .into_iter()
        .filter(|file| is_slow(file))
        .collect();
    let suites = Suites::build("upstream_stress_slow", SLOW_TIMEOUT, true);
    let totals = suites.run_unit_files(&files, &skip);
    eprintln!(
        "test/ruby under GC.stress: {} files, {totals:?}",
        files.len()
    );
}

/// The lists name files that exist, and a slow file is not also left out whole.
#[test]
fn the_lists_of_the_stressed_runs_name_vendored_files() {
    let left_out = left_out_under_stress();
    for name in ["test_integer_comb.rb", "test_m17n_comb.rb", "test_yield.rb"] {
        assert!(
            left_out.contains(&format!("ruby/{name}")),
            "xtask/src/stress.skip does not name {name} for the step"
        );
    }
    let files = suites::test_files("test/ruby");
    for (slow, _) in SLOW_UNDER_STRESS {
        let name = slow.strip_prefix("ruby/").expect("a file of test/ruby");
        assert!(files.iter().any(|file| file == name), "{slow}");
        assert!(!left_out.contains(&(*slow).to_owned()), "{slow}");
    }
    for entry in &left_out {
        let file = entry.split('#').next().expect("a file");
        let name = file.strip_prefix("ruby/").expect("a file of test/ruby");
        assert!(files.iter().any(|file| file == name), "{entry}");
    }
}
