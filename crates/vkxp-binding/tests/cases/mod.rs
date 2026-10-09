//! The Ruby test snippets of the binding and what each evaluates to.
//!
//! `tests/ruby.rs` evaluates them as they are; `tests/stress.rs` evaluates the same snippets
//! with a garbage collection at every allocation and expects the same results.
//!
//! One snippet is one evaluation, and evaluations do not interleave, so a snippet that
//! changes shared state such as the font defaults puts it back before it ends.

#![allow(dead_code)]

use vkxp_binding::testing;

/// What a snippet evaluates to.
#[derive(Debug, Clone, Copy)]
pub enum Expect {
    /// `inspect` of its value.
    Value(&'static str),
    /// It raises an exception of this class.
    Raises(&'static str),
    /// It raises an exception of this class with this message.
    RaisesWith(&'static str, &'static str),
}

/// A snippet and its expected outcome.
#[derive(Debug, Clone, Copy)]
pub struct Case {
    pub name: &'static str,
    pub source: &'static str,
    pub expect: Expect,
}

const fn value(name: &'static str, source: &'static str, inspect: &'static str) -> Case {
    Case {
        name,
        source,
        expect: Expect::Value(inspect),
    }
}

const fn raises(name: &'static str, source: &'static str, class: &'static str) -> Case {
    Case {
        name,
        source,
        expect: Expect::Raises(class),
    }
}

const fn raises_with(
    name: &'static str,
    source: &'static str,
    class: &'static str,
    message: &'static str,
) -> Case {
    Case {
        name,
        source,
        expect: Expect::RaisesWith(class, message),
    }
}

/// Evaluates each case's source after `wrap` has had its way with it and returns what is
/// wrong, one entry per failed case.
pub fn failures(cases: &[Case], wrap: impl Fn(&str) -> String) -> Vec<String> {
    cases
        .iter()
        .filter_map(|case| {
            let outcome = testing::eval(&wrap(case.source));
            let as_expected = match (&outcome, case.expect) {
                (Ok(inspect), Expect::Value(expected)) => inspect == expected,
                (Err(error), Expect::Raises(class)) => error.class_name == class,
                (Err(error), Expect::RaisesWith(class, message)) => {
                    error.class_name == class && error.message == message
                }
                _ => false,
            };
            (!as_expected).then(|| {
                let got = match outcome {
                    Ok(inspect) => format!("the value {inspect}"),
                    Err(error) => format!("{}: {}", error.class_name, error.message),
                };
                format!("{}: expected {:?}, got {got}", case.name, case.expect)
            })
        })
        .collect()
}

/// Every group of cases, by name.
pub const GROUPS: &[(&str, &[Case])] = &[
    ("value method surface", VALUE_SURFACE),
    ("color and tone", COLOR_AND_TONE),
    ("rect", RECT),
    ("table", TABLE),
    ("marshal", MARSHAL),
    ("font", FONT),
    ("copies of values", VALUE_COPIES),
];

// --- rgss-values: Method surface of the value classes -----------------------------------------

pub const VALUE_SURFACE: &[Case] = &[
    raises_with(
        "wrong number of arguments",
        "Rect.new(0, 0, 1, 1).send(:x=, 1, 2)",
        "ArgumentError",
        "wrong number of arguments (2 for 1)",
    ),
    value(
        "the classes are plain classes below Object",
        "[Color, Tone, Rect, Table, Font].map { |c| [c.class, c.superclass] }.uniq",
        "[[Class, Object]]",
    ),
    value(
        "a subclass defined by a script works",
        "class CaseColor < Color; def twice; red * 2; end; end; \
         c = CaseColor.new(10, 0, 0); [c.twice, c.class, c == Color.new(10, 0, 0)]",
        "[20.0, CaseColor, true]",
    ),
];

// --- rgss-values: Color and Tone --------------------------------------------------------------

pub const COLOR_AND_TONE: &[Case] = &[
    value(
        "construction and clamping",
        "c = Color.new(300, -5, 12.5); [c.red, c.green, c.blue, c.alpha]",
        "[255.0, 0.0, 12.5, 255.0]",
    ),
    value(
        "setters",
        "t = Tone.new(0, 0, 0); t.gray = 300; t.set(-300, 1, 2); [t.red, t.gray]",
        "[-255.0, 0.0]",
    ),
    value(
        "equality and printing",
        "[Color.new(1, 2, 3) == Color.new(1, 2, 3, 255), Color.new(1, 2, 3).to_s]",
        r#"[true, "(1.000000, 2.000000, 3.000000, 255.000000)"]"#,
    ),
    raises("wrong argument", r#"Color.new("red", 0, 0)"#, "TypeError"),
    value(
        "every component reads and writes by name",
        "c = Color.new(1, 2, 3, 4); c.red = 10; c.green = 20.5; c.blue = 30; c.alpha = 300; \
         t = Tone.new(1, 2, 3, 4); t.red = -10; t.green = -300; t.blue = 30; t.gray = 300; \
         [c.red, c.green, c.blue, c.alpha, t.red, t.green, t.blue, t.gray]",
        "[10.0, 20.5, 30.0, 255.0, -10.0, -255.0, 30.0, 255.0]",
    ),
    value(
        "tone defaults and printing",
        "t = Tone.new(-1, 2, 3.5); [t.gray, t.to_s, Tone.new(1, 2, 3, 4).gray]",
        r#"[0.0, "(-1.000000, 2.000000, 3.500000, 0.000000)", 4.0]"#,
    ),
    value(
        "set with four values and its result",
        "c = Color.new(0, 0, 0); r = c.set(1, 2, 3, 4); [r.equal?(c), c.to_s, c.set(9, 9, 9).alpha]",
        r#"[true, "(1.000000, 2.000000, 3.000000, 4.000000)", 255.0]"#,
    ),
    value(
        "the three comparisons agree",
        "a = Color.new(1, 2, 3); b = Color.new(1, 2, 3); c = Color.new(1, 2, 4); \
         [a == b, a === b, a.eql?(b), a == c, a === c, a.eql?(c), a != c, a.equal?(b)]",
        "[true, true, true, false, false, false, true, false]",
    ),
    value(
        "other classes are not equal",
        "[Color.new(0, 0, 0, 0) == Tone.new(0, 0, 0, 0), Color.new(0, 0, 0) == nil, \
          Tone.new(0, 0, 0) == 0, Tone.new(1, 2, 3) == Tone.new(1, 2, 3, 0)]",
        "[false, false, false, true]",
    ),
    raises_with(
        "too few arguments",
        "Color.new(1, 2)",
        "ArgumentError",
        "wrong number of arguments (2 for 3)",
    ),
    raises_with(
        "too many arguments",
        "Tone.new(1, 2, 3, 4, 5)",
        "ArgumentError",
        "wrong number of arguments (5 for 4)",
    ),
    raises(
        "a setter rejects what is not a number",
        "Tone.new(0, 0, 0).gray = nil",
        "TypeError",
    ),
    raises(
        "set rejects what is not a number",
        "Color.new(0, 0, 0).set(1, 2, :three)",
        "TypeError",
    ),
];

// --- rgss-values: Rect ------------------------------------------------------------------------

pub const RECT: &[Case] = &[
    raises(
        "four arguments required",
        "Rect.new(1, 2, 3)",
        "ArgumentError",
    ),
    value(
        "values and printing",
        "r = Rect.new(-1, 2, 30, 40); r.width = 5; r.to_s",
        r#""(-1, 2, 5, 40)""#,
    ),
    value(
        "empty",
        "r = Rect.new(1, 2, 3, 4); r.empty; r == Rect.new(0, 0, 0, 0)",
        "true",
    ),
    value(
        "every value reads and writes by name",
        "r = Rect.new(1, 2, 3, 4); r.x = -10; r.y = 20; r.width = -30; r.height = 40; \
         [r.x, r.y, r.width, r.height]",
        "[-10, 20, -30, 40]",
    ),
    value(
        "set and its result",
        "r = Rect.new(0, 0, 0, 0); s = r.set(5, 6, 7, 8); [s.equal?(r), r.to_s, r.empty.equal?(r)]",
        r#"[true, "(5, 6, 7, 8)", true]"#,
    ),
    value(
        "the three comparisons agree",
        "a = Rect.new(1, 2, 3, 4); b = Rect.new(1, 2, 3, 4); c = Rect.new(1, 2, 3, 5); \
         [a == b, a === b, a.eql?(b), a == c, a === c, a.eql?(c), a == nil, a == [1, 2, 3, 4]]",
        "[true, true, true, false, false, false, false, false]",
    ),
    value(
        "a float is truncated",
        "Rect.new(1.9, -1.9, 2.5, 0.1).to_s",
        r#""(1, -1, 2, 0)""#,
    ),
    raises(
        "a value that is not a number",
        r#"Rect.new(0, 0, "1", 1)"#,
        "TypeError",
    ),
    raises(
        "set takes four",
        "Rect.new(0, 0, 1, 1).set(1, 2, 3)",
        "ArgumentError",
    ),
];

// --- rgss-values: Table -----------------------------------------------------------------------

pub const TABLE: &[Case] = &[
    value(
        "read and write",
        "t = Table.new(4, 3); t[1, 2] = 7; [t[1, 2], t[0, 0], t[4, 0]]",
        "[7, 0, nil]",
    ),
    value(
        "sizes",
        "t = Table.new(17); [t.xsize, t.ysize, t.zsize]",
        "[17, 1, 1]",
    ),
    value(
        "resize keeps contents",
        "t = Table.new(2, 2); t[1, 1] = 9; t.resize(3, 3); [t[1, 1], t[2, 2]]",
        "[9, 0]",
    ),
    value(
        "three dimensions",
        "t = Table.new(2, 3, 4); t[1, 2, 3] = -5; t[0, 0, 0] = 1; \
         [t.xsize, t.ysize, t.zsize, t[1, 2, 3], t[0, 0, 0], t[1, 2, 4], t[-1, 0, 0]]",
        "[2, 3, 4, -5, 1, nil, nil]",
    ),
    value(
        "a write outside the bounds does nothing",
        "t = Table.new(2); t[2] = 5; t[-1] = 5; [t[0], t[1], t[2]]",
        "[0, 0, nil]",
    ),
    value(
        "resize to other dimensions",
        "t = Table.new(3); t[2] = 4; t.resize(4, 2); a = [t.xsize, t.ysize, t[2, 0], t[3, 1]]; \
         t.resize(1); a + [t.xsize, t.ysize, t[0], t[2]]",
        "[4, 2, 4, 0, 1, 1, 0, nil]",
    ),
    raises_with(
        "too few indices",
        "Table.new(2, 2)[1]",
        "ArgumentError",
        "wrong number of arguments (1 for 2)",
    ),
    raises_with(
        "too many indices in a write",
        "Table.new(2)[0, 0] = 1; nil",
        "ArgumentError",
        "wrong number of arguments (3 for 2)",
    ),
    raises("no size", "Table.new", "ArgumentError"),
    raises("four sizes", "Table.new(1, 1, 1, 1)", "ArgumentError"),
    raises(
        "an index that is not a number",
        r#"Table.new(2)["0"]"#,
        "TypeError",
    ),
];

// --- rgss-values: Marshal forms ---------------------------------------------------------------

pub const MARSHAL: &[Case] = &[
    value(
        "round trip through Marshal",
        "a = [Color.new(1, 2, 3, 4), Tone.new(-1, 2, 3, 4), Rect.new(1, 2, 3, 4)]; \
         b = Marshal.load(Marshal.dump(a)); \
         [b.size, b[0] == a[0], b[1] == a[1], b[2] == a[2], b.map { |o| o.class }]",
        "[3, true, true, true, [Color, Tone, Rect]]",
    ),
    // A user-defined object is `u`, the class name as a symbol, the payload length and the
    // payload; 1820 is written as the two bytes 0x1c 0x07 after a 2.
    value(
        "table bytes",
        r#"d = Marshal.dump(Table.new(20, 15, 3)); [d[0, 10] == "\004\010u:\012Table", d[10, 3] == "\002\034\007", d.size - 13, d[13, 20].unpack("V5")]"#,
        "[true, true, 1820, [3, 20, 15, 3, 900]]",
    ),
    // 20 bytes of header and 1200 of elements: 1220 is 0xc4 0x04.
    value(
        "data written by the original runtime",
        r#"payload = [2, 6, 100, 1, 600].pack("V5") + (0...600).to_a.pack("v*"); t = Marshal.load("\004\010u:\012Table\002\304\004" + payload); [t.class, t.xsize, t.ysize, t.zsize, t[0, 0], t[5, 99], t[1, 2]]"#,
        "[Table, 6, 100, 1, 0, 599, 13]",
    ),
    // A length of 31 is written as the byte 31 + 5.
    raises(
        "malformed payload",
        r#"Marshal.load("\004\010u:\012Color\044" + "\0" * 31)"#,
        "ArgumentError",
    ),
    value(
        "byte layouts of colour, tone and rectangle",
        r#"[Color.new(1, 2, 3, 4)._dump(0) == [1.0, 2.0, 3.0, 4.0].pack("E4"), Tone.new(-1, 2, 3, 4)._dump(0) == [-1.0, 2.0, 3.0, 4.0].pack("E4"), Rect.new(1, -2, 3, 4)._dump(0) == [1, -2, 3, 4].pack("l4")]"#,
        "[true, true, true]",
    ),
    value(
        "load from bytes",
        r#"[Color._load([5.5, 6.0, 7.0, 8.0].pack("E4")).to_s, Tone._load([-5.5, 6.0, 7.0, 8.0].pack("E4")).to_s, Rect._load([9, -8, 7, 6].pack("l4")).to_s]"#,
        r#"["(5.500000, 6.000000, 7.000000, 8.000000)", "(-5.500000, 6.000000, 7.000000, 8.000000)", "(9, -8, 7, 6)"]"#,
    ),
    value(
        "a table round trips with its contents",
        "t = Table.new(3, 2); t[2, 1] = 77; t[0, 0] = -1; u = Marshal.load(Marshal.dump(t)); \
         [u.xsize, u.ysize, u.zsize, u[2, 1], u[0, 0], u[1, 1]]",
        "[3, 2, 1, 77, -1, 0]",
    ),
    raises(
        "a truncated tone",
        r#"Tone._load("x" * 33)"#,
        "ArgumentError",
    ),
    raises(
        "a truncated rectangle",
        r#"Rect._load("")"#,
        "ArgumentError",
    ),
    raises(
        "a table whose header promises more",
        r#"Table._load([1, 5, 1, 1, 5].pack("V5") + "\0" * 8)"#,
        "ArgumentError",
    ),
    raises(
        "a payload that is not a string",
        "Color._load(31)",
        "TypeError",
    ),
];

// --- rgss-values: Font ------------------------------------------------------------------------

pub const FONT: &[Case] = &[
    value(
        "defaults are copied",
        "Font.default_size = 30; f = Font.new; Font.default_size = 22; f.size",
        "30",
    ),
    value(
        "name as an array",
        r#"old = Font.default_name; Font.default_name = ["NoSuchFont", "Liberation Sans"]; f = Font.new; Font.default_name = old; f.name"#,
        r#"["NoSuchFont", "Liberation Sans"]"#,
    ),
    value(
        "colour is the font's own",
        "f = Font.new; f.color.red = 0; [f.color.red, Font.default_color.red]",
        "[0.0, 255.0]",
    ),
    value("existence", r#"Font.exist?("NoSuchFont")"#, "false"),
    value(
        "a family the backend has exists",
        r#"[Font.exist?("Liberation Sans"), Font.exist?("Test Sans")]"#,
        "[true, true]",
    ),
    value(
        "the initial defaults",
        "[Font.default_name, Font.default_size, Font.default_bold, Font.default_italic, \
          Font.default_color.to_s]",
        r#"["Test Sans", 22, false, false, "(255.000000, 255.000000, 255.000000, 255.000000)"]"#,
    ),
    value(
        "a new font has the defaults",
        "f = Font.new; [f.name, f.size, f.bold, f.italic, f.color == Font.default_color, \
          f.color.equal?(Font.default_color)]",
        r#"["Test Sans", 22, false, false, true, false]"#,
    ),
    value(
        "name and size given",
        r#"f = Font.new("Arial", 12); g = Font.new(["A", "B"]); [f.name, f.size, g.name, g.size]"#,
        r#"["Arial", 12, ["A", "B"], 22]"#,
    ),
    value(
        "attributes read and write",
        r#"f = Font.new; f.name = ["X", "Y"]; f.size = 40; f.bold = true; f.italic = 1; a = [f.name, f.size, f.bold, f.italic]; f.name = "Z"; f.bold = nil; a + [f.name, f.bold]"#,
        r#"[["X", "Y"], 40, true, true, "Z", false]"#,
    ),
    value(
        "assigning a colour copies its value",
        "f = Font.new; own = f.color; c = Color.new(1, 2, 3); f.color = c; c.red = 9; \
         [f.color.equal?(own), f.color.to_s]",
        r#"[true, "(1.000000, 2.000000, 3.000000, 255.000000)"]"#,
    ),
    value(
        "every default is copied by new fonts",
        r#"begin; Font.default_name = "D"; Font.default_bold = true; Font.default_italic = true; Font.default_color = Color.new(1, 2, 3, 4); f = Font.new; [f.name, f.bold, f.italic, f.color.to_s, Font.default_bold, Font.default_italic]; ensure; Font.default_name = "Test Sans"; Font.default_bold = false; Font.default_italic = false; Font.default_color = Color.new(255, 255, 255); end"#,
        r#"["D", true, true, "(1.000000, 2.000000, 3.000000, 4.000000)", true, true]"#,
    ),
    value(
        "the default colour object is live",
        "begin; Font.default_color.green = 7; Font.new.color.green; \
         ensure; Font.default_color.green = 255; end",
        "7.0",
    ),
    raises("a name that is not a string", "Font.new(1)", "TypeError"),
    raises(
        "a name array with something else",
        r#"Font.new(["A", 2])"#,
        "TypeError",
    ),
    raises("three arguments", r#"Font.new("A", 1, 2)"#, "ArgumentError"),
    raises(
        "a colour that is not a Color",
        "Font.new.color = Tone.new(0, 0, 0)",
        "TypeError",
    ),
    raises(
        "a default name that is not a string",
        "Font.default_name = nil",
        "TypeError",
    ),
];

// --- rgss-values: Copies are independent ------------------------------------------------------

pub const VALUE_COPIES: &[Case] = &[
    value(
        "cloned table",
        "a = Table.new(2); b = a.clone; b[0] = 5; a[0]",
        "0",
    ),
    value(
        "table copies keep contents and sizes",
        "a = Table.new(2, 2); a[1, 1] = 3; b = a.dup; a[1, 1] = 4; a.resize(1); \
         [b.xsize, b.ysize, b[1, 1], b.class]",
        "[2, 2, 3, Table]",
    ),
    value(
        "colour",
        "a = Color.new(1, 2, 3, 4); b = a.clone; c = a.dup; e = [b == a, c == a, b.equal?(a)]; \
         b.red = 9; c.alpha = 0; e + [a.to_s]",
        r#"[true, true, false, "(1.000000, 2.000000, 3.000000, 4.000000)"]"#,
    ),
    value(
        "tone",
        "a = Tone.new(1, 2, 3, 4); b = a.clone; c = a.dup; e = [b == a, c == a]; \
         b.red = 9; c.gray = 0; e + [a.to_s]",
        r#"[true, true, "(1.000000, 2.000000, 3.000000, 4.000000)"]"#,
    ),
    value(
        "rectangle",
        "a = Rect.new(1, 2, 3, 4); b = a.clone; c = a.dup; e = [b == a, c == a]; \
         b.x = 9; c.empty; e + [a.to_s]",
        r#"[true, true, "(1, 2, 3, 4)"]"#,
    ),
    value(
        "font",
        r#"a = Font.new(["A", "B"], 10); a.bold = true; a.color = Color.new(1, 2, 3); b = a.clone; c = a.dup; e = [b.name, b.size, b.bold, b.italic, b.color == a.color, c.name, c.color == a.color, b.color.equal?(a.color), c.color.equal?(a.color)]; b.size = 20; b.name = "C"; b.color.red = 99; c.bold = false; c.color = Color.new(0, 0, 0); e + [a.name, a.size, a.bold, a.color.to_s]"#,
        r#"[["A", "B"], 10, true, false, true, ["A", "B"], true, false, false, ["A", "B"], 10, true, "(1.000000, 2.000000, 3.000000, 255.000000)"]"#,
    ),
    raises(
        "a copy from another class",
        "Color.new(0, 0, 0).send(:initialize_copy, Tone.new(0, 0, 0))",
        "TypeError",
    ),
];
