//! What the binding prepares over Ruby 1.9.2 before a game's scripts run: neither `$KCODE`
//! nor an emptied load path, and nothing of the prelude of the Ruby release.
#![cfg(not(vkrgss_interpreter_absent))]

use vkrgss_binding_rgss3::testing::host::{self, Script};
use vkrgss_binding_rgss3::testing::{self, Harness};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS3.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss3)
}

/// The value of a snippet evaluated after the preparation of a game of RGSS3, or its
/// exception as `Class: message`.
fn prepared(source: &str) -> String {
    match host::play(harness(), source, Script::new()).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

#[test]
fn the_globals_are_those_of_the_interpreter() {
    assert_eq!(prepared("[$KCODE, $:]"), r#"[nil, ["."]]"#);
}

#[test]
fn nothing_of_the_prelude_of_the_release_is_defined() {
    assert_eq!(
        prepared("[Mutex.new.respond_to?(:synchronize), Thread.respond_to?(:exclusive)]"),
        "[false, false]"
    );
}

/// The helpers of the binding's own prelude are private methods of `Object`, as over Ruby
/// 1.8.7.
#[test]
fn the_helpers_of_the_prelude_are_private() {
    assert_eq!(
        prepared(
            "[Object.private_method_defined?(:vkrgss_load_scripts),
              Object.public_method_defined?(:vkrgss_load_scripts)]"
        ),
        "[true, false]"
    );
}
