//! `String#hash` and `Float#hash` are those of Ruby 1.8.1, which the original runtime embeds.
//!
//! The expected values were measured in the original runtime.

mod common;

use std::process::Command;
use std::sync::OnceLock;

use common::eval_inspect;
use vkxp_ruby_sys::{Interpreter, Options};

fn eval(source: &'static str) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |_| eval_inspect(source))
}

#[test]
fn string_hashes_match_the_original() {
    assert_eq!(
        eval(
            r#"["", "a", "ab", "abc", "foo", "Graphics/Characters/001-Fighter01",
                "\xe6\xb5\x8b\xe8\xaf\x95", "\xff", "\x80\x7f"].map { |s| s.hash }"#
        ),
        Ok(
            "[0, 100, 6562051, 833038373, 876516207, -1017680531, 97144166, -2, -8658938]"
                .to_owned()
        )
    );
}

#[test]
fn string_keys_iterate_in_the_order_of_the_original() {
    assert_eq!(
        eval(
            "h = {}; \
             %w(hp sp str dex agi int atk pdef mdef eva name id x y z).each { |k| h[k] = 1 }; \
             h.keys.join(' ')"
        ),
        Ok("\"name dex x mdef atk y pdef agi z int id eva str sp hp\"".to_owned())
    );
}

#[test]
fn string_hash_is_the_same_in_every_process() {
    let scratch = common::scratch_dir("hashes");
    let ruby = common::build_ruby_executable(&scratch);
    let hash = || common::run(Command::new(&ruby).args(["-e", "print 'abc'.hash"]));
    let first = hash();
    assert_eq!(first, "833038373");
    assert_eq!(hash(), first);
}

#[test]
fn float_hashes_match_the_original() {
    assert_eq!(
        eval("[1.5.hash, 0.0.hash, -0.0.hash]"),
        Ok("[53405, 0, 0]".to_owned())
    );
}
