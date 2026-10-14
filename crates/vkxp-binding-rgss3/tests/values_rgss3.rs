//! The value classes `Color`, `Tone`, `Rect` and `Table` in a game of RGSS3: the scenarios
//! of `rgss-values` that name RGSS3. A file of this package alone, because the classes are
//! defined once per process, as the runtime of the run has them, and RGSS3 runs over Ruby
//! 1.9.2 only.
//!
//! The values are those measured on the original runtime of RGSS3 on 2026-10-13
//! (`ace-v1-bitmap-REPORT.md` section 3, `ace-v1-api-REPORT.md` sections 0, 2.4 and 3.3, kept
//! outside the repository); a test says where vkxp does something else.
#![cfg(not(vkxp_interpreter_absent))]
// The support module of `vkxp-binding` names the binding by that name.
extern crate vkxp_binding_rgss3 as vkxp_binding;

#[path = "../../vkxp-binding/tests/scene_support/mod.rs"]
mod scene_support;

use vkxp_binding::testing;
use vkxp_vfs::Runtime;

/// `inspect` of the value of `source` in a game of RGSS3, or the class and the message of
/// the exception that left it.
fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    scene_support::install();
    match testing::eval(source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// The class of the exception `source` raises.
fn raised(source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    match testing::eval(source) {
        Ok(inspect) => panic!("`{source}` raised nothing and gave {inspect}"),
        Err(error) => error.class_name,
    }
}

const ZEROS: &str = "(0.000000, 0.000000, 0.000000, 0.000000)";

// --- Color and Tone of RGSS3 ------------------------------------------------------------------

/// Scenario "Without arguments, measured on the original".
#[test]
fn a_colour_and_a_tone_without_arguments_are_zeros() {
    assert_eq!(
        eval("[Color.new.to_s, Tone.new.to_s]"),
        format!(r#"["{ZEROS}", "{ZEROS}"]"#)
    );
}

/// Scenario "Three and four numbers, measured on the original".
#[test]
fn three_numbers_give_the_default_fourth() {
    assert_eq!(
        eval("[Color.new(1, 2, 3).alpha, Tone.new(1, 2, 3).gray, Color.new(1.5, 2.25, 3, 4).to_s]"),
        r#"[255.0, 0.0, "(1.500000, 2.250000, 3.000000, 4.000000)"]"#
    );
}

/// Scenario "Limits, measured on the original".
#[test]
fn components_are_clamped_with_the_limits_of_rgss2() {
    assert_eq!(
        eval("Color.new(-300, 300, 7.6, -7).to_s"),
        r#""(-255.000000, 255.000000, 7.600000, 0.000000)""#
    );
    assert_eq!(eval("Color.new(2**40, 2, 3).red"), "255.0");
    assert_eq!(
        eval("Tone.new(-300, 300, 1000.5, -5).to_s"),
        r#""(-255.000000, 255.000000, 255.000000, 0.000000)""#
    );
    assert_eq!(eval("Tone.new(0, 0, 0, 300).gray"), "255.0");
}

/// Scenario "Writers, measured on the original".
#[test]
fn a_writer_clamps_and_returns_the_value_given() {
    assert_eq!(
        eval(
            "c = Color.new(1, 2, 3, 4); \
             [(c.red = 300), c.red, (c.alpha = -1), c.alpha, (c.green = -255.9), c.green]"
        ),
        "[300, 255.0, -1, 0.0, -255.9, -255.0]"
    );
    assert_eq!(
        eval("t = Tone.new(1, 2, 3, 4); [(t.gray = 300), t.gray, (t.red = -300), t.red]"),
        "[300, 255.0, -300, -255.0]"
    );
}

/// Scenario "Set with numbers and without, measured on the original".
#[test]
fn set_takes_numbers_or_nothing_and_returns_the_receiver() {
    let source = |class: &str| {
        format!(
            "c = {class}.new(1, 2, 3, 4); a = [c.set(10, 20, 30).equal?(c), c.to_s]; \
             b = [c.set.equal?(c), c.to_s]; [a, b]"
        )
    };
    assert_eq!(
        eval(&source("Color")),
        format!(r#"[[true, "(10.000000, 20.000000, 30.000000, 255.000000)"], [true, "{ZEROS}"]]"#)
    );
    assert_eq!(
        eval(&source("Tone")),
        format!(r#"[[true, "(10.000000, 20.000000, 30.000000, 0.000000)"], [true, "{ZEROS}"]]"#)
    );
}

/// Scenario "Set with an object, measured on the original".
#[test]
fn set_copies_an_object_of_its_class() {
    for class in ["Color", "Tone"] {
        assert_eq!(
            eval(&format!(
                "c = {class}.new(1, 2, 3, 4); o = {class}.new(-9, 255, 2.5, 77); \
                 [c.set(o).equal?(c), c.equal?(o), c.to_s]"
            )),
            r#"[true, false, "(-9.000000, 255.000000, 2.500000, 77.000000)"]"#,
            "{class}"
        );
    }
}

/// Scenario "Between the two classes, measured on the original".
#[test]
fn set_copies_the_four_numbers_of_the_other_class() {
    assert_eq!(
        eval(
            "[Color.new.set(Tone.new(1, 2, 3, 4)).to_s, Tone.new.set(Color.new(1, 2, 3, 4)).to_s]"
        ),
        r#"["(1.000000, 2.000000, 3.000000, 4.000000)", "(1.000000, 2.000000, 3.000000, 4.000000)"]"#
    );
}

/// Scenario "One argument that is no such object, measured on the original".
#[test]
fn set_with_one_value_that_is_no_object_raises_type_error() {
    assert_eq!(
        eval("Color.new.set(1)"),
        "TypeError: wrong argument type Fixnum (expected Data)"
    );
    assert_eq!(
        eval("Color.new.set(nil)"),
        "TypeError: wrong argument type nil (expected Data)"
    );
    assert_eq!(
        eval("Tone.new.set(1)"),
        "TypeError: wrong argument type Fixnum (expected Data)"
    );
}

/// Scenario "An object of another native class" of `Color`: the original copies its bytes.
#[test]
fn set_with_an_object_of_another_native_class_raises_type_error() {
    assert_eq!(raised("Color.new.set(Rect.new(1, 2, 3, 4))"), "TypeError");
    assert_eq!(raised("Tone.new.set(Table.new(1))"), "TypeError");
}

// --- Rect of RGSS3 ---------------------------------------------------------------------------

/// Scenario "Without arguments, measured on the original".
#[test]
fn a_rect_without_arguments_is_zeros() {
    assert_eq!(eval("Rect.new.to_s"), r#""(0, 0, 0, 0)""#);
}

/// Scenario "Floats and negative values, measured on the original".
#[test]
fn floats_are_truncated_and_negative_values_kept() {
    assert_eq!(
        eval("Rect.new(1.9, -2.9, 3.5, 4.5).to_s"),
        r#""(1, -2, 3, 4)""#
    );
    assert_eq!(
        eval("Rect.new(-1, -2, -3, -4).to_s"),
        r#""(-1, -2, -3, -4)""#
    );
    assert_eq!(
        eval("r = Rect.new(1, 2, 3, 4); [(r.x = 1.9), r.x, (r.width = -5), r.width]"),
        "[1.9, 1, -5, -5]"
    );
}

/// Scenario "Set, measured on the original".
#[test]
fn set_takes_four_integers_or_a_rect_and_returns_the_receiver() {
    assert_eq!(
        eval(
            "r = Rect.new(1, 2, 3, 4); a = [r.set(10, 20, 30, 40).equal?(r), r.to_s]; \
             b = [r.set(Rect.new(-5, 6, -7, 8)).equal?(r), r.to_s]; \
             c = [r.empty.equal?(r), r.to_s]; [a, b, c]"
        ),
        r#"[[true, "(10, 20, 30, 40)"], [true, "(-5, 6, -7, 8)"], [true, "(0, 0, 0, 0)"]]"#
    );
}

/// Scenario "One argument that is no Rect, measured on the original".
#[test]
fn set_with_one_value_that_is_no_rect_raises_type_error() {
    assert_eq!(
        eval("Rect.new.set(1)"),
        "TypeError: wrong argument type Fixnum (expected Data)"
    );
    assert_eq!(
        eval("Rect.new.set(nil)"),
        "TypeError: wrong argument type nil (expected Data)"
    );
}

/// Scenario "An object of another native class" of `Rect`: the original copies its bytes.
#[test]
fn set_of_a_rect_with_an_object_of_another_native_class_raises_type_error() {
    assert_eq!(raised("Rect.new.set(Color.new(1, 2, 3, 4))"), "TypeError");
}

/// Scenario "Seen by the element": every form of `set` reaches the sprite the object
/// belongs to.
#[test]
fn a_change_by_set_reaches_the_element() {
    assert_eq!(
        eval(
            "bitmap = solid(4, 4, 255, 0, 0)
             sprite = Sprite.new
             begin
               sprite.bitmap = bitmap
               sprite.src_rect.set(Rect.new(0, 0, 2, 2))
               cut = [pixel(1, 1), pixel(3, 3)]
               sprite.color.set(Color.new(0, 0, 255, 255))
               blue = [pixel(1, 1), pixel(3, 3)]
               sprite.color.set
               [cut, blue, [pixel(1, 1), pixel(3, 3)]]
             ensure
               sprite.dispose
               bitmap.dispose
             end"
        ),
        "[[[255, 0, 0, 255], [0, 0, 0, 255]], [[0, 0, 255, 255], [0, 0, 0, 255]], \
         [[255, 0, 0, 255], [0, 0, 0, 255]]]"
    );
}

// --- Argument errors of the value classes in RGSS3 --------------------------------------------

/// Scenario "Color and Tone, measured on the original".
#[test]
fn a_wrong_count_of_a_colour_or_tone_names_the_expected_count_first() {
    let calls = [
        ("Color.new(1)", "3 for 1"),
        ("Color.new(1, 2)", "3 for 2"),
        ("Color.new(1, 2, 3, 4, 5)", "4 for 5"),
        ("Color.new(Color.new)", "3 for 1"),
        ("Tone.new(1)", "3 for 1"),
        ("Tone.new(1, 2)", "3 for 2"),
        ("Tone.new(1, 2, 3, 4, 5)", "4 for 5"),
        ("Color.new.set(1, 2)", "3 for 2"),
        ("Color.new.set(1, 2, 3, 4, 5)", "4 for 5"),
        ("Tone.new.set(1, 2, 3, 4, 5)", "4 for 5"),
    ];
    for (source, counts) in calls {
        assert_eq!(
            eval(source),
            format!("ArgumentError: wrong number of arguments ({counts})"),
            "{source}"
        );
    }
}

/// Scenario "Rect, measured on the original".
#[test]
fn a_wrong_count_of_a_rect_names_four_first() {
    let calls = [
        ("Rect.new(1)", 1),
        ("Rect.new(1, 2)", 2),
        ("Rect.new(1, 2, 3)", 3),
        ("Rect.new(1, 2, 3, 4, 5)", 5),
        ("Rect.new(Rect.new)", 1),
        ("r = Rect.new; r.set", 0),
        ("r = Rect.new; r.set(1, 2)", 2),
        ("r = Rect.new; r.set(1, 2, 3)", 3),
        ("r = Rect.new; r.set(1, 2, 3, 4, 5)", 5),
    ];
    for (source, given) in calls {
        assert_eq!(
            eval(source),
            format!("ArgumentError: wrong number of arguments (4 for {given})"),
            "{source}"
        );
    }
}

/// Scenario "Out of range": `RangeError`, whose message is not the original's (a 64-bit
/// guest; `docs/decisions.md`, "Word size").
#[test]
fn a_value_out_of_the_range_of_32_bits_raises_range_error() {
    for source in [
        "Rect.new(2**31, 2, 3, 4)",
        "Rect.new.x = 2**31",
        "Rect.new(1e10, 0, 0, 0)",
    ] {
        assert_eq!(raised(source), "RangeError", "{source}");
    }
}

/// Scenario "Numbers of the wrong kind, measured on the original", the calls on `Color`.
#[test]
fn a_component_of_the_wrong_kind_raises_the_error_of_ruby_1_9() {
    assert_eq!(
        eval(r#"Color.new("1", 2, 3)"#),
        "TypeError: no implicit conversion to float from string"
    );
    assert_eq!(
        eval("Color.new(nil, 2, 3)"),
        "TypeError: no implicit conversion to float from nil"
    );
    assert_eq!(
        eval("Color.new.red = nil"),
        "TypeError: no implicit conversion to float from nil"
    );
}

/// Scenario "Numbers of the wrong kind, measured on the original", the calls on `Rect`.
#[test]
fn a_value_of_a_rect_of_the_wrong_kind_raises_the_error_of_ruby_1_9() {
    assert_eq!(
        eval(r#"Rect.new("1", 2, 3, 4)"#),
        "TypeError: can't convert String into Integer"
    );
    assert_eq!(
        eval("Rect.new(nil, 2, 3, 4)"),
        "TypeError: no implicit conversion from nil to integer"
    );
    assert_eq!(
        eval(r#"Rect.new.x = "a""#),
        "TypeError: can't convert String into Integer"
    );
}

/// Scenario "A method of fixed arity, measured on the original": the interpreter counts.
#[test]
fn a_method_of_fixed_arity_is_counted_by_the_interpreter() {
    assert_eq!(
        eval("Color.new.to_s(1)"),
        "ArgumentError: wrong number of arguments(1 for 0)"
    );
}
