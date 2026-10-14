//! The security fixes backported to the vendored tree (`docs/vendored-security.md`): what a
//! few lines can show of each defect, and that the results beside it are unchanged.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

fn eval(source: &'static str) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |ruby| eval_inspect(ruby, source))
}

/// CVE-2013-4164: a digit string of this length overflowed the heap in `ruby_strtod`. The
/// same conversion reads a dumped Float in `Marshal.load`.
#[test]
fn a_very_long_digit_string_converts_to_a_float() {
    assert_eq!(
        eval("(('0.' + '1' * 300_000).to_f * 9 - 1.0).abs < 1e-12"),
        Ok("true".to_owned())
    );
    assert_eq!(
        eval(
            "digits = '1.' + '5' * 300_000
             Marshal.load(\"\\x04\\x08f\\x03\" + [digits.size].pack('V')[0, 3] + digits).floor"
        ),
        Ok("1".to_owned())
    );
    // Short strings convert as before.
    assert_eq!(
        eval("['0.1'.to_f, '1e3'.to_f, '123456789.125'.to_f, '-2.5e-3'.to_f, '1_0.5'.to_f]"),
        Ok("[0.1, 1000.0, 123456789.125, -0.0025, 10.5]".to_owned())
    );
}
