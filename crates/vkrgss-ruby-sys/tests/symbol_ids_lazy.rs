//! Names the interpreter interns as Ruby 1.8.1 does, measured on the original runtime of VX
//! (2026-10-12, `RGSS202E.dll` 2.0.2.1): `recv` at the first name error of method dispatch,
//! and `__member__`, a name of the start-up table, for the members of a `Struct` class.
//!
//! A process has one interpreter and one mark, which is why this is a binary of its own.

mod common;

use common::eval_inspect;
use vkrgss_ruby_sys::{Interpreter, Options, SymbolTable};

/// How far the sequence moves over each step: the whole script is parsed before it runs, so
/// its identifiers have their numbers by then, and the names noted are made with
/// `String#to_sym`.
const SCRIPT: &str = r#"
zz_steps = [
  proc { begin; nil.undefined_method_of_the_test; rescue NoMethodError; end },
  proc { begin; nil.undefined_method_of_the_test; rescue NoMethodError; end },
  proc { Struct.new(:left, :top) },
  proc { Struct.new(:left, :top) },
]
zz_index = 0
zz_steps.map do |zz_step|
  zz_index += 1
  zz_before = ("zz_before" + zz_index.to_s).to_sym.to_i
  zz_step.call
  (("zz_after" + zz_index.to_s).to_sym.to_i - zz_before) >> 3
end
"#;

#[test]
fn a_name_error_interns_recv_once_and_a_struct_interns_nothing() {
    let options = Options {
        symbols: SymbolTable::Rgss2,
        ..Options::default()
    };
    let interpreter = Interpreter::start(options).expect("the interpreter starts");
    let steps = interpreter.run(|ruby| {
        ruby.mark_game_scripts();
        eval_inspect(SCRIPT)
    });
    assert_eq!(steps, Ok("[2, 1, 1, 1]".to_owned()));
}
