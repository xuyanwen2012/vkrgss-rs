//! The value classes `Color`, `Tone`, `Rect` and `Table` in a game of RGSS3: the scenarios
//! of `rgss-values` that name RGSS3. A file of this package alone, because the classes are
//! defined once per process, as the runtime of the run has them, and RGSS3 runs over Ruby
//! 1.9.2 only.
//!
//! The values are those measured on the original runtime of RGSS3 on 2026-10-13
//! (`ace-v1-bitmap-REPORT.md` section 3, `ace-v1-api-REPORT.md` sections 0, 2.4 and 3.3, kept
//! outside the repository); a test says where vkrgss does something else.
#![cfg(not(vkrgss_interpreter_absent))]
// The support module of `vkrgss-binding` names the binding by that name.
extern crate vkrgss_binding_rgss3 as vkrgss_binding;

#[path = "../../vkrgss-binding/tests/scene_support/mod.rs"]
mod scene_support;

use vkrgss_binding::testing;
use vkrgss_vfs::Runtime;

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

// --- Table of RGSS3 --------------------------------------------------------------------------

/// Scenario "Sizes, measured on the original".
#[test]
fn a_table_has_one_to_three_sizes() {
    assert_eq!(
        eval(
            "[Table.new(2), Table.new(2, 3), Table.new(0), Table.new(-1), Table.new(0, 0, 0), \
              Table.new(1.9, 2.9)].map { |t| [t.xsize, t.ysize, t.zsize] }"
        ),
        "[[2, 1, 1], [2, 3, 1], [0, 1, 1], [0, 1, 1], [0, 0, 0], [1, 2, 1]]"
    );
}

/// Scenario "Elements, measured on the original".
#[test]
fn an_element_keeps_the_low_16_bits() {
    assert_eq!(
        eval(
            "t = Table.new(2, 2, 2); t[0, 0, 0] = 40000; t[1, 1, 1] = -40000; \
             a = [t[0, 0, 0], t[1, 1, 1]]; t[1, 1, 1] = 1.9; \
             [a, t[1, 1, 1], t[2, 0, 0], t[-1, 0, 0], (t[2, 0, 0] = 5), t[1.9, 0, 0]]"
        ),
        "[[-25536, 25536], 1, nil, nil, 5, 0]"
    );
}

/// Scenario "Resize, measured on the original".
#[test]
fn resize_returns_the_table() {
    assert_eq!(
        eval(
            "t = Table.new(2, 2, 2); \
             [t.resize(3).equal?(t), [t.xsize, t.ysize, t.zsize], t.resize(-1).xsize]"
        ),
        "[true, [3, 1, 1], 0]"
    );
    assert_eq!(
        eval("d = Table.new(2, 1, 1).dup; [d.xsize, d.ysize, d.zsize]"),
        "[2, 1, 1]"
    );
}

// --- Comparison of the value classes in RGSS3 -------------------------------------------------

/// Scenario "Equal values, measured on the original".
#[test]
fn equal_values_compare_equal() {
    assert_eq!(
        eval(
            "a = Color.new(1.5, 0, 0); b = Color.new(1.5, 0, 0); \
             [a == b, a.eql?(b), a === b, a.equal?(b), a == Color.new(1.5, 0, 1)]"
        ),
        "[true, true, true, false, false]"
    );
    assert_eq!(
        eval("Tone.new(-255, 6, 7, 255) == Tone.new(-255, 6, 7, 255)"),
        "true"
    );
    assert_eq!(
        eval("Rect.new(1, 0, -5, 0) == Rect.new(1, 0, -5, 0)"),
        "true"
    );
}

/// Scenario "Another class, measured on version 3.0.1 of the original": false, never the
/// `TypeError` of version 3.0.0 (`docs/decisions.md`, "Text and bitmaps of RGSS3").
#[test]
fn another_class_compares_unequal_without_raising() {
    assert_eq!(
        eval("c = Color.new(1, 2, 3, 4); [c == nil, c == 5, c == Tone.new(1, 2, 3, 4)]"),
        "[false, false, false]"
    );
    assert_eq!(
        eval("t = Tone.new(1, 2, 3, 4); [t == nil, t == Color.new(1, 2, 3, 4)]"),
        "[false, false]"
    );
    assert_eq!(
        eval("r = Rect.new(1, 2, 3, 4); [r == nil, r == 5, r == Color.new(1, 2, 3, 4)]"),
        "[false, false, false]"
    );
}

/// Scenario "Hash, measured on the original".
#[test]
fn the_hash_is_not_computed_from_the_values() {
    assert_eq!(
        eval("Color.new(1, 2, 3).hash == Color.new(1, 2, 3).hash"),
        "false"
    );
}

/// Scenario "Copies, measured on the original".
#[test]
fn a_copy_is_independent() {
    assert_eq!(
        eval("a = Color.new(1, 2, 3, 4); b = a.dup; b.red = 9; [a.to_s, b.to_s, a.clone == a]"),
        r#"["(1.000000, 2.000000, 3.000000, 4.000000)", "(9.000000, 2.000000, 3.000000, 4.000000)", true]"#
    );
}

// --- Text forms of the value classes in RGSS3 -------------------------------------------------

/// Scenario "Text, measured on the original".
#[test]
fn to_s_gives_the_components() {
    assert_eq!(
        eval(
            "[Color.new(1, 2.5, 3, 4).to_s, Tone.new(1, 2.5, 3, 4).to_s, Rect.new(1, -2, 3, 4).to_s]"
        ),
        r#"["(1.000000, 2.500000, 3.000000, 4.000000)", "(1.000000, 2.500000, 3.000000, 4.000000)", "(1, -2, 3, 4)"]"#
    );
}

/// Scenario "Inspect, measured on the original": `Object#inspect` of a data object of Ruby
/// 1.9.2 is its `to_s`.
#[test]
fn inspect_gives_the_text_of_to_s() {
    assert_eq!(
        eval("[Tone.new.inspect, Rect.new.inspect, Color.new(1, 2, 3).inspect]"),
        format!(r#"["{ZEROS}", "(0, 0, 0, 0)", "(1.000000, 2.000000, 3.000000, 255.000000)"]"#)
    );
}

/// Scenario "Encoding, measured on the original".
#[test]
fn the_texts_are_bytes() {
    assert_eq!(
        eval(
            "[Color.new.to_s, Tone.new.to_s, Rect.new.to_s, Color.new.inspect]\
               .map { |text| text.encoding.name }"
        ),
        r#"["ASCII-8BIT", "ASCII-8BIT", "ASCII-8BIT", "ASCII-8BIT"]"#
    );
}

/// Scenario "Table, measured on the original": a table has no text form of its own.
#[test]
fn a_table_is_inspected_as_an_object() {
    assert_eq!(eval(r"Table.new(1).inspect =~ /\A#<Table:0x\h+>\z/"), "0");
}

// --- Serialized forms of the value classes in RGSS3 ------------------------------------------

/// Scenario "Bytes, measured on the original".
#[test]
fn the_dumps_hold_the_bytes_of_rgss2() {
    assert_eq!(
        eval(r#"Color.new(1, 2, 3, 4)._dump(0).unpack("H*")"#),
        r#"["000000000000f03f000000000000004000000000000008400000000000001040"]"#
    );
    assert_eq!(
        eval(r#"Rect.new(1, 2, 3, 4)._dump(0).unpack("H*")"#),
        r#"["01000000020000000300000004000000"]"#
    );
    assert_eq!(
        eval(r#"t = Table.new(2, 1, 1); t[0, 0, 0] = 7; t._dump(0).unpack("l5s*")"#),
        "[3, 2, 1, 1, 2, 7, 0]"
    );
}

/// Scenario "Sizes of a table in its dump, measured on the original".
#[test]
fn a_dumped_table_writes_its_dimensions_and_sizes() {
    assert_eq!(
        eval(r#"Table.new(0)._dump(0).unpack("l5")"#),
        "[1, 0, 1, 1, 0]"
    );
    assert_eq!(
        eval(r#"t = Table.new(2, 2, 2); t.resize(3); t._dump(0).unpack("l5")"#),
        "[1, 3, 1, 1, 3]"
    );
}

/// Scenario "The argument and the encoding".
#[test]
fn the_argument_of_dump_is_ignored_and_the_dump_is_bytes() {
    assert_eq!(
        eval(
            "c = Color.new(1, 2, 3, 4); \
             [c._dump(0) == c._dump(-1), c._dump(0).encoding.name, Marshal.dump(c)[2]]"
        ),
        r#"[true, "ASCII-8BIT", "u"]"#
    );
}

/// Scenario "Loading what was dumped, measured on the original".
#[test]
fn loading_a_dump_gives_its_values() {
    let four = r#""(1.000000, 2.000000, 3.000000, 4.000000)""#;
    assert_eq!(
        eval(r#"Color._load([1.0, 2.0, 3.0, 4.0].pack("d4")).to_s"#),
        four
    );
    assert_eq!(
        eval(r#"Tone._load([1.0, 2.0, 3.0, 4.0].pack("d4")).to_s"#),
        four
    );
    assert_eq!(
        eval(r#"Rect._load([1, 2, 3, 4].pack("l4")).to_s"#),
        r#""(1, 2, 3, 4)""#
    );
    assert_eq!(
        eval(
            r#"t = Table._load([1, 2, 1, 1, 2, 5, 6].pack("l5s2"))
               [t.xsize, t.ysize, t.zsize, t[0], t[1]]"#
        ),
        "[2, 1, 1, 5, 6]"
    );
}

/// Scenario "A colour of another length, measured on the original".
#[test]
fn a_colour_of_another_length_loads_as_zeros() {
    assert_eq!(eval(r#"Color._load("abc").to_s"#), format!(r#""{ZEROS}""#));
}

/// Scenario "A short rectangle": the missing bytes are zeros, where the original reads the
/// memory behind the string (its fourth value was 8199).
#[test]
fn a_short_rectangle_loads_with_zeros_for_what_is_missing() {
    assert_eq!(eval(r#"Rect._load("ab").to_s"#), r#""(25185, 0, 0, 0)""#);
}

/// Scenario "Dimensions of a loaded table, measured on the original".
#[test]
fn a_loaded_table_keeps_the_dimensions_of_its_dump() {
    let load = r#"t = Table._load(([1, 2, 2, 2, 8] + [0] * 8).pack("l5s8"))"#;
    assert_eq!(
        eval(&format!("{load}; [t.xsize, t.ysize, t[0]]")),
        "[2, 2, 0]"
    );
    assert_eq!(raised(&format!("{load}; t[0, 0]")), "ArgumentError");
}

/// Scenario "A short table": the original ends with a page fault.
#[test]
fn a_short_table_raises_and_the_interpreter_goes_on() {
    assert_eq!(raised(r#"Table._load("ab")"#), "ArgumentError");
    assert_eq!(
        eval(r#"begin; Table._load("ab"); rescue ArgumentError; :went_on; end"#),
        ":went_on"
    );
}

/// Scenario "A subclass and instance variables, measured on the original".
#[test]
fn a_dump_names_a_subclass_and_writes_instance_variables() {
    assert_eq!(
        eval(
            "class ValuesRgss3Colour < Color; end
             [Marshal.dump(ValuesRgss3Colour.new(1, 2, 3)).include?('ValuesRgss3Colour'),
              Marshal.dump(Color.new.tap { |c| c.instance_variable_set(:@zz, 5) })[2]]"
        ),
        r#"[true, "I"]"#
    );
}

/// Scenario "Round trip".
#[test]
fn values_round_trip_through_marshal() {
    assert_eq!(
        eval(
            "values = [Color.new(1, 2, 3, 4), Tone.new(-1, 2, 3, 4), Rect.new(1, 2, 3, 4)]
             loaded = Marshal.load(Marshal.dump(values))
             values.zip(loaded).map { |value, copy| value == copy }"
        ),
        "[true, true, true]"
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

/// Scenario "Table, measured on the original".
#[test]
fn a_wrong_count_of_indices_names_the_dimensions_first() {
    let calls = [
        ("t[0]", "3 for 1"),
        ("t[0, 0]", "3 for 2"),
        ("t[0, 0, 0, 0]", "3 for 4"),
        ("t[0] = 5", "3 for 2"),
        ("t.resize(3, 3); t[0]", "2 for 1"),
    ];
    for (call, counts) in calls {
        assert_eq!(
            eval(&format!("t = Table.new(2, 2, 2); {call}")),
            format!("ArgumentError: wrong number of arguments ({counts})"),
            "{call}"
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

/// Scenario "Numbers of the wrong kind, measured on the original", the calls on `Table`.
#[test]
fn a_size_or_index_of_the_wrong_kind_raises_the_error_of_ruby_1_9() {
    assert_eq!(
        eval(r#"Table.new("a")"#),
        "TypeError: can't convert String into Integer"
    );
    assert_eq!(
        eval("Table.new(nil)"),
        "TypeError: no implicit conversion from nil to integer"
    );
    assert_eq!(
        eval(r#"Table.new(1)["a"]"#),
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
