//! The class `Bitmap` in a game of RGSS3 over Ruby 1.9.2: the scenarios of "Bitmap font and
//! copies of RGSS3", "Text of a bitmap of RGSS3", "Creation from a size in RGSS3" and
//! "Argument errors of Bitmap in RGSS3" of `rgss-bitmap`, and those of `Font` that draw,
//! measure or make a bitmap. A file of this package alone, because the classes are defined
//! once per process, as the runtime of the run has them, and RGSS3 runs over Ruby 1.9.2
//! only.
//!
//! The expected values said to be measured are those of the original runtime of VX Ace (DLL
//! versions 3.0.0.1 and 3.0.1.1, 2026-10-13, `ace-v1-bitmap-REPORT.md` sections 2, 4, 5, 6g
//! and 7, kept outside the repository). The harness's text backend has the families "Test
//! Sans" and "Liberation Sans" and draws every character as a cell half the font size wide
//! and the font size high. No snippet makes a bitmap larger than 2049 by 2048, and a large
//! one is disposed before the snippet ends.
#![cfg(not(vkxp_interpreter_absent))]

use vkxp_binding_rgss3::testing;
use vkxp_vfs::Runtime;

/// `inspect` of the value of `source` in a game of RGSS3, or the class and the message of
/// the exception that left it. `error { ... }` is the class and message of the exception
/// the block raises; `b` is a new bitmap of 40 by 40, `r` a `Rect` (0, 0, 4, 4).
fn eval(source: &str) -> String {
    testing::start_as(Runtime::Rgss3);
    let source = format!(
        "def error; yield; rescue Exception; \"#{{$!.class}}: #{{$!.message}}\"; end
         b = Bitmap.new(40, 40); r = Rect.new(0, 0, 4, 4)
         begin
           {source}
         ensure
           b.dispose
         end"
    );
    match testing::eval(&source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// What `eval` returns for a list of messages, each `ArgumentError: wrong number of
/// arguments` and the counts.
fn counts(counts: &[&str]) -> String {
    let messages: Vec<String> = counts
        .iter()
        .map(|counts| format!("\"ArgumentError: wrong number of arguments {counts}\""))
        .collect();
    format!("[{}]", messages.join(", "))
}

/// Whether every pixel of the bitmap `b` is (0, 0, 0, 0), as a Ruby expression.
const UNTOUCHED: &str = "(0...b.width).all? { |x| (0...b.height).all? { |y| \
                         c = b.get_pixel(x, y); [c.red, c.green, c.blue, c.alpha] == [0, 0, 0, 0] } }";

const FAILED: &str = "RGSSError: failed to create bitmap";

// --- Font of RGSS3 ----------------------------------------------------------------------------

/// Scenario "A new bitmap after the defaults changed, measured on the original".
#[test]
fn a_new_bitmap_takes_the_defaults_of_the_moment() {
    assert_eq!(
        eval(
            "saved = [Font.default_size, Font.default_outline, Font.default_out_color]
             begin
               Font.default_size = 31; Font.default_outline = false
               Font.default_out_color = Color.new(1, 1, 1, 1)
               c = Bitmap.new(4, 4); f = c.font; c.dispose
               [f.size, f.outline, f.out_color.to_s]
             ensure
               Font.default_size, Font.default_outline, Font.default_out_color = *saved
             end"
        ),
        r#"[31, false, "(1.000000, 1.000000, 1.000000, 1.000000)"]"#
    );
}

/// Scenario "Drawn with the integer, measured on the original".
#[test]
fn a_size_is_measured_with_its_integer() {
    assert_eq!(
        eval(
            "b.font.size = 24.7; a = b.text_size(\"Hello\").to_s; b.font.size = 24; \
             a == b.text_size(\"Hello\").to_s"
        ),
        "true"
    );
}

/// Scenario "No name, measured on the original": nothing is drawn. Drawn with a name, the
/// same text changes the bitmap.
#[test]
fn a_font_without_a_name_draws_nothing() {
    assert_eq!(
        eval(&format!(
            "b.font.name = \"\"; drawn = b.draw_text(0, 0, 40, 40, \"x\"); empty = {UNTOUCHED}
             b.font.name = []; listed = b.draw_text(0, 0, 40, 40, \"x\"); none = {UNTOUCHED}
             b.font.name = \"Test Sans\"; b.draw_text(0, 0, 40, 40, \"x\"); named = {UNTOUCHED}
             [drawn, empty, listed, none, named]"
        )),
        "[nil, true, nil, true, false]"
    );
}

/// Scenario "The size of no name".
#[test]
fn a_font_without_a_name_has_no_size() {
    assert_eq!(
        eval("b.font.name = \"\"; b.text_size(\"x\").to_s"),
        r#""(0, 0, 0, 0)""#
    );
}

// --- Bitmap font and copies of RGSS3 ----------------------------------------------------------

/// Scenario "Everything is copied, measured on the original".
#[test]
fn assigning_a_font_copies_every_attribute() {
    assert_eq!(
        eval(
            "f = Font.new(\"Arial\", 30); f.bold = f.italic = f.shadow = true; f.outline = false
             f.color = Color.new(1, 2, 3, 4); f.out_color = Color.new(5, 6, 7, 8)
             b.font = f; f.size = 40; g = b.font
             [g.name, g.size, g.bold, g.italic, g.shadow, g.outline, g.color.to_s, \
              g.out_color.to_s, g.equal?(f), g.color.equal?(f.color), \
              g.out_color.equal?(f.out_color)]"
        ),
        r#"["Arial", 30, true, true, true, false, "(1.000000, 2.000000, 3.000000, 4.000000)", "(5.000000, 6.000000, 7.000000, 8.000000)", false, false, false]"#
    );
}

/// Scenario "Not a font, measured on the original".
#[test]
fn assigning_anything_but_a_font_raises_type_error() {
    assert_eq!(
        eval("[error { b.font = nil }, error { b.font = \"Arial\" }]"),
        r#"["TypeError: can't convert NilClass into Font", "TypeError: can't convert String into Font"]"#
    );
}

/// Scenario "Copies of a bitmap, measured on the original".
#[test]
fn a_copy_of_a_bitmap_has_a_font_of_its_own() {
    assert_eq!(
        eval(
            "b.font.size = 40; b.font.shadow = true; b.font.out_color.red = 99
             c = b.dup; d = b.clone; c.font.size = 10
             result = [[c.font.size, c.font.shadow, c.font.out_color.red], \
                       [d.font.size, d.font.shadow, d.font.out_color.red], b.font.size, \
                       c.font.equal?(b.font)]
             c.dispose; d.dispose
             result"
        ),
        "[[10, true, 99.0], [40, true, 99.0], 40, false]"
    );
}

// --- Text of a bitmap of RGSS3 ----------------------------------------------------------------

/// Scenario "The work rectangle": the text pass of RGSS3 writes its colour with alpha 0 over
/// the pixels of the work rectangle that no glyph covers. The cell of "x" at size 24 is 12
/// by 24 from (0, 7) and does not reach (38, 38).
#[test]
fn the_work_rectangle_is_written() {
    assert_eq!(
        eval(
            "b.draw_text(0, 0, 40, 40, \"x\"); c = b.get_pixel(38, 38)
             [c.red, c.green, c.blue, c.alpha].map { |v| v.to_i }"
        ),
        "[255, 255, 255, 0]"
    );
}

/// Scenario "Text of any class, measured on the original".
#[test]
fn a_text_of_any_class_is_drawn_as_its_to_s() {
    assert_eq!(
        eval(
            "[b.draw_text(0, 0, 40, 40, 123), b.draw_text(0, 0, 40, 40, nil), \
              b.draw_text(0, 0, 40, 40, :a), b.draw_text(Rect.new(0, 0, 40, 40), \"a\"), \
              b.draw_text(0.5, 0, 40.9, 40, \"a\", 1.9), b.text_size(nil).to_s]"
        ),
        r#"[nil, nil, nil, nil, nil, "(0, 0, 0, 0)"]"#
    );
}

/// Scenario "Measured text".
#[test]
fn a_text_is_measured_without_the_outline() {
    assert_eq!(
        eval(
            "same = b.text_size(123).to_s == b.text_size(\"123\").to_s
             b.font.outline = true; on = b.text_size(\"Hello\").to_s
             b.font.outline = false; off = b.text_size(\"Hello\").to_s
             [same, on == off]"
        ),
        "[true, true]"
    );
}

/// Scenario "Alignment of the wrong kind, measured on the original".
#[test]
fn an_alignment_that_is_no_number_raises_type_error() {
    assert_eq!(
        eval(
            "[error { b.draw_text(0, 0, 40, 40, \"a\", nil) }, \
              error { b.draw_text(0, 0, 40, 40, \"a\", \"x\") }]"
        ),
        r#"["TypeError: no implicit conversion from nil to integer", "TypeError: can't convert String into Integer"]"#
    );
}

// --- Creation from a size in RGSS3 ------------------------------------------------------------

/// Scenario "Sizes, measured on the original".
#[test]
fn a_new_bitmap_has_no_limit_of_4194304_pixels() {
    assert_eq!(
        eval(
            "[[2049, 2048], [65536, 1], [10, -5], [3.9, 2.9]].map do |w, h|
               c = Bitmap.new(w, h); size = [c.width, c.height]; c.dispose; size
             end"
        ),
        "[[2049, 2048], [65536, 1], [10, 5], [3, 2]]"
    );
}

/// Scenario "Refused sizes, measured on the original": the last two are above the largest
/// bitmap of the arena, and are refused before memory is taken.
#[test]
fn a_size_without_pixels_or_too_large_raises_rgss_error() {
    let refused = [FAILED; 8]
        .map(|message| format!("\"{message}\""))
        .join(", ");
    assert_eq!(
        eval(
            "[[0, 10], [10, 0], [0, 0], [-10, 1], [-3, -4], [0.9, 5], [32768, 32768], \
              [20000, 20000]].map { |w, h| error { Bitmap.new(w, h) } }"
        ),
        format!("[{refused}]")
    );
}

/// Scenario "Arguments, measured on the original".
#[test]
fn the_arguments_of_a_new_bitmap_are_checked_as_measured() {
    assert_eq!(
        eval(
            "[error { Bitmap.new() }, error { Bitmap.new(1, 2, 3) }, error { Bitmap.new(1) }, \
              error { Bitmap.new(nil) }, error { Bitmap.new(\"x\", 1) }, \
              error { Bitmap.new(1, nil) }]"
        ),
        format!(
            "[{}]",
            [
                "\"ArgumentError: wrong number of arguments (0 for 1..2)\"",
                "\"ArgumentError: wrong number of arguments (3 for 1..2)\"",
                "\"TypeError: can't convert Fixnum into String\"",
                "\"TypeError: can't convert nil into String\"",
                "\"TypeError: can't convert String into Integer\"",
                "\"TypeError: no implicit conversion from nil to integer\"",
            ]
            .join(", ")
        )
    );
}

// --- Argument errors of Bitmap in RGSS3 -------------------------------------------------------

/// Scenario "Without arguments, measured on the original": each raises, and the process
/// goes on.
#[test]
fn a_method_without_arguments_raises_argument_error() {
    let methods = [
        ("blt", "(4 for 0)"),
        ("clear_rect", "(4 for 0)"),
        ("draw_text", "(5 for 0)"),
        ("fill_rect", "(5 for 0)"),
        ("gradient_fill_rect", "(6 for 0)"),
        ("set_pixel", "(0 for 3)"),
        ("stretch_blt", "(3 for 0)"),
        ("text_size", "(0 for 1)"),
    ];
    for (method, counts) in methods {
        let raised = eval(&format!("error {{ b.{method} }}"));
        assert!(
            raised.starts_with("\"ArgumentError: wrong number of arguments")
                && raised.contains(counts),
            "{method}: {raised}"
        );
    }
}

/// Scenario "Counts, measured on the original".
#[test]
fn a_wrong_count_names_the_expected_count_first() {
    assert_eq!(
        eval(
            "[error { b.fill_rect(1, 2, 3) }, error { b.fill_rect(1, 2, 3, 4) }, \
              error { b.fill_rect(1, 2, 3, 4, 5, 6) }, error { b.fill_rect(r, nil) }, \
              error { b.blt(0, 0, b) }, error { b.blt(0, 0, b, r, 255, 1) }, \
              error { b.stretch_blt(r, b) }, error { b.clear_rect(1, 2) }, \
              error { b.draw_text(1, 2, 3, 4) }, error { b.draw_text(r) }, \
              error { b.draw_text(1, 2, 3, 4, \"a\", 0, 0) }, error { b.draw_text(r, \"a\", 0, 0) }]"
        ),
        counts(&[
            "(5 for 3)",
            "(5 for 4)",
            "(5 for 6)",
            "(5 for 2)",
            "(4 for 3)",
            "(5 for 6)",
            "(3 for 2)",
            "(4 for 2)",
            "(5 for 4)",
            "(2 for 1)",
            "(6 for 7)",
            "(3 for 4)",
        ])
    );
}

/// Scenario "Types, measured on the original".
#[test]
fn an_argument_of_another_class_raises_type_error() {
    assert_eq!(
        eval(
            "[error { b.set_pixel(0, 0, nil) }, error { b.set_pixel(0, 0, Tone.new(1, 2, 3)) }, \
              error { b.fill_rect(0, 0, 1, 1, nil) }, error { b.blt(0, 0, b, nil) }, \
              error { b.blt(0, 0, \"x\", r) }, error { b.blt(0, 0, nil, r) }]"
        ),
        r#"["TypeError: can't convert NilClass into Color", "TypeError: can't convert Tone into Color", "TypeError: can't convert NilClass into Color", "TypeError: can't convert NilClass into Rect", "TypeError: can't convert String into Bitmap", "TypeError: can't convert NilClass into Bitmap"]"#
    );
}

/// Scenario "Fixed arity and conversions, measured on the original": Ruby's own messages.
#[test]
fn a_method_of_fixed_arity_has_the_message_of_ruby() {
    assert_eq!(
        eval(
            "[error { b.get_pixel(0) }, error { b.clear(1) }, error { b.hue_change }, \
              error { b.hue_change(nil) }, error { b.blt(0, 0, b, r, nil) }]"
        ),
        r#"["ArgumentError: wrong number of arguments(1 for 2)", "ArgumentError: wrong number of arguments(1 for 0)", "ArgumentError: wrong number of arguments(0 for 1)", "TypeError: no implicit conversion from nil to integer", "TypeError: no implicit conversion from nil to integer"]"#
    );
}
