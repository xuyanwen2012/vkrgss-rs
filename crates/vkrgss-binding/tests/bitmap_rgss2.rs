//! `Bitmap` and `Font` in a game of RGSS2: the scenarios of `rgss-bitmap` that are of that
//! runtime, as Ruby snippets. A binary of its own, because the classes are defined once per
//! process, as the runtime of the run has them.
//!
//! The harness's text backend draws every character as a cell half the font size wide and
//! the font size high, fully inked except for spaces: at size 20 that is 10 by 20 pixels.
//! One snippet is one evaluation; a snippet that changes the font defaults puts them back.

use vkrgss_binding::testing::{self, Harness};
use vkrgss_vfs::Runtime;

/// The harness, started as a game of RGSS2.
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss2)
}

/// `inspect` of the value of a snippet, or its exception as `Class: message`.
fn eval(source: &str) -> String {
    match harness().eval(source) {
        Ok(inspect) => inspect,
        Err(error) => format!("{}: {}", error.class_name, error.message),
    }
}

/// `px[bitmap]`: the pixels of a bitmap as rows of `[r, g, b, a]` integers. The names inside
/// are its own: the snippets of a binary share their local variables, and a block of Ruby
/// 1.8 assigns to a variable of that name outside it.
const PIXELS: &str = "px = lambda { |px_b| (0...px_b.height).map { |px_y| \
                      (0...px_b.width).map { |px_x| px_c = px_b.get_pixel(px_x, px_y); \
                      [px_c.red.to_i, px_c.green.to_i, px_c.blue.to_i, px_c.alpha.to_i] } } }\n";

// --- Bitmap methods of RGSS2 ------------------------------------------------------------------

#[test]
fn the_four_methods_return_nil_and_draw() {
    assert_eq!(
        eval(&format!(
            "{PIXELS}b = Bitmap.new(3, 3)
             b.fill_rect(b.rect, Color.new(90, 0, 0, 0))
             b.set_pixel(1, 1, Color.new(0, 0, 0, 255))
             r = [b.blur]
             centre = px[b][1][1]
             r << b.clear_rect(0, 0, 1, 1) << b.clear_rect(Rect.new(2, 2, 5, 5))
             r << b.radial_blur(0, 2)
             [r, centre, px[b][0][0], px[b][2][2], px[b][1][1] == centre]"
        )),
        "[[nil, nil, nil, nil], [80, 0, 0, 28], [0, 0, 0, 0], [0, 0, 0, 0], true]"
    );
    assert_eq!(
        eval(&format!(
            "{PIXELS}b = Bitmap.new(4, 1)
             r = b.gradient_fill_rect(0, 0, 4, 1, Color.new(0, 0, 0, 255), Color.new(255, 10, 0, 0))
             [r, px[b][0]]"
        )),
        "[nil, [[0, 0, 0, 255], [85, 3, 0, 170], [170, 6, 0, 85], [255, 10, 0, 0]]]"
    );
}

#[test]
fn the_arguments_are_integers_and_floats_are_truncated() {
    assert_eq!(
        eval(&format!(
            "{PIXELS}b = Bitmap.new(12, 12)
             b.fill_rect(b.rect, Color.new(1, 2, 3, 4))
             b.clear_rect(10.9, 10.9, 1.9, 1.9)
             kept = px[b].flatten.size / 4 - 1
             unchanged = px[b].inject(0) {{ |n, row| n + row.select {{ |pixel| pixel == [1, 2, 3, 4] }}.size }}
             a = Bitmap.new(12, 12); c = Bitmap.new(12, 12)
             12.times {{ |i| a.set_pixel(i, i, Color.new(i * 20, 9, 9, 255)); c.set_pixel(i, i, Color.new(i * 20, 9, 9, 255)) }}
             a.radial_blur(45.9, 4.9); c.radial_blur(45, 4)
             [px[b][10][10], unchanged == kept, px[a] == px[c], px[a] == px[Bitmap.new(12, 12)]]"
        )),
        "[[0, 0, 0, 0], true, true, false]"
    );
    assert_eq!(
        eval("Bitmap.new(4, 4).radial_blur(nil, 2)"),
        "TypeError: no implicit conversion from nil to integer"
    );
}

/// Scenario "Counts, measured on the original".
#[test]
fn wrong_counts_raise_argument_error() {
    let setup =
        "b = Bitmap.new(4, 4); r = Rect.new(0, 0, 4, 4); c1 = Color.new(1, 2, 3); c2 = c1\n";
    for (call, message) in [
        ("b.clear_rect(1, 2)", "wrong # of arguments (4 for 2)"),
        ("b.clear_rect(r, 1)", "wrong # of arguments (1 for 2)"),
        ("b.clear_rect(nil)", "wrong # of arguments (4 for 1)"),
        (
            "b.clear_rect(1, 2, 3, 4, 5)",
            "wrong # of arguments (4 for 5)",
        ),
        (
            "b.gradient_fill_rect(r, c1)",
            "wrong # of arguments (3 for 2)",
        ),
        (
            "b.gradient_fill_rect(0, 0, 4, 4, c1)",
            "wrong # of arguments (6 for 5)",
        ),
        (
            "b.gradient_fill_rect(r, c1, c2, true, 1)",
            "wrong # of arguments (4 for 5)",
        ),
        (
            "b.gradient_fill_rect(0, 0, 4, 4, c1, c2, true, 1)",
            "wrong # of arguments (7 for 8)",
        ),
        (
            "b.gradient_fill_rect(nil, c1, c2)",
            "wrong # of arguments (6 for 3)",
        ),
    ] {
        assert_eq!(
            eval(&format!("{setup}{call}")),
            format!("ArgumentError: {message}"),
            "{call}"
        );
    }
    for call in ["b.blur(1)", "b.radial_blur(1)", "b.radial_blur(1, 2, 3)"] {
        let outcome = eval(&format!("{setup}{call}"));
        assert!(outcome.starts_with("ArgumentError: "), "{call}: {outcome}");
    }
}

/// Scenario "No argument": the original ends with a page fault there.
#[test]
fn no_argument_raises_argument_error() {
    for call in ["clear_rect", "gradient_fill_rect"] {
        let outcome = eval(&format!("Bitmap.new(4, 4).{call}"));
        assert!(outcome.starts_with("ArgumentError: "), "{call}: {outcome}");
    }
}

/// Scenario "Seventh argument, measured on the original".
#[test]
fn a_seventh_argument_makes_the_gradient_vertical_whatever_it_is() {
    let direction = "dir = lambda { |dir_b| dir_a = dir_b.get_pixel(0, 0); \
                     [dir_b.get_pixel(3, 0) == dir_a, dir_b.get_pixel(0, 3) == dir_a] }\n\
                     c1 = Color.new(0, 0, 0, 255); c2 = Color.new(90, 90, 90, 255)\n";
    // [the top row has one colour, the left column has one colour]
    let (down, across) = ("[true, false]", "[false, true]");
    for (arguments, expected) in [
        ("0, 0, 4, 4, c1, c2, false", down),
        ("0, 0, 4, 4, c1, c2, nil", down),
        ("0, 0, 4, 4, c1, c2, true", down),
        ("0, 0, 4, 4, c1, c2", across),
        ("Rect.new(0, 0, 4, 4), c1, c2, false", across),
        ("Rect.new(0, 0, 4, 4), c1, c2, nil", across),
        ("Rect.new(0, 0, 4, 4), c1, c2", across),
        ("Rect.new(0, 0, 4, 4), c1, c2, true", down),
        ("Rect.new(0, 0, 4, 4), c1, c2, 0", down),
    ] {
        assert_eq!(
            eval(&format!(
                "{direction}b = Bitmap.new(4, 4); b.gradient_fill_rect({arguments}); dir[b]"
            )),
            expected,
            "{arguments}"
        );
    }
}

/// Scenario "Wrong colour, measured on the original".
#[test]
fn a_colour_of_another_class_raises_type_error() {
    let setup = "b = Bitmap.new(4, 4); r = Rect.new(0, 0, 4, 4); c2 = Color.new(1, 2, 3)\n";
    assert_eq!(
        eval(&format!("{setup}b.gradient_fill_rect(r, nil, c2)")),
        "TypeError: can't convert NilClass into Color"
    );
    assert_eq!(
        eval(&format!("{setup}b.gradient_fill_rect(0, 0, 4, 4, c2, 5)")),
        "TypeError: can't convert Fixnum into Color"
    );
    assert_eq!(
        eval(&format!(
            "{setup}b.gradient_fill_rect(r, Tone.new(0, 0, 0), c2)"
        )),
        "TypeError: can't convert Tone into Color"
    );
}

#[test]
fn a_disposed_bitmap_raises_rgss_error_before_the_arguments_are_looked_at() {
    for call in [
        "blur",
        "radial_blur(10, 2)",
        "radial_blur(nil, nil)",
        "clear_rect(0, 0, 1, 1)",
        "clear_rect(1, 2)",
        "gradient_fill_rect(Rect.new(0, 0, 1, 1), Color.new(0, 0, 0), Color.new(0, 0, 0))",
        "gradient_fill_rect(nil)",
        "text_size(5)",
        "draw_text(0, 0, 1, 1, 5)",
    ] {
        assert_eq!(
            eval(&format!("b = Bitmap.new(4, 4); b.dispose; b.{call}")),
            "RGSSError: disposed bitmap",
            "{call}"
        );
    }
}

// --- Font of RGSS2 ----------------------------------------------------------------------------

/// Scenario "Defaults, measured on the original".
#[test]
fn the_font_defaults_are_those_of_rgss2() {
    assert_eq!(
        eval("f = Font.new; [f.name, f.size, f.bold, f.italic, f.shadow]"),
        r#"[["Verdana", "Arial", "Courier New"], 20, false, false, true]"#
    );
    assert_eq!(
        eval(
            "c = Font.new.color; [Font.default_name, Font.default_size, Font.default_bold, \
             Font.default_italic, Font.default_shadow, [c.red, c.green, c.blue, c.alpha]]"
        ),
        r#"[["Verdana", "Arial", "Courier New"], 20, false, false, true, [255.0, 255.0, 255.0, 255.0]]"#
    );
    assert_eq!(
        eval("f = Bitmap.new(1, 1).font; [f.name, f.size, f.shadow]"),
        r#"[["Verdana", "Arial", "Courier New"], 20, true]"#
    );
    assert_eq!(
        eval("f = Font.new(\"Arial\", 30); [f.name, f.size, f.shadow]"),
        r#"["Arial", 30, true]"#
    );
}

#[test]
fn a_new_font_takes_the_default_shadow_of_the_moment() {
    assert_eq!(
        eval(
            "r = []
             begin
               Font.default_shadow = nil
               r << Font.default_shadow << Font.new.shadow << Bitmap.new(1, 1).font.shadow
               Font.default_shadow = 5
               r << Font.default_shadow << Font.new.shadow
             ensure
               Font.default_shadow = true
             end
             r"
        ),
        "[false, false, false, true, true]"
    );
}

/// Scenario "Truthiness, measured on the original".
#[test]
fn the_shadow_takes_any_object_and_reads_true_or_false() {
    assert_eq!(
        eval(
            "f = Font.new; r = []
             f.shadow = 0; r << f.shadow
             f.shadow = nil; r << f.shadow
             f.shadow = \"12\"; r << f.shadow
             f.shadow = false; r << f.shadow
             r << (f.shadow = Color.new(0, 0, 0)).class << f.shadow"
        ),
        "[true, false, true, false, Color, true]"
    );
}

/// Scenario "Assigning a font, measured on the original".
#[test]
fn assigning_a_font_copies_everything_but_the_shadow() {
    assert_eq!(
        eval(
            "b = Bitmap.new(1, 1); f = Font.new; f.shadow = false; f.size = 30; b.font = f; \
             [b.font.size, b.font.shadow]"
        ),
        "[30, true]"
    );
    // In the other direction, and what else is copied: a copy, not the object given.
    assert_eq!(
        eval(
            "b = Bitmap.new(1, 1); own = b.font; own.shadow = false
             f = Font.new(\"Arial\", 24); f.bold = true; f.italic = true; f.color = Color.new(1, 2, 3, 4)
             given = (b.font = f)
             f.size = 30; f.color.red = 99
             c = b.font.color
             [given.equal?(f), b.font.equal?(own), b.font.equal?(f), b.font.name, b.font.size, \
              b.font.bold, b.font.italic, [c.red, c.green, c.blue, c.alpha], b.font.shadow, \
              c.equal?(f.color)]"
        ),
        r#"[true, true, false, "Arial", 24, true, true, [1.0, 2.0, 3.0, 4.0], false, false]"#
    );
    assert_eq!(
        eval("Bitmap.new(1, 1).font = nil"),
        "TypeError: wrong argument type NilClass (expected Font)"
    );
}

#[test]
fn a_copy_of_a_bitmap_has_its_pixels_and_its_shadow() {
    assert_eq!(
        eval(
            "b = Bitmap.new(2, 2); b.set_pixel(1, 1, Color.new(5, 6, 7, 8))
             b.font.shadow = false; b.font.size = 37
             d = b.dup; c = b.clone
             b.font.shadow = true
             p = d.get_pixel(1, 1)
             [d.font.shadow, c.font.shadow, d.font.size, d.font.equal?(b.font), \
              [p.red, p.green, p.blue, p.alpha], Bitmap.new(1, 1).dup.font.shadow]"
        ),
        "[false, false, 37, false, [5.0, 6.0, 7.0, 8.0], true]"
    );
}

// --- Differences of Bitmap in RGSS2 -----------------------------------------------------------

/// Scenario "Undecodable file, measured on the original".
#[test]
fn a_file_that_is_no_picture_raises_rgss_error() {
    let empty = harness().game_dir().join("Graphics/Pictures/Empty.png");
    std::fs::write(&empty, b"").unwrap();
    for name in ["Broken", "Empty"] {
        assert_eq!(
            eval(&format!("Bitmap.new(\"Graphics/Pictures/{name}\")")),
            "RGSSError: failed to create bitmap",
            "{name}"
        );
    }
    assert_eq!(
        eval("b = Bitmap.new(\"Graphics/Pictures/Dot\"); [b.width, b.height, b.font.shadow]"),
        "[2, 2, true]"
    );
}

/// Scenario "Size limit, measured on the original".
#[test]
fn a_new_bitmap_has_at_most_4194304_pixels() {
    assert_eq!(
        eval("b = Bitmap.new(2048, 2048); r = [b.width, b.height]; b.dispose; r"),
        "[2048, 2048]"
    );
    assert_eq!(
        eval("Bitmap.new(2049, 2048)"),
        "RGSSError: failed to create bitmap"
    );
    assert_eq!(
        eval("b = Bitmap.new(65536, 64); r = b.width; b.dispose; r"),
        "65536"
    );
    assert_eq!(
        eval("Bitmap.new(640, 6554)"),
        "RGSSError: failed to create bitmap"
    );
    assert_eq!(
        eval("Bitmap.new(0, 10)"),
        "RGSSError: failed to create bitmap"
    );
    // The limit is on the size form: a copy of the largest bitmap works.
    assert_eq!(
        eval("b = Bitmap.new(2048, 2048); d = b.dup; r = d.height; b.dispose; d.dispose; r"),
        "2048"
    );
}

/// Scenario "Text of another class, measured on the original".
#[test]
fn a_text_of_any_class_is_drawn_as_its_to_s() {
    assert_eq!(
        eval(
            "b = Bitmap.new(1, 1)
             [b.text_size(123).width == b.text_size(\"123\").width, b.text_size(nil).width, \
              b.text_size(nil).to_s, b.text_size(:sym).width, b.text_size(1.5).width]"
        ),
        r#"[true, 0, "(0, 0, 0, 0)", 30, 30]"#
    );
    assert_eq!(
        eval(
            "inked = lambda { |ink_text| ink_b = Bitmap.new(60, 20); ink_b.font.shadow = false
               ink_r = ink_b.draw_text(0, 0, 60, 20, ink_text)
               [ink_r, (0...60).select { |ink_x| ink_b.get_pixel(ink_x, 10).alpha > 0 }.size] }
             [inked[123], inked[\"123\"], inked[nil], inked[:ab], \
              Bitmap.new(60, 20).draw_text(Rect.new(0, 0, 60, 20), Object.new, 1)]"
        ),
        "[[nil, 30], [nil, 30], [nil, 0], [nil, 20], nil]"
    );
}

// --- Text of RGSS2 through the class ----------------------------------------------------------

#[test]
fn text_is_drawn_with_a_shadow_by_default() {
    // One cell of 10 by 20 at the top left of a transparent bitmap: the text, then black
    // one pixel beyond its right and bottom edge, and nothing further. The first row and
    // column of the text have no shadow under them; elsewhere the text is blended onto it.
    assert_eq!(
        eval(&format!(
            "{PIXELS}b = Bitmap.new(14, 24); b.draw_text(0, 0, 10, 20, \"A\")
             p = px[b]
             plain = Bitmap.new(14, 24); plain.font.shadow = false; plain.draw_text(0, 0, 10, 20, \"A\")
             q = px[plain]
             [p[0][5][0, 3], p[5][5][3] >= q[5][5][3], p[5][10][0, 3], p[5][10][3] > 0, \
              p[20][5][0, 3], p[20][5][3] > 0, p[5][11], p[21][5], q[5][10], q[20][5], \
              b.text_size(\"A\").to_s == plain.text_size(\"A\").to_s]"
        )),
        "[[255, 255, 255], true, [0, 0, 0], true, [0, 0, 0], true, [0, 0, 0, 0], [0, 0, 0, 0], \
         [0, 0, 0, 0], [0, 0, 0, 0], true]"
    );
}

#[test]
fn text_is_squeezed_to_a_fifth_step() {
    // Ten characters of advance 10 at size 20. The steps are 90, 80, 70, 60 and 50 wide;
    // below the last the text starts at the left edge and is cut off.
    assert_eq!(
        eval(
            "width = lambda { |sq_w| sq_b = Bitmap.new(120, 20); sq_b.font.shadow = false
               sq_b.draw_text(0, 0, sq_w, 20, \"ABCDEFGHIJ\")
               (0...120).select { |sq_x| sq_b.get_pixel(sq_x, 10).alpha > 0 }.size }
             [100, 99, 89, 79, 69, 59, 55, 50, 49].map { |sq_w| width[sq_w] }"
        ),
        "[100, 90, 80, 70, 60, 50, 50, 50, 49]"
    );
}

/// Scenario "VX" of "Font and Bitmap of RGSS1 and RGSS2 are unchanged by RGSS3": a game of
/// RGSS2 has no outline and no out colour, drops the fraction of a size and keeps its
/// default names.
#[test]
fn the_font_of_rgss3_does_not_reach_rgss2() {
    assert_eq!(
        eval(
            "[Font.new.respond_to?(:outline), Font.respond_to?(:default_out_color), \
             (f = Font.new; f.size = 24.7; f.size), Font.default_name]"
        ),
        r#"[false, false, 24, ["Verdana", "Arial", "Courier New"]]"#
    );
}
