//! A copy of a process whose script had made a thread: the copy forgets the timer thread it
//! does not have, and a thread a script creates there works, as it does in the original.
//!
//! One test per file: the process forks, and has one interpreter.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::time::Duration;

use common::copy;

#[test]
fn a_thread_made_after_a_copy_runs_in_the_copy_and_in_the_original() {
    let outcome = copy::run_around_a_copy(
        "Thread.new { 1 + 1 }.value",
        "t = Thread.new { sleep 0.05; :done }; t.join; t.value",
        Duration::from_secs(5),
    );
    assert_eq!(outcome.copy, ":done", "the copy");
    assert!(outcome.clean, "the copy did not exit with 0");
    assert_eq!(outcome.original, Ok(":done".to_owned()), "the original");
}
