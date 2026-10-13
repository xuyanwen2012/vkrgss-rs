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
SHALL raise `TypeError` and a disposed source bitmap SHALL raise `RGSSError`.

#### Scenario: Fill and read
- **WHEN** `b = Bitmap.new(4, 4); b.fill_rect(1, 1, 2, 2, Color.new(255, 0, 0)); b.fill_rect(Rect.new(0, 0, 1, 1), Color.new(0, 255, 0, 128))` is evaluated
- **THEN** `b.get_pixel(1, 1)` equals `Color.new(255, 0, 0, 255)`, `b.get_pixel(0, 0)` equals `Color.new(0, 255, 0, 128)` and `b.get_pixel(3, 3)` equals `Color.new(0, 0, 0, 0)`

#### Scenario: Transfer
- **WHEN** a 2 by 2 red bitmap is transferred with `blt(1, 1, source, source.rect)` into a 4 by 4 transparent bitmap
- **THEN** pixels (1, 1) to (2, 2) of the destination are red and the others transparent

#### Scenario: Wrong class
- **WHEN** `b.blt(0, 0, "not a bitmap", Rect.new(0, 0, 1, 1))` is evaluated
- **THEN** a `TypeError` is raised

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
