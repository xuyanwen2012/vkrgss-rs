//! What the prelude prepares before a game's scripts run: the scenarios of `rgss-runtime`
//! that need no game, evaluated as Ruby snippets against a scripted host.

use std::fs;

use vkxp_binding::testing;
use vkxp_binding::testing::host::{self, Script, with_error_stream};

/// The value of a snippet, or its exception as `Class: message`.
fn eval(source: &str) -> String {
    match host::play(source, Script::new()).0 {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

#[test]
fn globals_are_those_of_the_original_runtime() {
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
}

#[test]
fn system_starts_nothing_and_says_so() {
    let marker = testing::game_dir().join("should_not_exist");
    let source = format!(
        "Dir.chdir('{}') {{ system('touch should_not_exist') }}",
        testing::game_dir().display()
    );
    assert_eq!(
        eval(&with_error_stream(&source)),
        r#"[nil, "vkxp: system is turned off, nothing was started\n"]"#
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
         \"vkxp: ` is turned off, nothing was started\\n\
         vkxp: ` is turned off, nothing was started\\n\
         vkxp: exec is turned off, nothing was started\\n\
         vkxp: fork is turned off, nothing was started\\n\
         vkxp: fork is turned off, nothing was started\\n\
         vkxp: syscall is turned off, nothing was started\\n\
         vkxp: IO.popen is turned off, nothing was started\\n\
         vkxp: system is turned off, nothing was started\\n\
         vkxp: Process.fork is turned off, nothing was started\\n\"]"
    );
}

#[test]
fn a_ruby_file_in_the_game_directory_is_loaded() {
    assert_eq!(eval("require 'Extra'; VKXP_EXTRA"), ":extra");
    // Required once, loaded as often as asked, whatever the letter case of the name.
    assert_eq!(
        eval(
            "before = $vkxp_extra_loads
             again = require('Extra')
             load 'extra.rb'
             [again, $vkxp_extra_loads - before]"
        ),
        "[false, 1]"
    );
}

#[test]
fn nothing_else_is_loaded() {
    let outside = testing::game_dir().with_file_name("other.rb");
    fs::write(&outside, "$vkxp_outside = true\n").unwrap();
    fs::write(
        testing::game_dir().join("Plain.txt"),
        "$vkxp_plain = true\n",
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
    assert_eq!(eval("[$vkxp_outside, $vkxp_plain]"), "[nil, nil]");
}

#[test]
fn win32api_returns_zero_and_names_a_function_once() {
    assert_eq!(
        eval(&with_error_stream(
            "api = Win32API.new('user32', 'GetAsyncKeyState', 'i', 'i')
             [api.call(13), api.call(13), Win32API.new('user32', 'GetAsyncKeyState', 'i', 'i').Call(1)]"
        )),
        r#"[[0, 0, 0], "vkxp: Win32API user32 GetAsyncKeyState is not emulated and returns 0\n"]"#
    );
}
