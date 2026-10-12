//! With `VKXP_GC_STRESS=1` the collector runs at every allocation.
//!
//! One test per file: the variable is read when the one interpreter of the process starts.

mod common;

use vkxp_ruby_sys::{GC_STRESS_VARIABLE, Interpreter, Options};

#[test]
fn every_allocation_collects() {
    // SAFETY: the only test of this process, and no other thread exists yet.
    unsafe { std::env::set_var(GC_STRESS_VARIABLE, "1") };
    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let (grown, result) = interpreter.run(|_| common::collections_during(common::HUNDRED_OBJECTS));
    assert_eq!(result, Ok("100".to_owned()));
    assert!(grown >= 100, "{grown} collection(s) for 100 objects");
}
