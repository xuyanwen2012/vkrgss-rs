//! A copy of a process whose script had made a thread has a timer thread again once a
//! script creates a thread there: a main thread that never sleeps still lets the other run.
//!
//! One test per file: the process forks, and has one interpreter.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::time::Duration;

use common::copy;

#[test]
fn a_busy_main_thread_of_a_copy_gives_way_to_a_new_thread() {
    let outcome = copy::run_around_a_copy(
        "Thread.new { 1 + 1 }.value",
        "$ran = false; Thread.new { $ran = true }; limit = Time.now + 30; \
         nil until $ran || Time.now > limit; $ran",
        Duration::from_secs(40),
    );
    assert_eq!(outcome.copy, "true", "the copy");
    assert!(outcome.clean, "the copy did not exit with 0");
    assert_eq!(outcome.original, Ok("true".to_owned()), "the original");
}
