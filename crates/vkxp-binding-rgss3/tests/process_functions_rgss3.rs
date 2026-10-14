//! The functions of Ruby 1.9 that start processes, turned off as the binding prepares a
//! guest by default: the scenarios "Functions of Ruby 1.9" and "Open with a file name" of
//! "Process functions of Ruby 1.9 are off by default" of `rgss-runtime`, asked of the
//! binding directly.
#![cfg(not(vkxp_interpreter_absent))]

use vkxp_binding_rgss3::testing;
use vkxp_binding_rgss3::testing::host::{self, Script, ruby_literal, with_error_stream};
use vkxp_vfs::Runtime;

/// A fresh directory for the files the commands would make.
fn scratch(name: &str) -> std::path::PathBuf {
    testing::start_as(Runtime::Rgss3);
    let dir = testing::game_dir().with_file_name(name);
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
    let (value, _) = host::play(&with_error_stream(&source), Script::new());
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
            value.contains(&format!("vkxp: {call} is turned off")),
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
    let (value, _) = host::play(&source, Script::new());
    assert!(value.is_ok(), "{value:?}");
    assert_eq!(std::fs::read_to_string(note).unwrap(), "x");
}
