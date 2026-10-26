//! The functions that start or signal processes, turned off as the binding prepares a guest
//! of RGSS3 by default: the scenarios "Functions of Ruby 1.9" and "Open with a file name" of
//! "Process functions of Ruby 1.9 are off by default", and those of "Process functions are
//! off by default" of `rgss-runtime` in a game of RGSS3, asked of the binding directly.
#![cfg(not(vkrgss_interpreter_absent))]

use vkrgss_binding_rgss3::testing::host::{self, Script, ruby_literal, with_error_stream};
use vkrgss_binding_rgss3::testing::{self, Harness};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS3.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss3)
}

/// A fresh directory for the files the commands would make.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = harness().game_dir().with_file_name(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_functions_of_ruby_1_9_start_nothing_and_say_so() {
    let dir = scratch("process_functions");
    let file = |name: &str| ruby_literal(&dir.join(name).to_string_lossy());
    let source = format!(
        "[spawn('touch ' + {a}), Process.spawn('touch ' + {b}), open('|touch ' + {c}),
          system('touch', {d}), `touch #{{{e}}}`, IO.popen(['touch', {f}])]",
        a = file("a"),
        b = file("b"),
        c = file("c"),
        d = file("d"),
        e = file("e"),
        f = file("f"),
    );
    let (value, _) = host::play(harness(), &with_error_stream(&source), Script::new());
    let value = value.expect("the snippet runs");
    assert!(
        value.starts_with("[[nil, nil, nil, nil, nil, nil], \""),
        "{value}"
    );
    // One line of the error stream for each call.
    assert_eq!(value.matches("\\n").count(), 6, "{value}");
    for (call, name) in [
        ("spawn", "a"),
        ("Process.spawn", "b"),
        ("open", "c"),
        ("system", "d"),
        ("`", "e"),
        ("IO.popen", "f"),
    ] {
        assert!(
            value.contains(&format!("vkrgss: {call} is turned off")),
            "{value}"
        );
        assert!(!dir.join(name).exists(), "{name} exists");
    }
}

#[test]
fn open_with_a_file_name_works() {
    let dir = scratch("open_file");
    let note = dir.join("note.txt");
    let source = format!(
        "open({}, 'w') {{ |file| file.write('x') }}",
        ruby_literal(&note.to_string_lossy())
    );
    let (value, _) = host::play(harness(), &source, Script::new());
    assert!(value.is_ok(), "{value:?}");
    assert_eq!(std::fs::read_to_string(note).unwrap(), "x");
}

/// Requirement "Process functions are off by default", scenario "Commands through open and
/// IO", in a game of RGSS3: the binding has no sandbox, as with `--sandbox off`. Beside the
/// scenario `IO.binread`, and a name that Ruby 1.9 takes with `to_path`.
#[test]
fn open_and_io_start_no_command() {
    let dir = scratch("pipe_commands");
    let file = |name: &str| ruby_literal(&dir.join(name).to_string_lossy());
    let source = format!(
        "named = Object.new
         def named.to_path
           '|touch ' + {g}
         end
         [open('|touch ' + {a}), IO.read('|touch ' + {b}), IO.readlines('|touch ' + {c}),
          IO.foreach('|touch ' + {d}) {{}}, File.read('|touch ' + {e}),
          IO.binread('|touch ' + {f}), open(named), File.readlines(named)]",
        a = file("a"),
        b = file("b"),
        c = file("c"),
        d = file("d"),
        e = file("e"),
        f = file("f"),
        g = file("g"),
    );
    let (value, _) = host::play(harness(), &with_error_stream(&source), Script::new());
    let value = value.expect("the snippet runs");
    assert_eq!(
        value,
        "[[nil, nil, nil, nil, nil, nil, nil, nil], \
         \"vkrgss: open is turned off, nothing was started\\n\
         vkrgss: IO.read is turned off, nothing was started\\n\
         vkrgss: IO.readlines is turned off, nothing was started\\n\
         vkrgss: IO.foreach is turned off, nothing was started\\n\
         vkrgss: IO.read is turned off, nothing was started\\n\
         vkrgss: IO.binread is turned off, nothing was started\\n\
         vkrgss: open is turned off, nothing was started\\n\
         vkrgss: IO.readlines is turned off, nothing was started\\n\"]"
    );
    for name in ["a", "b", "c", "d", "e", "f", "g"] {
        assert!(!dir.join(name).exists(), "{name} exists");
    }
}

/// Requirement "Process functions are off by default", scenario "Files through open and IO",
/// in a game of RGSS3.
#[test]
fn open_and_io_read_and_write_files() {
    let dir = scratch("pipe_files");
    let note = ruby_literal(&dir.join("note.txt").to_string_lossy());
    let source = format!(
        "open({note}, 'w') {{ |file| file.write(\"one\\ntwo\\n\") }}
         lines = []
         IO.foreach({note}) {{ |line| lines << line }}
         [IO.read({note}), IO.readlines({note}), File.read({note}), IO.binread({note}), lines]"
    );
    let (value, _) = host::play(harness(), &source, Script::new());
    assert_eq!(
        value.expect("the snippet runs"),
        r#"["one\ntwo\n", ["one\n", "two\n"], "one\ntwo\n", "one\ntwo\n", ["one\n", "two\n"]]"#
    );
}

/// Requirement "Process functions are off by default", scenario "Process.kill", in a game of
/// RGSS3.
#[test]
fn process_kill_signals_nothing() {
    let (value, _) = host::play(
        harness(),
        &with_error_stream("Process.kill(0, Process.ppid)"),
        Script::new(),
    );
    assert_eq!(
        value.expect("the snippet runs"),
        r#"[nil, "vkrgss: Process.kill is turned off, nothing was signalled\n"]"#
    );
}
