## ADDED Requirements

### Requirement: Font of RGSS3
In RGSS3 the font defaults SHALL be the String `"VL Gothic"`, size 24, bold, italic and
shadow off, outline on, the colour white and the out colour (0, 0, 0, 128), and `Font` SHALL
have `outline`, `out_color`, `Font.default_outline` and `Font.default_out_color` with their
writers. A new font and a new bitmap's font SHALL take the defaults of the moment they are
made. In RGSS1 and RGSS2 `Font` SHALL stay as it is.

#### Scenario: Defaults, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Font.default_name, Font.default_size, Font.default_bold, Font.default_italic, Font.default_shadow, Font.default_outline, Font.default_color.to_s, Font.default_out_color.to_s]`
- **THEN** the result is `["VL Gothic", 24, false, false, false, true, "(255.000000, 255.000000, 255.000000, 255.000000)", "(0.000000, 0.000000, 0.000000, 128.000000)"]`

#### Scenario: A new font, measured on the original
- **WHEN** a game of RGSS3 evaluates `f = Font.new; [f.name, f.name.encoding.name, f.size, f.bold, f.italic, f.shadow, f.outline, f.color.to_s, f.out_color.to_s]`
- **THEN** the result is `["VL Gothic", "ASCII-8BIT", 24, false, false, false, true, "(255.000000, 255.000000, 255.000000, 255.000000)", "(0.000000, 0.000000, 0.000000, 128.000000)"]`

#### Scenario: A new bitmap after the defaults changed, measured on the original
- **WHEN** a game of RGSS3 evaluates `Font.default_size = 31; Font.default_outline = false; Font.default_out_color = Color.new(1, 1, 1, 1); f = Bitmap.new(4, 4).font; [f.size, f.outline, f.out_color.to_s]` and puts the defaults back
- **THEN** the result is `[31, false, "(1.000000, 1.000000, 1.000000, 1.000000)"]`

### Requirement: Font sizes of RGSS3
In RGSS3 `Font#size=`, `Font.default_size=` and the size of `Font.new` SHALL convert the
value as an Integer, truncating a Float, SHALL raise `ArgumentError` with the message `bad
value for size` (`bad value for default_size` for the default) when that integer is outside
6 to 96, and otherwise SHALL keep the number given for the reader while text is measured
and drawn with the integer.

#### Scenario: Fractions, measured on the original
- **WHEN** a game of RGSS3 evaluates `f = Font.new; [(f.size = 24.7), f.size, (f.size = 96.99), f.size, (f.size = 6), f.size]` and `Font.default_size = 20.5; Font.default_size`
- **THEN** the results are `[24.7, 24.7, 96.99, 96.99, 6, 6]` and `20.5`

#### Scenario: Limits, measured on the original
- **WHEN** a game of RGSS3 evaluates `f.size = 5`, `f.size = 5.9`, `f.size = 97`, `f.size = 0`, `f.size = 1000`, `Font.new("Arial", 5)`, `Font.new("Arial", 97)`, `Font.default_size = 5` and `Font.default_size = 97`
- **THEN** each raises `ArgumentError`, with the message `bad value for default_size` for the last two and `bad value for size` for the others, and the sizes are unchanged

#### Scenario: Wrong kinds, measured on the original
- **WHEN** a game of RGSS3 evaluates `f.size = nil`, `f.size = "x"` and `Font.new("Arial", nil)`
- **THEN** each raises `TypeError`, with the messages `no implicit conversion from nil to integer`, `can't convert String into Integer` and `no implicit conversion from nil to integer`

#### Scenario: Drawn with the integer, measured on the original
- **WHEN** a game of RGSS3 evaluates `b = Bitmap.new(200, 60); b.font.size = 24.7; a = b.text_size("Hello").to_s; b.font.size = 24; a == b.text_size("Hello").to_s`
- **THEN** the result is `true`

### Requirement: Font names of RGSS3
In RGSS3 `Font#name=`, `Font.default_name=` and the name of `Font.new` SHALL keep the object
given for the reader. The font SHALL be drawn with the first element of an Array that exists
exactly, else its first element, else the object; by its `to_s`, cut to 31 characters. An
empty Array or an empty name SHALL draw nothing. `Font.exist?` SHALL be exact and
case-sensitive, a Symbol taken by its name and every other non-String false.

#### Scenario: The object is kept, measured on the original
- **WHEN** a game of RGSS3 evaluates `a = ["Arial", "Tahoma", "x"]; f = Font.new; f.name = a; [f.name.equal?(a), (f.name = nil), f.name, (f.name = 1), f.name, Font.new(nil).name, Font.new(1).name, Font.new(["Nope", "Arial"]).name]`
- **THEN** the result is `[true, nil, nil, 1, 1, nil, 1, ["Nope", "Arial"]]`

#### Scenario: The default name, measured on the original
- **WHEN** a game of RGSS3 evaluates `Font.default_name = ["Nope", "Arial"]; [Font.default_name, Font.new.name]` and puts the default back
- **THEN** the result is `[["Nope", "Arial"], ["Nope", "Arial"]]`

#### Scenario: The name drawn with
- **WHEN** the name of a font of RGSS3 is chosen, with "Test Sans" and "Liberation Sans" existing, from `["Nope", "Test Sans", "Liberation Sans"]`, `["Nope", "Other"]`, `[1, "Test Sans"]`, `:"Test Sans"`, `"test sans"`, `[]`, `""`, `nil` and a String of "Test Sans" followed by 30 spaces
- **THEN** the names are "Test Sans", "Nope", "Test Sans", "Test Sans", "test sans", none, none, none and "Test Sans" followed by 22 spaces

#### Scenario: Exact existence
- **WHEN** a game of RGSS3 whose text backend has the families "Test Sans" and "Liberation Sans" evaluates `Font.exist?` of `"Test Sans"`, `:"Test Sans"`, `"test sans"`, `"TEST SANS"`, `"Test Sans Regular"`, `""`, `" "`, `"Nope"`, `nil`, `1` and `["Test Sans"]`
- **THEN** the results are `true`, `true` and nine times `false`, and nothing is raised

#### Scenario: No name, measured on the original
- **WHEN** a game of RGSS3 sets the font name of a transparent bitmap to `""`, and of another to `[]`, and draws `"x"` into each
- **THEN** both bitmaps are unchanged and `draw_text` returns `nil`

#### Scenario: The size of no name
- **WHEN** a game of RGSS3 evaluates `text_size("x")` on a bitmap whose font name is `""`
- **THEN** the result is a `Rect` (0, 0, 0, 0), where the original returns uninitialised numbers

### Requirement: Font flags, colours and copies of RGSS3
In RGSS3 `bold=`, `italic=`, `shadow=` and `outline=` SHALL read false for `nil` and `false`
and true otherwise; the four flag writers of the class SHALL keep the object given;
`color=` and `out_color=` SHALL copy a `Color` into the font's own object;
`default_color=` and `default_out_color=` SHALL keep the `Color` given. Another class SHALL
raise `TypeError`. `Font.new` SHALL take at most two arguments; `dup` and `clone` SHALL copy
every attribute.

#### Scenario: Flags, measured on the original
- **WHEN** a game of RGSS3 evaluates, for each of `nil`, `false`, `true`, `0`, `1` and `"x"` as `v`, `f.outline = v; f.outline` and `Font.default_outline = v; [Font.default_outline, Font.new.outline]`, and the same for `shadow`, `bold` and `italic`
- **THEN** the reader of the font is false for `nil` and `false` and true for the others, and the default reads `v` itself while a font made then reads its truth

#### Scenario: Colours are copied into the font, measured on the original
- **WHEN** a game of RGSS3 evaluates `f = Font.new; c = Color.new(9, 8, 7); f.out_color = c; f.color = c; [f.out_color.equal?(c), f.color.equal?(c), f.out_color.equal?(f.out_color), f.out_color.to_s]`
- **THEN** the result is `[false, false, true, "(9.000000, 8.000000, 7.000000, 255.000000)"]`

#### Scenario: Default colours are kept, measured on the original
- **WHEN** a game of RGSS3 evaluates `c = Color.new(100, 8, 7); Font.default_out_color = c; f = Font.new; c.red = 50; [Font.default_out_color.equal?(c), f.out_color.equal?(c), f.out_color.red, Font.new.out_color.red]`, the same with `default_color` and `color`, and puts the defaults back
- **THEN** both results are `[true, false, 100.0, 50.0]`

#### Scenario: Wrong colours, measured on the original
- **WHEN** a game of RGSS3 evaluates `f.color = nil`, `f.out_color = nil`, `f.out_color = Tone.new(1, 2, 3)`, `Font.default_color = nil` and `Font.default_out_color = nil`
- **THEN** each raises `TypeError`, with the messages `can't convert NilClass into Color`, `can't convert NilClass into Color`, `can't convert Tone into Color`, `can't convert NilClass into Color` and `can't convert NilClass into Color`

#### Scenario: Arguments of Font.new, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Font.new("Arial").name, Font.new("Arial", 30).size]` and `Font.new("Arial", 30, 1)`
- **THEN** the first gives `["Arial", 30]` and the second raises `ArgumentError` with the message `wrong number of arguments (3 for 0..2)`

#### Scenario: Copies and comparison, measured on the original
- **WHEN** a game of RGSS3 evaluates `f = Font.new("Arial"); f.outline = false; f.out_color = Color.new(1, 2, 3, 4); g = f.dup; h = f.clone; [g.outline, g.out_color.to_s, g.color.equal?(f.color), g.out_color.equal?(f.out_color), h.out_color.to_s, Font.new == Font.new, f == f]`
- **THEN** the result is `[false, "(1.000000, 2.000000, 3.000000, 4.000000)", false, false, "(1.000000, 2.000000, 3.000000, 4.000000)", false, true]`

#### Scenario: No dump, measured on the original
- **WHEN** a game of RGSS3 evaluates `Marshal.dump(Font.new)`
- **THEN** `TypeError` is raised with the message `no marshal_dump is defined for class Font`

### Requirement: Bitmap font and copies of RGSS3
In RGSS3 `Bitmap#font=` SHALL take a `Font` only and SHALL copy name, size, bold, italic,
shadow, outline, colour and out colour into the bitmap's own `Font` object, whose colour
objects stay its own; later changes of the font given SHALL not reach the bitmap. `dup` and
`clone` of a bitmap SHALL give the copy a font of its own with all eight attributes.

#### Scenario: Everything is copied, measured on the original
- **WHEN** a game of RGSS3 evaluates `f = Font.new("Arial", 30); f.bold = f.italic = f.shadow = true; f.outline = false; f.color = Color.new(1, 2, 3, 4); f.out_color = Color.new(5, 6, 7, 8); b = Bitmap.new(4, 4); b.font = f; f.size = 40; g = b.font; [g.name, g.size, g.bold, g.italic, g.shadow, g.outline, g.color.to_s, g.out_color.to_s, g.equal?(f), g.color.equal?(f.color), g.out_color.equal?(f.out_color)]`
- **THEN** the result is `["Arial", 30, true, true, true, false, "(1.000000, 2.000000, 3.000000, 4.000000)", "(5.000000, 6.000000, 7.000000, 8.000000)", false, false, false]`

#### Scenario: Not a font, measured on the original
- **WHEN** a game of RGSS3 evaluates `b.font = nil` and `b.font = "Arial"`
- **THEN** each raises `TypeError`, with the messages `can't convert NilClass into Font` and `can't convert String into Font`

#### Scenario: Copies of a bitmap, measured on the original
- **WHEN** a game of RGSS3 evaluates `b = Bitmap.new(4, 4); b.font.size = 40; b.font.shadow = true; b.font.out_color.red = 99; c = b.dup; d = b.clone; c.font.size = 10; [[c.font.size, c.font.shadow, c.font.out_color.red], [d.font.size, d.font.shadow, d.font.out_color.red], b.font.size, c.font.equal?(b.font)]`
- **THEN** the result is `[[10, true, 99.0], [40, true, 99.0], 40, false]`

### Requirement: Text of a bitmap of RGSS3
In RGSS3 `Bitmap#draw_text` SHALL draw by the text rules of RGSS3 of the core (the work
rectangle, the one-step squeeze, the outline, the shadow off by default) and `text_size`
SHALL return the size the core measures, without outline or shadow. The text SHALL be the
`to_s` of any object, `align` an Integer by conversion, and `draw_text` SHALL return `nil`.

#### Scenario: The work rectangle
- **WHEN** a game of RGSS3 draws `"x"` with the default font into the rectangle (0, 0, 40, 40) of a transparent bitmap of 40 by 40
- **THEN** a pixel inside the rectangle's border of one pixel that no glyph covers, (38, 38), is (255, 255, 255, 0): the colour of the text pass written with alpha 0, which the cell of RGSS2 never reaches

#### Scenario: Text of any class, measured on the original
- **WHEN** a game of RGSS3 evaluates `b.draw_text(0, 0, 40, 40, 123)`, `b.draw_text(0, 0, 40, 40, nil)`, `b.draw_text(0, 0, 40, 40, :a)`, `b.draw_text(Rect.new(0, 0, 40, 40), "a")`, `b.draw_text(0.5, 0, 40.9, 40, "a", 1.9)` and `b.text_size(nil).to_s`
- **THEN** the five draws return `nil` and the size is `"(0, 0, 0, 0)"`

#### Scenario: Measured text
- **WHEN** a game of RGSS3 evaluates `b.text_size(123).to_s == b.text_size("123").to_s` and `b.text_size("Hello").to_s` with the outline on and off
- **THEN** the first is `true` and the two sizes are equal

#### Scenario: Alignment of the wrong kind, measured on the original
- **WHEN** a game of RGSS3 evaluates `b.draw_text(0, 0, 40, 40, "a", nil)` and `b.draw_text(0, 0, 40, 40, "a", "x")`
- **THEN** each raises `TypeError`, with the messages `no implicit conversion from nil to integer` and `can't convert String into Integer`

### Requirement: Creation from a size in RGSS3
In RGSS3 `Bitmap.new(width, height)` SHALL have no limit of 4194304 pixels: a size SHALL be
refused only above the largest bitmap of the arena, 268435456 pixels, with `RGSSError` and
the message `failed to create bitmap`. A width of 0 or less or a height of 0 SHALL raise the
same; a negative height SHALL be taken by its size and Floats truncated.

#### Scenario: Sizes, measured on the original
- **WHEN** a game of RGSS3 evaluates `[w, h]` of `Bitmap.new(2049, 2048)`, `Bitmap.new(65536, 1)`, `Bitmap.new(10, -5)` and `Bitmap.new(3.9, 2.9)`
- **THEN** the results are `[2049, 2048]`, `[65536, 1]`, `[10, 5]` and `[3, 2]`

#### Scenario: Refused sizes, measured on the original
- **WHEN** a game of RGSS3 evaluates `Bitmap.new` with `(0, 10)`, `(10, 0)`, `(0, 0)`, `(-10, 1)`, `(-3, -4)`, `(0.9, 5)`, `(32768, 32768)` and `(20000, 20000)`
- **THEN** each raises `RGSSError` with the message `failed to create bitmap`

#### Scenario: Arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `Bitmap.new()`, `Bitmap.new(1, 2, 3)`, `Bitmap.new(1)`, `Bitmap.new(nil)`, `Bitmap.new("x", 1)` and `Bitmap.new(1, nil)`
- **THEN** the first two raise `ArgumentError` with `wrong number of arguments (0 for 1..2)` and `wrong number of arguments (3 for 1..2)`, and the others `TypeError` with `can't convert Fixnum into String`, `can't convert nil into String`, `can't convert String into Integer` and `no implicit conversion from nil to integer`

### Requirement: Argument errors of Bitmap in RGSS3
In RGSS3 a wrong count of a method of `Bitmap` of arity -1 SHALL raise `ArgumentError` with
`wrong number of arguments (E for G)`, the expected count first, as measured; a `Color`,
`Rect`, `Bitmap` or `Font` argument of another class SHALL raise `TypeError` with `can't
convert <class> into <Color|Rect|Bitmap|Font>`. A method called without arguments SHALL
raise and never end the process.

#### Scenario: Without arguments, measured on the original
- **WHEN** a game of RGSS3 calls `blt`, `clear_rect`, `draw_text`, `fill_rect`, `gradient_fill_rect`, `set_pixel`, `stretch_blt` and `text_size` of a bitmap without arguments
- **THEN** each raises `ArgumentError`, with `(4 for 0)`, `(4 for 0)`, `(5 for 0)`, `(5 for 0)`, `(6 for 0)`, `(0 for 3)`, `(3 for 0)` and `(0 for 1)` in its message

#### Scenario: Counts, measured on the original
- **WHEN** a game of RGSS3 evaluates `fill_rect` with 3, 4 and 6 numbers and with `(rect, nil)`, `blt` with 3 and 6 arguments, `stretch_blt` with 2, `clear_rect(1, 2)`, and `draw_text` with `(1, 2, 3, 4)`, `(rect)`, 7 arguments and `(rect, "a", 0, 0)`
- **THEN** the messages are `wrong number of arguments (5 for 3)`, `(5 for 4)`, `(5 for 6)`, `(5 for 2)`, `(4 for 3)`, `(5 for 6)`, `(3 for 2)`, `(4 for 2)`, `(5 for 4)`, `(2 for 1)`, `(6 for 7)` and `(3 for 4)`

#### Scenario: Types, measured on the original
- **WHEN** a game of RGSS3 evaluates `set_pixel(0, 0, nil)`, `set_pixel(0, 0, Tone.new(1, 2, 3))`, `fill_rect(0, 0, 1, 1, nil)`, `blt(0, 0, b, nil)`, `blt(0, 0, "x", r)` and `blt(0, 0, nil, r)`
- **THEN** each raises `TypeError`, with `can't convert NilClass into Color`, `can't convert Tone into Color`, `can't convert NilClass into Color`, `can't convert NilClass into Rect`, `can't convert String into Bitmap` and `can't convert NilClass into Bitmap`

#### Scenario: Fixed arity and conversions, measured on the original
- **WHEN** a game of RGSS3 evaluates `get_pixel(0)`, `clear(1)`, `hue_change`, `hue_change(nil)` and `blt(0, 0, b, r, nil)`
- **THEN** the first three raise `ArgumentError` with `wrong number of arguments(1 for 2)`, `wrong number of arguments(1 for 0)` and `wrong number of arguments(0 for 1)`, and the last two `TypeError` with `no implicit conversion from nil to integer`

### Requirement: Method surface of Bitmap and Font in RGSS3
In RGSS3 `Bitmap` and `Font` SHALL define, with the same arity, exactly the methods the
committed method list of RGSS3 gives them, and SHALL be among the classes its check covers.

#### Scenario: The list is checked
- **WHEN** the method-list check of RGSS3 runs
- **THEN** `Bitmap` and `Font` are among the classes it checks, and each listed method exists with its listed arity and no other public method is defined

#### Scenario: New methods of Font
- **WHEN** a game of RGSS3 evaluates `[Font.method(:default_outline).arity, Font.method(:default_out_color=).arity, Font.instance_method(:outline=).arity, Font.instance_method(:out_color).arity]`
- **THEN** the result is `[0, 1, 1, 0]`

### Requirement: Font and Bitmap of RGSS1 and RGSS2 are unchanged by RGSS3
The forms of RGSS3 SHALL not reach a game of RGSS1 or RGSS2: every existing test of `Font`
and `Bitmap` SHALL pass over both interpreters with its expected values unedited, and a game
of RGSS1 or RGSS2 SHALL have neither `outline` nor `out_color`.

#### Scenario: VX
- **WHEN** a game of RGSS2 evaluates `[Font.new.respond_to?(:outline), Font.respond_to?(:default_out_color), (f = Font.new; f.size = 24.7; f.size), Font.default_name]`
- **THEN** the result is `[false, false, 24, ["Verdana", "Arial", "Courier New"]]`

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `[Font.new.respond_to?(:outline), Font.respond_to?(:default_outline)]`
- **THEN** the result is `[false, false]`
