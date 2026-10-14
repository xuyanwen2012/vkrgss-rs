//! What a game of RGSS3 loads and registers beside its scripts: `at_exit`, and the scenarios
//! of "at_exit and exit! in RGSS3" of `rgss-runtime` that need no process of their own
//! (`exit!` ends the process, and is tested through the program), measured on the original
//! runtime of RGSS3 (`ace-v1-api-REPORT.md` section 1.6 of the measurements kept outside the
//! repository).
#![cfg(not(vkxp_interpreter_absent))]

use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Script};
use vkxp_vfs::Runtime;

/// `inspect` of the value of `source`, evaluated on the guest of a game of RGSS3 with its
/// prelude, or the class and message of what it raised.
fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    match host::play(source, Script::new()).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

// --- at_exit ----------------------------------------------------------------------------------

/// Requirement "at_exit and exit! in RGSS3", scenario "Value of at_exit".
#[test]
fn at_exit_returns_its_block() {
    assert_eq!(
        eval("[at_exit { 1 }.class, Kernel.at_exit { 2 }.class]"),
        "[Proc, Proc]"
    );
    assert_eq!(
        eval("b = proc { 3 }; [at_exit(&b).equal?(b), method(:at_exit).arity]"),
        "[true, 0]"
    );
    assert_eq!(eval("at_exit"), "ArgumentError: called without a block");
}
