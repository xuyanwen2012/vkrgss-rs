//! `String#hash` and the other hashes are the same in every process, and a hash keeps its
//! keys in insertion order.
//!
//! The test starts itself a second time as a child process and compares the values.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::process::Command;

use common::eval_inspect;
use vkrgss_ruby192_sys::{HASH_SEED, Interpreter, Options};

/// Set for the child process, which prints its values and does nothing else.
const CHILD_VARIABLE: &str = "VKRGSS_RUBY192_HASH_SEED_CHILD";

/// The line the child prints its values on.
const MARKER: &str = "hashes of the child: ";

const HASHES: &str = "['abc'.hash, 1.5.hash, [1, 2].hash, :a.hash]";

#[test]
fn hashes_are_the_same_in_two_processes() {
    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let hashes = interpreter
        .run(|ruby| eval_inspect(ruby, HASHES))
        .expect("the hashes are computed");
    if std::env::var_os(CHILD_VARIABLE).is_some() {
        println!("{MARKER}{hashes}");
        return;
    }
    assert_eq!(HASH_SEED, 1, "the seed the feasibility study ran with");

    let output = Command::new(std::env::current_exe().expect("the path of this test"))
        .args([
            "--exact",
            "hashes_are_the_same_in_two_processes",
            "--nocapture",
        ])
        .env(CHILD_VARIABLE, "1")
        .output()
        .expect("the test starts itself again");
    let text = String::from_utf8_lossy(&output.stdout);
    let of_child = text
        .lines()
        .find_map(|line| line.strip_prefix(MARKER))
        .unwrap_or_else(|| panic!("the child printed no values:\n{text}"));
    assert!(output.status.success(), "the child failed:\n{text}");
    assert_eq!(of_child, hashes, "the two processes disagree");
    assert_eq!(hashes.matches(',').count(), 3, "four numbers: {hashes}");

    // Measured on the original runtime: a hash keeps the order of insertion, whatever the
    // hash values of its keys are.
    let keys = interpreter.run(|ruby| eval_inspect(ruby, "{:b => 1, :a => 2, 'c' => 3}.keys"));
    assert_eq!(keys, Ok("[:b, :a, \"c\"]".to_owned()));
}
