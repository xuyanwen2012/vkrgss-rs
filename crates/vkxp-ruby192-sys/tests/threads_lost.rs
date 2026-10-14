//! `threads_lost_in_a_copy()` is true exactly while a Ruby thread other than the main one
//! is alive.
//!
//! One test per file: the process has one interpreter.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use common::eval_inspect;
use vkxp_ruby192_sys::ffi::{self, VALUE};
use vkxp_ruby192_sys::{Arity, HostResult, Interpreter, Options, Ruby};

/// `lost`, the answer of the crate as a Ruby boolean.
fn lost(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(if vkxp_ruby192_sys::threads_lost_in_a_copy() {
        ffi::Qtrue
    } else {
        ffi::Qfalse
    })
}

#[test]
fn threads_are_lost_exactly_while_a_script_thread_is_alive() {
    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let value = interpreter.run(|ruby| {
        ruby.define_global_function_with_arity(c"lost", Arity::Fixed(0), lost)
            .expect("the host function is defined");
        eval_inspect(
            ruby,
            "a = lost; t = Thread.new { sleep }; b = lost; t.kill; t.join; [a, b, lost]",
        )
    });
    assert_eq!(value, Ok("[false, true, false]".to_owned()));
}
