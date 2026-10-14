//! Without `VKRGSS_GC_STRESS`, and with it empty or 0, the collector runs when it has to.
//!
//! One test per file: the variable is read when the one interpreter of the process starts.

mod common;

use vkrgss_ruby_sys::{GC_STRESS_VARIABLE, Interpreter, Options, StartError};

#[test]
fn no_allocation_collects_without_the_variable() {
    // SAFETY: the only test of this process, and no other thread exists yet.
    unsafe { std::env::set_var(GC_STRESS_VARIABLE, "often") };
    let refused = Interpreter::start(Options::default());
    assert!(matches!(refused, Err(StartError::InvalidGcStress(value)) if value == "often"));

    // SAFETY: as above. A run under `cargo xtask stress` has the variable set.
    unsafe { std::env::remove_var(GC_STRESS_VARIABLE) };
    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let (grown, result) = interpreter.run(|_| common::collections_during(common::HUNDRED_OBJECTS));
    assert_eq!(result, Ok("100".to_owned()));
    // The heap of a fresh interpreter has room for far more than 100 objects.
    assert_eq!(grown, 0, "{grown} collection(s) for 100 objects");
}
