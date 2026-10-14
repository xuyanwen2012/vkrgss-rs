//! With `VKRGSS_GC_STRESS=1` the collector runs at every allocation.
//!
//! One test per file: the variable is read when the one interpreter of the process starts.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use vkrgss_ruby192_sys::{GC_STRESS_VARIABLE, Interpreter, Options};

#[test]
fn every_allocation_collects() {
    // SAFETY: the only test of this process, and no other thread exists yet.
    unsafe { std::env::set_var(GC_STRESS_VARIABLE, "1") };
    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let stress = interpreter.run(|ruby| common::eval_inspect(ruby, "GC.stress"));
    assert_eq!(stress, Ok("true".to_owned()));
    let counted = interpreter
        .run(|ruby| common::eval_inspect(ruby, common::COLLECTIONS_FOR_HUNDRED_OBJECTS))
        .expect("the script runs");
    let (made, grown) = common::parse_pair(&counted);
    assert_eq!(made, 100);
    assert!(grown >= 100, "{grown} collection(s) for 100 objects");
}
