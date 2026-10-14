//! The class `Font` in a game of RGSS3 over Ruby 1.9.2: the scenarios of "Font of RGSS3",
//! "Font sizes of RGSS3", "Font names of RGSS3" and "Font flags, colours and copies of
//! RGSS3" of `rgss-bitmap` that are about `Font` alone, and the arities of "Method surface of
//! Bitmap and Font in RGSS3". Those that draw or measure are in `bitmap_rgss3.rs`. A file of
//! this package alone, because the classes are defined once per process, as the runtime of
//! the run has them, and RGSS3 runs over Ruby 1.9.2 only.
//!
//! The expected values said to be measured are those of the original runtime of VX Ace (DLL
//! versions 3.0.0.1 and 3.0.1.1, 2026-10-13, `ace-v1-bitmap-REPORT.md` sections 4 and 7,
//! kept outside the repository). The harness's text backend has the families "Test Sans"
//! and "Liberation Sans", which stand for fonts that exist.
//!
//! The snippets of a binary share the interpreter: a snippet that changes the font defaults
//! puts them back before it ends ([`with_defaults`]), with the objects it found, so that the
//! next snippet sees the defaults of RGSS3 as a game starts with them.
#![cfg(not(vkrgss_interpreter_absent))]

use vkrgss_binding_rgss3::testing;
use vkrgss_vfs::Runtime;

/// `inspect` of the value of `source` in a game of RGSS3, or the class and the message of
/// the exception that left it. `error { ... }` is the class and message of the exception
/// the block raises.
fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    let source = format!(
        "def error; yield; rescue Exception; \"#{{$!.class}}: #{{$!.message}}\"; end\n{source}"
    );
    match testing::eval(&source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// [`eval`] of `source` with every default of `Font` put back afterwards, the objects the
/// readers returned before included.
fn with_defaults(source: &str) -> String {
    eval(&format!(
        "saved = [Font.default_name, Font.default_size, Font.default_bold, \
           Font.default_italic, Font.default_shadow, Font.default_outline, \
           Font.default_color, Font.default_out_color]
         begin
           {source}
         ensure
           Font.default_name, Font.default_size, Font.default_bold, Font.default_italic, \
             Font.default_shadow, Font.default_outline, Font.default_color, \
             Font.default_out_color = *saved
         end"
    ))
}

const WHITE: &str = "(255.000000, 255.000000, 255.000000, 255.000000)";
const OUT_COLOR: &str = "(0.000000, 0.000000, 0.000000, 128.000000)";
const NIL_INTEGER: &str = "TypeError: no implicit conversion from nil to integer";
const NIL_COLOR: &str = "TypeError: can't convert NilClass into Color";

// --- Font of RGSS3 ----------------------------------------------------------------------------

/// Scenario "Defaults, measured on the original".
#[test]
fn the_defaults_are_those_of_rgss3() {
    assert_eq!(
        eval(
            "[Font.default_name, Font.default_size, Font.default_bold, Font.default_italic, \
              Font.default_shadow, Font.default_outline, Font.default_color.to_s, \
              Font.default_out_color.to_s]"
        ),
        format!(r#"["VL Gothic", 24, false, false, false, true, "{WHITE}", "{OUT_COLOR}"]"#)
    );
}

/// Scenario "A new font, measured on the original".
#[test]
fn a_new_font_has_the_defaults_of_rgss3() {
    assert_eq!(
        eval(
            "f = Font.new; [f.name, f.name.encoding.name, f.size, f.bold, f.italic, f.shadow, \
             f.outline, f.color.to_s, f.out_color.to_s]"
        ),
        format!(
            r#"["VL Gothic", "ASCII-8BIT", 24, false, false, false, true, "{WHITE}", "{OUT_COLOR}"]"#
        )
    );
}

// --- Font sizes of RGSS3 ----------------------------------------------------------------------

/// Scenario "Fractions, measured on the original".
#[test]
fn a_size_keeps_the_number_given() {
    assert_eq!(
        eval(
            "f = Font.new; \
             [(f.size = 24.7), f.size, (f.size = 96.99), f.size, (f.size = 6), f.size]"
        ),
        "[24.7, 24.7, 96.99, 96.99, 6, 6]"
    );
    assert_eq!(
        with_defaults("Font.default_size = 20.5; Font.default_size"),
        "20.5"
    );
}

/// Scenario "Limits, measured on the original".
#[test]
fn a_size_outside_6_to_96_raises_argument_error() {
    let size = "ArgumentError: bad value for size";
    let default_size = "ArgumentError: bad value for default_size";
    assert_eq!(
        with_defaults(
            "f = Font.new; f.size = 30
             [error { f.size = 5 }, error { f.size = 5.9 }, error { f.size = 97 }, \
              error { f.size = 0 }, error { f.size = 1000 }, error { Font.new(\"Arial\", 5) }, \
              error { Font.new(\"Arial\", 97) }, error { Font.default_size = 5 }, \
              error { Font.default_size = 97 }, f.size, Font.default_size]"
        ),
        format!(
            r#"["{size}", "{size}", "{size}", "{size}", "{size}", "{size}", "{size}", "{default_size}", "{default_size}", 30, 24]"#
        )
    );
}

/// Scenario "Wrong kinds, measured on the original".
#[test]
fn a_size_that_is_no_number_raises_type_error() {
    assert_eq!(
        eval(
            "f = Font.new; [error { f.size = nil }, error { f.size = \"x\" }, \
             error { Font.new(\"Arial\", nil) }, f.size]"
        ),
        format!(
            r#"["{NIL_INTEGER}", "TypeError: can't convert String into Integer", "{NIL_INTEGER}", 24]"#
        )
    );
}

// --- Font names of RGSS3 ----------------------------------------------------------------------

/// Scenario "The object is kept, measured on the original".
#[test]
fn a_name_keeps_the_object_given() {
    assert_eq!(
        eval(
            "a = [\"Arial\", \"Tahoma\", \"x\"]; f = Font.new; f.name = a; \
             [f.name.equal?(a), (f.name = nil), f.name, (f.name = 1), f.name, \
              Font.new(nil).name, Font.new(1).name, Font.new([\"Nope\", \"Arial\"]).name]"
        ),
        r#"[true, nil, nil, 1, 1, nil, 1, ["Nope", "Arial"]]"#
    );
}

/// Scenario "The default name, measured on the original".
#[test]
fn the_default_name_keeps_the_object_given() {
    assert_eq!(
        with_defaults(
            "Font.default_name = [\"Nope\", \"Arial\"]; [Font.default_name, Font.new.name]"
        ),
        r#"[["Nope", "Arial"], ["Nope", "Arial"]]"#
    );
}

/// Scenario "Exact existence".
#[test]
fn a_font_exists_by_its_exact_name() {
    assert_eq!(
        eval(
            "[\"Test Sans\", :\"Test Sans\", \"test sans\", \"TEST SANS\", \
              \"Test Sans Regular\", \"\", \" \", \"Nope\", nil, 1, [\"Test Sans\"]]\
             .map { |name| Font.exist?(name) }"
        ),
        "[true, true, false, false, false, false, false, false, false, false, false]"
    );
}

// --- Font flags, colours and copies of RGSS3 --------------------------------------------------

/// Scenario "Flags, measured on the original".
#[test]
fn a_flag_is_true_or_false_and_its_default_keeps_the_object() {
    let values = ["nil", "false", "true", "0", "1", "\"x\""];
    let per_value: Vec<String> = values
        .iter()
        .map(|value| {
            let truth = !matches!(*value, "nil" | "false");
            format!("[{truth}, {value}, {truth}]")
        })
        .collect();
    let per_flag = format!("[{}]", per_value.join(", "));
    assert_eq!(
        with_defaults(
            "%w(outline shadow bold italic).map do |flag|
               [nil, false, true, 0, 1, \"x\"].map do |v|
                 f = Font.new; f.send(\"#{flag}=\", v); read = f.send(flag)
                 Font.send(\"default_#{flag}=\", v)
                 [read, Font.send(\"default_#{flag}\"), Font.new.send(flag)]
               end
             end"
        ),
        format!("[{}]", [per_flag.as_str(); 4].join(", "))
    );
}

/// Scenario "Colours are copied into the font, measured on the original".
#[test]
fn a_colour_is_copied_into_the_font() {
    assert_eq!(
        eval(
            "f = Font.new; c = Color.new(9, 8, 7); f.out_color = c; f.color = c; \
             [f.out_color.equal?(c), f.color.equal?(c), f.out_color.equal?(f.out_color), \
              f.out_color.to_s]"
        ),
        r#"[false, false, true, "(9.000000, 8.000000, 7.000000, 255.000000)"]"#
    );
}

/// Scenario "Default colours are kept, measured on the original".
#[test]
fn a_default_colour_is_kept() {
    for (default, attribute) in [
        ("default_out_color", "out_color"),
        ("default_color", "color"),
    ] {
        assert_eq!(
            with_defaults(&format!(
                "c = Color.new(100, 8, 7); Font.{default} = c; f = Font.new; c.red = 50; \
                 [Font.{default}.equal?(c), f.{attribute}.equal?(c), f.{attribute}.red, \
                  Font.new.{attribute}.red]"
            )),
            "[true, false, 100.0, 50.0]",
            "{default}"
        );
    }
}

/// Scenario "Wrong colours, measured on the original".
#[test]
fn a_colour_of_another_class_raises_type_error() {
    assert_eq!(
        with_defaults(
            "f = Font.new; [error { f.color = nil }, error { f.out_color = nil }, \
             error { f.out_color = Tone.new(1, 2, 3) }, error { Font.default_color = nil }, \
             error { Font.default_out_color = nil }, Font.default_out_color.to_s]"
        ),
        format!(
            r#"["{NIL_COLOR}", "{NIL_COLOR}", "TypeError: can't convert Tone into Color", "{NIL_COLOR}", "{NIL_COLOR}", "{OUT_COLOR}"]"#
        )
    );
}

/// Scenario "Arguments of Font.new, measured on the original".
#[test]
fn a_new_font_takes_at_most_two_arguments() {
    assert_eq!(
        eval("[Font.new(\"Arial\").name, Font.new(\"Arial\", 30).size]"),
        r#"["Arial", 30]"#
    );
    assert_eq!(
        eval("Font.new(\"Arial\", 30, 1)"),
        "ArgumentError: wrong number of arguments (3 for 0..2)"
    );
}

/// Scenario "Copies and comparison, measured on the original".
#[test]
fn a_copy_has_every_attribute_and_colours_of_its_own() {
    assert_eq!(
        eval(
            "f = Font.new(\"Arial\"); f.outline = false; f.out_color = Color.new(1, 2, 3, 4); \
             g = f.dup; h = f.clone; \
             [g.outline, g.out_color.to_s, g.color.equal?(f.color), \
              g.out_color.equal?(f.out_color), h.out_color.to_s, Font.new == Font.new, f == f]"
        ),
        r#"[false, "(1.000000, 2.000000, 3.000000, 4.000000)", false, false, "(1.000000, 2.000000, 3.000000, 4.000000)", false, true]"#
    );
}

/// Scenario "No dump, measured on the original".
#[test]
fn a_font_cannot_be_dumped() {
    assert_eq!(
        eval("Marshal.dump(Font.new)"),
        "TypeError: no marshal_dump is defined for class Font"
    );
}

// --- Method surface ---------------------------------------------------------------------------

/// Scenario "New methods of Font".
#[test]
fn the_new_methods_of_font_have_the_listed_arities() {
    assert_eq!(
        eval(
            "[Font.method(:default_outline).arity, Font.method(:default_out_color=).arity, \
              Font.instance_method(:outline=).arity, Font.instance_method(:out_color).arity]"
        ),
        "[0, 1, 1, 0]"
    );
}
