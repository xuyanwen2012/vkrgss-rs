//! What differs between the platforms the interpreter is built for: `String#crypt`, which
//! bionic lacks. The platform string is tested with the identity of the original
//! (`tests/original.rs`).
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

fn eval(source: &str) -> Result<String, String> {
    static SHARED: OnceLock<Interpreter> = OnceLock::new();
    let source = source.to_owned();
    SHARED
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |ruby| eval_inspect(ruby, &source))
}

/// bionic has no `crypt(3)`.
#[cfg(target_os = "android")]
#[test]
fn crypt_raises_not_implemented_error() {
    assert_eq!(
        eval("begin; 'x'.crypt('ab'); rescue NotImplementedError => e; e.message; end"),
        Ok("\"crypt() is not available on Android\"".to_owned())
    );
}

/// glibc's `crypt(3)`, with a salt of two characters, is DES.
#[cfg(not(target_os = "android"))]
#[test]
fn crypt_is_that_of_the_c_library() {
    assert_eq!(eval("'x'.crypt('ab')"), Ok("\"abiQ6Ep3EYTHc\"".to_owned()));
}
