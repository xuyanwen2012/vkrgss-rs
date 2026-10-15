//! What the prelude prepares before a game's scripts run: the scenarios of `rgss-runtime`
//! that need no game, evaluated as Ruby snippets against a scripted host.

use std::fs;

use vkrgss_binding::testing::host::{self, Script, with_error_stream};
use vkrgss_binding::testing::{self, Harness};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS1.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}

/// The value of a snippet, or its exception as `Class: message`.
fn eval(source: &str) -> String {
    match host::play(harness(), source, Script::new()).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

#[test]
fn globals_are_those_of_the_original_runtime() {
    vkrgss_binding::pending_over_ruby_1_9_2!();
    assert_eq!(
        eval("[$KCODE, $:, $DEBUG, $BTEST]"),
        r#"["UTF8", [], false, false]"#
    );
}

#[test]
fn reset_and_hangup_are_exceptions_outside_standard_error() {
    assert_eq!(
        eval("[Reset.ancestors.include?(Exception), Reset.ancestors.include?(StandardError)]"),
        "[true, false]"
    );
    assert_eq!(
        eval("[Hangup.ancestors.include?(Exception), Hangup.ancestors.include?(StandardError)]"),
        "[true, false]"
    );
    // The scenario "XP" of "Small differences of RGSS2".
    assert_eq!(eval("Object.const_defined?(:Hangup)"), "true");
}

#[test]
fn system_starts_nothing_and_says_so() {
    let marker = harness().game_dir().join("should_not_exist");
    let source = format!(
        "Dir.chdir('{}') {{ system('touch should_not_exist') }}",
        harness().game_dir().display()
    );
    assert_eq!(
        eval(&with_error_stream(&source)),
        r#"[nil, "vkrgss: system is turned off, nothing was started\n"]"#
    );
    assert!(!marker.exists());
}

#[test]
fn every_process_function_returns_nil() {
    assert_eq!(
        eval(&with_error_stream(
            "[`true`, %x{true}, exec('true'), fork, fork { 1 }, syscall(39), \
             IO.popen('true'), Kernel.system('true'), Process.fork]"
        )),
        "[[nil, nil, nil, nil, nil, nil, nil, nil, nil], \
         \"vkrgss: ` is turned off, nothing was started\\n\
         vkrgss: ` is turned off, nothing was started\\n\
         vkrgss: exec is turned off, nothing was started\\n\
         vkrgss: fork is turned off, nothing was started\\n\
         vkrgss: fork is turned off, nothing was started\\n\
         vkrgss: syscall is turned off, nothing was started\\n\
         vkrgss: IO.popen is turned off, nothing was started\\n\
         vkrgss: system is turned off, nothing was started\\n\
         vkrgss: Process.fork is turned off, nothing was started\\n\"]"
    );
}

#[test]
fn a_ruby_file_in_the_game_directory_is_loaded() {
    assert_eq!(eval("require 'Extra'; VKRGSS_EXTRA"), ":extra");
    // Required once, loaded as often as asked, whatever the letter case of the name.
    assert_eq!(
        eval(
            "before = $vkrgss_extra_loads
             again = require('Extra')
             load 'extra.rb'
             [again, $vkrgss_extra_loads - before]"
        ),
        "[false, 1]"
    );
}

#[test]
fn nothing_else_is_loaded() {
    let outside = harness().game_dir().with_file_name("other.rb");
    fs::write(&outside, "$vkrgss_outside = true\n").unwrap();
    fs::write(
        harness().game_dir().join("Plain.txt"),
        "$vkrgss_plain = true\n",
    )
    .unwrap();
    for source in [
        "require '/etc/passwd'",
        "load '../other.rb'",
        "require '../other'",
        "load '/etc/passwd'",
        "load 'Plain.txt'",
        "load 'Extra'",
        "require 'zlib.so'",
        "require 'Missing'",
        "Kernel.require '/etc/passwd'",
    ] {
        let outcome = eval(source);
        assert!(outcome.starts_with("LoadError: "), "{source}: {outcome}");
    }
    assert_eq!(eval("[$vkrgss_outside, $vkrgss_plain]"), "[nil, nil]");
}

#[test]
fn win32api_returns_zero_and_names_a_function_once() {
    assert_eq!(
        eval(&with_error_stream(
            "api = Win32API.new('user32', 'GetKeyNameTextA', 'i', 'i')
             [api.call(13), api.call(13), Win32API.new('user32', 'GetKeyNameTextA', 'i', 'i').Call(1)]"
        )),
        r#"[[0, 0, 0], "vkrgss: Win32API user32 GetKeyNameTextA is not emulated and returns 0\n"]"#
    );
}
