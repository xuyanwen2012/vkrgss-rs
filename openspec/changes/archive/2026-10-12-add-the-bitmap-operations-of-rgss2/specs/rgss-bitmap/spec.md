## ADDED Requirements

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
