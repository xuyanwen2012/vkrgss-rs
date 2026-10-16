//! An interpreter started with the options of RGSS3 has no script name, as the original
//! runtime's: `$0` is `false` and the frame of the `eval` at the bottom of a backtrace is named
//! `ruby` (`ace-v1-api-REPORT.md` section 1.1 and `ace-v0-REPORT.md` section 8 of the
//! measurements of the original runtime). The default options keep `vkrgss`, which
//! `interpreter.rs` holds.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

#[test]
fn the_options_of_rgss3_have_no_script_name() {
    assert_eq!(Options::for_rgss(3).script_name, "");
    assert_eq!(Options::for_rgss(1).script_name, "vkrgss");
    assert_eq!(Options::for_rgss(2).script_name, "vkrgss");
    assert_eq!(Options::default().script_name, "vkrgss");
}

#[test]
fn an_interpreter_without_a_script_name_is_that_of_the_original() {
    let interpreter = Interpreter::start(Options::for_rgss(3)).expect("the interpreter starts");
    let globals = interpreter.run(|ruby| eval_inspect(ruby, "[$0, $PROGRAM_NAME]"));
    assert_eq!(globals, Ok("[false, false]".to_owned()));
    let last = interpreter.run(|ruby| eval_inspect(ruby, "caller(0).last"));
    let last = last.expect("caller(0) has entries");
    assert!(
        last.starts_with("\"ruby:in `eval'"),
        "the last entry of the backtrace is {last}"
    );
}
