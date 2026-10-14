//! A main thread that never blocks gives the interpreter's lock to a thread that waits for
//! it at the end of its time slice (patch `gvl-handover`). The lock of the release is a
//! plain mutex: the main thread took it again at once, and of twenty new threads measured
//! without the patch seven had not run after 30 seconds and the others ran after 0.1 to 28
//! seconds; with it each runs after one time slice of 10 ms. Twenty new threads in a row
//! each run here while the main thread waits for them without blocking.
//!
//! One test per file: the timer thread of the interpreter belongs to the process.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

#[test]
fn a_busy_main_thread_gives_way_to_every_new_thread() {
    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let value = interpreter.run(|ruby| {
        eval_inspect(
            ruby,
            "(1..20).map { $ran = false; Thread.new { $ran = true }; \
             limit = Time.now + 30; nil until $ran || Time.now > limit; $ran }.count(true)",
        )
    });
    assert_eq!(value, Ok("20".to_owned()));
}
