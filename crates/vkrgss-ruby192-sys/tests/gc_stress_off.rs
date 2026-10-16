//! Without `VKRGSS_GC_STRESS`, and with it empty or 0, the collector runs when it has to.
//!
//! One test per file: the variable is read when the one interpreter of the process starts.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use vkrgss_ruby192_sys::{GC_STRESS_VARIABLE, Interpreter, Options, StartError};

#[test]
fn no_allocation_collects_without_the_variable() {
    // SAFETY: the only test of this process, and no other thread exists yet.
    unsafe { std::env::set_var(GC_STRESS_VARIABLE, "often") };
    let refused = Interpreter::start(Options::default());
    assert!(matches!(refused, Err(StartError::InvalidGcStress(value)) if value == "often"));

    // SAFETY: as above. A run under `cargo xtask stress` has the variable set.
    unsafe { std::env::remove_var(GC_STRESS_VARIABLE) };
    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let stress = interpreter.run(|ruby| common::eval_inspect(ruby, "GC.stress"));
    assert_eq!(stress, Ok("false".to_owned()));
    let counted = interpreter
        .run(|ruby| common::eval_inspect(ruby, common::COLLECTIONS_FOR_HUNDRED_OBJECTS))
        .expect("the script runs");
    let (made, grown) = common::parse_pair(&counted);
    assert_eq!(made, 100);
    assert!(grown < 100, "{grown} collection(s) for 100 objects");
}
