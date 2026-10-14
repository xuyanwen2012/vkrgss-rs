# rgss-bitmap Specification

## Purpose
Defines the Ruby class `Bitmap` as scripts see it: creation from a size or a file, drawing,
text, copying and disposal, with the errors the original runtime raises.

## Requirements

### Requirement: Method surface of Bitmap
`Bitmap` SHALL define every method the original runtime defines for it with the same arity, as
listed in the committed method list.

#### Scenario: Every listed method exists
- **WHEN** each `Bitmap` entry of the method list is checked with `instance_method`
- **THEN** the method exists and its arity equals the listed arity

### Requirement: Creation from a size
`Bitmap.new(width, height)` SHALL create a transparent bitmap of that size. A size the bitmap
arena refuses SHALL raise `RGSSError` with the message `failed to create bitmap`.

#### Scenario: Blank bitmap
- **WHEN** `b = Bitmap.new(32, 16)` is evaluated
- **THEN** `b.width` is `32`, `b.height` is `16`, `b.rect` equals `Rect.new(0, 0, 32, 16)` and `b.get_pixel(0, 0)` equals `Color.new(0, 0, 0, 0)`

#### Scenario: Zero size
- **WHEN** `Bitmap.new(0, 10)` is evaluated
- **THEN** an `RGSSError` with the message `failed to create bitmap` is raised

### Requirement: Creation from a file
`Bitmap.new(path)` SHALL look the path up as an image through the game's files, as
`game-files` defines (letter case, either slash, optional extension, game directory before
RTPs), decode it and create a bitmap of its pixels. A path that is not found SHALL raise
`Errno::ENOENT` whose message ends with the path as given, and a file that cannot be decoded
SHALL raise `RGSSError`.

#### Scenario: File from the game directory
- **WHEN** the game directory holds `Graphics/Pictures/Dot.png`, a 2 by 2 image, and `Bitmap.new("graphics\\pictures\\dot")` is evaluated
- **THEN** the bitmap is 2 by 2 and its pixels equal the image's

#### Scenario: File from the run-time package
- **WHEN** the picture exists only in the configured run-time package
- **THEN** it is loaded from there

#### Scenario: Missing file
- **WHEN** `Bitmap.new("Graphics/Pictures/Nope")` is evaluated and no such file exists
- **THEN** `Errno::ENOENT` is raised and its message is `No such file or directory - Graphics/Pictures/Nope`

#### Scenario: Not an image
- **WHEN** the path resolves to a file whose contents are not a supported image
- **THEN** an `RGSSError` is raised

### Requirement: Disposal
`dispose` SHALL free the bitmap, `disposed?` SHALL report it, and every other method called on
a disposed bitmap SHALL raise `RGSSError` with the message `disposed bitmap`. Disposing twice
SHALL do nothing. A bitmap that becomes unreachable SHALL be freed by the garbage collector
without being disposed explicitly.

#### Scenario: Use after dispose
- **WHEN** `b = Bitmap.new(1, 1); b.dispose; b.width` is evaluated
- **THEN** an `RGSSError` with the message `disposed bitmap` is raised

#### Scenario: Unreachable bitmaps are freed
- **WHEN** ten thousand bitmaps are created in a loop without being kept or disposed, with garbage collections in between
- **THEN** the number of bitmaps in the arena ends far below ten thousand

### Requirement: Drawing methods
`clear`, `fill_rect` (with a rectangle and a colour, or with x, y, width, height and a colour),
`get_pixel`, `set_pixel`, `blt(x, y, source, source_rect, opacity = 255)`,
`stretch_blt(dest_rect, source, source_rect, opacity = 255)` and `hue_change(hue)` SHALL
perform the operation of the same name defined by `core-bitmap`. An argument of the wrong class
SHALL raise `TypeError`, and a disposed source bitmap SHALL raise nothing. The opacity SHALL
be passed on as the integer given.

#### Scenario: Fill and read
- **WHEN** `b = Bitmap.new(4, 4); b.fill_rect(1, 1, 2, 2, Color.new(255, 0, 0)); b.fill_rect(Rect.new(0, 0, 1, 1), Color.new(0, 255, 0, 128))` is evaluated
- **THEN** `b.get_pixel(1, 1)` equals `Color.new(255, 0, 0, 255)`, `b.get_pixel(0, 0)` equals `Color.new(0, 255, 0, 128)` and `b.get_pixel(3, 3)` equals `Color.new(0, 0, 0, 0)`

#### Scenario: Transfer
- **WHEN** a 2 by 2 red bitmap is transferred with `blt(1, 1, source, source.rect)` into a 4 by 4 transparent bitmap
- **THEN** pixels (1, 1) to (2, 2) of the destination are red and the others transparent

#### Scenario: Wrong class
- **WHEN** `b.blt(0, 0, "not a bitmap", Rect.new(0, 0, 1, 1))` is evaluated
- **THEN** a `TypeError` is raised

#### Scenario: Disposed source, measured on the originals
- **WHEN** `s.dispose; b.blt(0, 0, s, Rect.new(0, 0, 2, 2))` is evaluated on a bitmap `b` whose pixels have alpha 128
- **THEN** no error is raised and the four pixels from (0, 0) have alpha 191 with their colour unchanged

#### Scenario: Opacity above 255, measured on the originals
- **WHEN** `b.blt(0, 0, source, source.rect, 256)` and `b.blt(0, 0, source, source.rect, 255.9)` are evaluated
- **THEN** the first changes nothing and the second draws at opacity 255

### Requirement: Text
`draw_text` SHALL accept `(x, y, width, height, string, align = 0)` and
`(rect, string, align = 0)` and draw as `core-bitmap` defines with the bitmap's font, glyphs
coming from the guest's text backend. `text_size(string)` SHALL return a `Rect` at 0, 0 with the measured
width and height. A value that is not a String SHALL be converted with `to_s`. `font` SHALL
return the bitmap's own `Font` object, whose changes affect later drawing, and `font=` SHALL
replace it.

#### Scenario: Text leaves ink
- **WHEN** `b = Bitmap.new(200, 40); b.draw_text(0, 0, 200, 40, "Hello")` is evaluated
- **THEN** some pixel of `b` has a non-zero alpha and its colour is the font's colour

#### Scenario: Size of text
- **WHEN** `b.text_size("Hello")` is evaluated with font size 22
- **THEN** the result is a `Rect` with x 0, y 0, a positive width and height 22

#### Scenario: Font object is live
- **WHEN** `b.font.color = Color.new(255, 0, 0); b.draw_text(0, 0, 200, 40, "A")` is evaluated
- **THEN** the pixels the text covers are red

#### Scenario: Numbers are converted
- **WHEN** `b.draw_text(0, 0, 200, 40, 123)` is evaluated
- **THEN** the text `123` is drawn and no exception is raised

### Requirement: Copying
`clone` and `dup` of a bitmap SHALL return a new bitmap with the same size, pixels and font
values that is independent of the original.

#### Scenario: Independent copy
- **WHEN** `a = Bitmap.new(2, 2); b = a.clone; b.set_pixel(0, 0, Color.new(1, 2, 3)); a.dispose` is evaluated
- **THEN** `b.get_pixel(0, 0)` equals `Color.new(1, 2, 3, 255)` and `a.disposed?` is `true` while `b.disposed?` is `false`

### Requirement: Text argument errors of the original
`Font#size=` SHALL raise `ArgumentError` with the message `bad value for size` for a value
below 6 or above 96 and accept fractional values inside that range. `Bitmap#draw_text` and
`Bitmap#text_size` SHALL raise `TypeError` when the text argument is not a string.

#### Scenario: Size out of range
- **WHEN** `Font.new.size = 5` and `Font.new.size = 97` are evaluated
- **THEN** each raises `ArgumentError` with the message `bad value for size`, and `96.5` is accepted

#### Scenario: Not a string
- **WHEN** `bitmap.draw_text(0, 0, 100, 32, 123)` is evaluated
- **THEN** a `TypeError` is raised

### Requirement: Bitmap methods of RGSS2
In RGSS2 `Bitmap` SHALL define `blur`, `radial_blur(angle, division)`, `clear_rect` with a
`Rect` or four integers, and `gradient_fill_rect` with a `Rect` or four integers, two colours
and an optional vertical flag, each returning `nil` and raising `RGSSError` on a disposed
bitmap. With seven arguments the gradient SHALL be vertical whatever the seventh is. In RGSS1
none of them SHALL be defined.

#### Scenario: Counts, measured on the original
- **WHEN** a game of RGSS2 evaluates `b.clear_rect(1, 2)`, `b.gradient_fill_rect(r, c1)` and `b.blur(1)`
- **THEN** each raises `ArgumentError`

#### Scenario: Seventh argument, measured on the original
- **WHEN** a game of RGSS2 evaluates `b.gradient_fill_rect(0, 0, 4, 4, c1, c2, false)` and `b.gradient_fill_rect(Rect.new(0, 0, 4, 4), c1, c2, false)` on two bitmaps
- **THEN** the first gradient runs from top to bottom and the second from left to right

#### Scenario: Wrong colour, measured on the original
- **WHEN** a game of RGSS2 evaluates `b.gradient_fill_rect(r, nil, c2)`
- **THEN** a `TypeError` with the message `can't convert NilClass into Color` is raised

#### Scenario: No argument
- **WHEN** a game of RGSS2 evaluates `b.clear_rect` and `b.gradient_fill_rect`
- **THEN** each raises `ArgumentError`

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Bitmap.new(1, 1).respond_to?(:blur)`
- **THEN** the result is `false`

### Requirement: Font of RGSS2
In RGSS2 the font defaults SHALL be the name `["Verdana", "Arial", "Courier New"]`, size 20,
bold and italic off, shadow on and the colour white; `Font#shadow` SHALL read true or false
and `Font#shadow=` and `Font.default_shadow=` SHALL take any object; `Bitmap#font=` SHALL copy
name, size, bold, italic and colour and not the shadow; `dup` and `clone` of a bitmap SHALL
copy the shadow. In RGSS1 no shadow property SHALL be defined and the defaults SHALL stay.

#### Scenario: Defaults, measured on the original
- **WHEN** a game of RGSS2 evaluates `f = Font.new; [f.name, f.size, f.bold, f.italic, f.shadow]`
- **THEN** the result is `[["Verdana", "Arial", "Courier New"], 20, false, false, true]`

#### Scenario: Assigning a font, measured on the original
- **WHEN** a game of RGSS2 evaluates `f = Font.new; f.shadow = false; f.size = 30; b.font = f; [b.font.size, b.font.shadow]`
- **THEN** the result is `[30, true]`

#### Scenario: Truthiness, measured on the original
- **WHEN** a game of RGSS2 evaluates `f.shadow = 0; f.shadow` and `f.shadow = nil; f.shadow`
- **THEN** the results are `true` and `false`

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Font.new.respond_to?(:shadow)` and `Font.default_size`
- **THEN** the results are `false` and `22`

### Requirement: Differences of Bitmap in RGSS2
In RGSS2 `Bitmap.new(path)` for a file that is empty or cannot be decoded, and
`Bitmap.new(width, height)` for more than 4194304 pixels, SHALL raise `RGSSError` with the
message `failed to create bitmap`; and `draw_text` and `text_size` SHALL take a text of any
class and use its `to_s`. In RGSS1 these cases SHALL stay as they are.

#### Scenario: Undecodable file, measured on the original
- **WHEN** a game of RGSS2 evaluates `Bitmap.new("Graphics/broken")` for a file of that name with no picture in it
- **THEN** `RGSSError` is raised with the message `failed to create bitmap`

#### Scenario: Size limit, measured on the original
- **WHEN** a game of RGSS2 evaluates `Bitmap.new(2048, 2048)` and `Bitmap.new(2049, 2048)`
- **THEN** the first succeeds and the second raises `RGSSError`

#### Scenario: Text of another class, measured on the original
- **WHEN** a game of RGSS2 evaluates `b.text_size(123).width == b.text_size("123").width` and `b.text_size(nil).width`
- **THEN** the results are `true` and `0`

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
