//! A `class` statement that names an existing class with another superclass defines a new
//! class, as in Ruby 1.8.1, which the original runtime embeds.
//!
//! The first scenario was measured in the original runtime.

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby_sys::{Interpreter, Options};

fn eval(source: &'static str) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |_| eval_inspect(source))
}

#[test]
fn another_superclass_defines_a_new_class() {
    assert_eq!(
        eval(
            "class KA; def x; end; end; class KB; end; old = KA; \
             class KA < KB; def y; end; end; \
             [KA.equal?(old), KA.superclass, KA.new.respond_to?(:y), KA.new.respond_to?(:x), \
              old.name, KA.name, old.superclass, old.new.respond_to?(:x)]"
        ),
        Ok("[false, KB, true, false, \"KA\", \"KA\", Object, true]".to_owned())
    );
}

#[test]
fn an_explicit_object_defines_a_new_class() {
    assert_eq!(
        eval(
            "class KD; end; class KC < KD; end; old = KC; class KC < Object; end; \
             [KC.superclass, KC.equal?(old)]"
        ),
        Ok("[Object, false]".to_owned())
    );
}

#[test]
fn a_class_is_reopened_without_a_superclass_or_with_its_own() {
    assert_eq!(
        eval(
            "class KE; end; class KF < KE; end; old = KF; \
             class KF; def z; end; end; class KF < KE; def w; end; end; \
             [KF.equal?(old), KF.new.respond_to?(:z), KF.new.respond_to?(:w)]"
        ),
        Ok("[true, true, true]".to_owned())
    );
}

#[test]
fn a_new_class_in_a_module_takes_the_name_there() {
    assert_eq!(
        eval(
            "module KN; class A; end; class B; end; Old = A; class A < B; end; end; \
             [KN::A.equal?(KN::Old), KN::A.superclass, KN::A.name]"
        ),
        Ok("[false, KN::B, \"KN::A\"]".to_owned())
    );
}

#[test]
fn a_module_named_in_a_class_statement_raises() {
    assert_eq!(
        eval(
            "module KM; end; \
             begin; class KM; end; rescue TypeError => e; [e.class, e.message]; end"
        ),
        Ok("[TypeError, \"KM is not a class\"]".to_owned())
    );
}
