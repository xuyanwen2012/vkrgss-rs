//! What the prelude prepares before a game's scripts run: the scenarios of `rgss-runtime`
//! that need no game, evaluated as Ruby snippets against a scripted host.

use std::fs;

use vkrgss_binding::testing::host::{self, Script, ruby_literal, with_error_stream};
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

/// A fresh directory beside the game directory, for the files a command would make.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = harness().game_dir().with_file_name(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Requirement "Process functions are off by default", scenario "Commands through open and
/// IO": the binding has no sandbox, as with `--sandbox off`.
#[test]
fn open_and_io_start_no_command() {
    let dir = scratch("pipe_commands");
    let file = |name: &str| ruby_literal(&dir.join(name).to_string_lossy());
    let source = format!(
        "[open('|touch ' + {a}), IO.read('|touch ' + {b}), IO.readlines('|touch ' + {c}),
          IO.foreach('|touch ' + {d}) {{}}, File.read('|touch ' + {e}),
          Kernel.open('|touch ' + {f})]",
        a = file("a"),
        b = file("b"),
        c = file("c"),
        d = file("d"),
        e = file("e"),
        f = file("f"),
    );
    // One line of the error stream for each call.
    assert_eq!(
        eval(&with_error_stream(&source)),
        "[[nil, nil, nil, nil, nil, nil], \
         \"vkrgss: open is turned off, nothing was started\\n\
         vkrgss: IO.read is turned off, nothing was started\\n\
         vkrgss: IO.readlines is turned off, nothing was started\\n\
         vkrgss: IO.foreach is turned off, nothing was started\\n\
         vkrgss: IO.read is turned off, nothing was started\\n\
         vkrgss: open is turned off, nothing was started\\n\"]"
    );
    for name in ["a", "b", "c", "d", "e", "f"] {
        assert!(!dir.join(name).exists(), "{name} exists");
    }
}

/// Requirement "Process functions are off by default", scenario "Files through open and IO".
#[test]
fn open_and_io_read_and_write_files() {
    let dir = scratch("pipe_files");
    let note = ruby_literal(&dir.join("note.txt").to_string_lossy());
    assert_eq!(
        eval(&format!(
            "open({note}, 'w') {{ |file| file.write(\"one\\ntwo\\n\") }}
             lines = []
             IO.foreach({note}) {{ |line| lines << line }}
             [IO.read({note}), IO.readlines({note}), File.read({note}), lines]"
        )),
        r#"["one\ntwo\n", ["one\n", "two\n"], "one\ntwo\n", ["one\n", "two\n"]]"#
    );
}

/// Requirement "Process functions are off by default", scenario "Process.kill".
#[test]
fn process_kill_signals_nothing() {
    assert_eq!(
        eval(&with_error_stream(
            "[Process.kill(0, Process.ppid), Object.new.extend(Process).send(:kill, 0, Process.ppid)]"
        )),
        "[[nil, nil], \
         \"vkrgss: Process.kill is turned off, nothing was signalled\\n\
         vkrgss: Process.kill is turned off, nothing was signalled\\n\"]"
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

/// Writes `contents` to `name` in the game directory.
fn put(name: &str, contents: &str) {
    fs::write(harness().game_dir().join(name), contents).unwrap();
}

/// Requirement "Loading code", scenario "autoload over Ruby 1.8": `autoload` follows the rule
/// of `require` at the call, and the file loads at the first use of the constant.
#[test]
fn autoload_loads_only_what_require_may() {
    put(
        "Lazy.rb",
        "$vkrgss_lazy = ($vkrgss_lazy || 0) + 1\nclass Lazy\nend\n",
    );
    assert_eq!(
        eval(
            "far = begin
               autoload(:Far, '/etc/passwd')
             rescue LoadError => error
               error.class
             end
             autoload(:Lazy, 'Lazy')
             before = $vkrgss_lazy.to_i
             [far, Object.autoload?(:Far), before, Lazy.class, $vkrgss_lazy]"
        ),
        "[LoadError, nil, 0, Class, 1]"
    );
}

/// Requirement "Loading code", scenario "A file that raised is required again".
#[test]
fn a_file_that_raised_is_required_again() {
    put(
        "Flaky.rb",
        "$vkrgss_flaky = ($vkrgss_flaky || 0) + 1\n\
         raise 'the first time' if $vkrgss_flaky == 1\n\
         VKRGSS_FLAKY = $vkrgss_flaky\n",
    );
    assert_eq!(
        eval(
            "first = (require 'Flaky' rescue nil)
             [first, require('Flaky'), VKRGSS_FLAKY, require('Flaky')]"
        ),
        "[nil, true, 2, false]"
    );
}

/// Requirement "Loading code", scenario "A file that requires itself".
#[test]
fn a_file_that_requires_itself_is_evaluated_once() {
    put(
        "Self.rb",
        "$vkrgss_self = ($vkrgss_self || 0) + 1\n$vkrgss_inner = require 'Self'\n",
    );
    assert_eq!(
        eval("[require('Self'), $vkrgss_self, $vkrgss_inner]"),
        "[true, 1, false]"
    );
}

/// Requirement "Loading code", scenario "Wrapped load".
#[test]
fn a_wrapped_load_keeps_its_constants() {
    put("Part.rb", "PART = :part\n$vkrgss_part = Module.nesting\n");
    assert_eq!(
        eval(
            "[load('Part.rb', true), defined?(PART), $vkrgss_part.size,
              $vkrgss_part[0].const_get(:PART)]"
        ),
        "[true, nil, 1, :part]"
    );
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
