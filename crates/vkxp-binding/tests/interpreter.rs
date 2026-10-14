//! What one test file can assert of both interpreters: which Ruby the harness runs, and what
//! the preparation defines over either.

use vkxp_binding::testing;
use vkxp_binding::testing::host::{self, Script};
use vkxp_ruby_sys::{RUBY_API, RubyApi};

/// The value of a snippet evaluated after the preparation, or its exception as
/// `Class: message`.
fn prepared(source: &str) -> String {
    match host::play(source, Script::new()).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

#[test]
fn the_interpreter_is_the_one_its_crate_names() {
    let expected = match RUBY_API {
        RubyApi::Ruby18 => r#""1.8.7""#,
        RubyApi::Ruby19 => r#""1.9.2""#,
    };
    assert_eq!(testing::eval("RUBY_VERSION").as_deref(), Ok(expected));
}

#[test]
fn both_interpreters_get_reset_save_data_and_load_data() {
    assert_eq!(
        prepared("[Reset.superclass, defined?(save_data).nil?, defined?(load_data).nil?]"),
        "[Exception, false, false]"
    );
}
