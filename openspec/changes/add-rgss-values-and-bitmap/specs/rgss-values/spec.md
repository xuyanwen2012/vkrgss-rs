# Spec Delta

## Purpose
Defines the Ruby classes `Color`, `Tone`, `Rect`, `Table` and `Font` as scripts see them: their
constructors, methods and arities, and the serialized forms `Marshal` uses, matching the
original runtime.

## ADDED Requirements

### Requirement: Method surface of the value classes
`Color`, `Tone`, `Rect`, `Table` and `Font` SHALL define every method the original runtime
defines for them, instance or class level, with the same arity, as listed in the committed
method list read from the original runtime.

#### Scenario: Every listed method exists
- **WHEN** each entry of the method list for these five classes is checked with `instance_method` or `method`
- **THEN** the method exists and its arity equals the listed arity

#### Scenario: Wrong number of arguments
- **WHEN** `Rect.new(0, 0, 1, 1).send(:x=, 1, 2)` is evaluated
- **THEN** an `ArgumentError` with the message `wrong number of arguments (2 for 1)` is raised

### Requirement: Color and Tone
`Color.new(red, green, blue, alpha = 255)` and `Tone.new(red, green, blue, gray = 0)` SHALL
create values whose components are Floats, clamped as `vkxp-core` defines, readable and
writable by name, settable together with `set`, comparable with `==`, `===` and `eql?`, and
printable with `to_s` as `(r, g, b, a)` with six decimals. Passing something that is not a
number SHALL raise `TypeError`, and a wrong number of arguments `ArgumentError`.

#### Scenario: Construction and clamping
- **WHEN** `c = Color.new(300, -5, 12.5)` is evaluated
- **THEN** `c.red`, `c.green`, `c.blue` and `c.alpha` are `255.0`, `0.0`, `12.5` and `255.0`

#### Scenario: Setters
- **WHEN** `t = Tone.new(0, 0, 0); t.gray = 300; t.set(-300, 1, 2)` is evaluated
- **THEN** `t.red` is `-255.0` and `t.gray` is `0.0`

#### Scenario: Equality and printing
- **WHEN** `Color.new(1, 2, 3) == Color.new(1, 2, 3, 255)` and `Color.new(1, 2, 3).to_s` are evaluated
- **THEN** the results are `true` and `"(1.000000, 2.000000, 3.000000, 255.000000)"`

#### Scenario: Wrong argument
- **WHEN** `Color.new("red", 0, 0)` is evaluated
- **THEN** a `TypeError` is raised

### Requirement: Rect
`Rect.new(x, y, width, height)` SHALL take exactly four Integers and store them as given. The
four values SHALL be readable and writable by name, settable together with `set`, cleared to
zero by `empty`, comparable with `==`, `===` and `eql?`, and printable with `to_s` as
`(x, y, width, height)`.

#### Scenario: Four arguments required
- **WHEN** `Rect.new(1, 2, 3)` is evaluated
- **THEN** an `ArgumentError` is raised

#### Scenario: Values and printing
- **WHEN** `r = Rect.new(-1, 2, 30, 40); r.width = 5` is evaluated
- **THEN** `r.to_s` is `"(-1, 2, 5, 40)"`

#### Scenario: Empty
- **WHEN** `r.empty` is evaluated for a rectangle with non-zero values
- **THEN** `r == Rect.new(0, 0, 0, 0)` is `true`

### Requirement: Table
`Table.new(xsize, ysize = 1, zsize = 1)` SHALL create a table of zeros with the behaviour of
`vkxp-core`'s table. `[]` with as many indices as the table has dimensions SHALL return the
element or `nil` outside the bounds, `[]=` SHALL store a value or do nothing outside the
bounds, and `resize`, `xsize`, `ysize` and `zsize` SHALL be provided.

#### Scenario: Read and write
- **WHEN** `t = Table.new(4, 3); t[1, 2] = 7` is evaluated
- **THEN** `t[1, 2]` is `7`, `t[0, 0]` is `0` and `t[4, 0]` is `nil`

#### Scenario: Sizes
- **WHEN** `t = Table.new(17)` is evaluated
- **THEN** `t.xsize`, `t.ysize` and `t.zsize` are `17`, `1` and `1`

#### Scenario: Resize keeps contents
- **WHEN** `t = Table.new(2, 2); t[1, 1] = 9; t.resize(3, 3)` is evaluated
- **THEN** `t[1, 1]` is `9` and `t[2, 2]` is `0`

### Requirement: Marshal forms
`Color`, `Tone`, `Rect` and `Table` SHALL define `_dump` and the class method `_load` with the
byte layouts of `vkxp-core`, so that `Marshal.dump` writes and `Marshal.load` reads them as the
original runtime does. Loading malformed data SHALL raise an exception instead of creating a
value.

#### Scenario: Round trip through Marshal
- **WHEN** `Marshal.load(Marshal.dump([Color.new(1, 2, 3, 4), Tone.new(-1, 2, 3, 4), Rect.new(1, 2, 3, 4)]))` is evaluated
- **THEN** the result equals the original array element by element

#### Scenario: Table bytes
- **WHEN** `Marshal.dump(Table.new(20, 15, 3))` is evaluated
- **THEN** the dump contains a user-defined `Table` payload of 1820 bytes that starts with the 32-bit integers 3, 20, 15, 3, 900

#### Scenario: Data written by the original runtime
- **WHEN** a dump holding a `Table` payload with header 2, 6, 100, 1, 600 and 1200 bytes of elements is loaded
- **THEN** the result is a table with `xsize` 6 and `ysize` 100

#### Scenario: Malformed payload
- **WHEN** a `Color` payload of 31 bytes is loaded
- **THEN** an exception is raised

### Requirement: Font
`Font.new(name = Font.default_name, size = Font.default_size)` SHALL create a font whose `name`
(a String or an Array of Strings), `size`, `bold`, `italic` and `color` are readable and
writable. The class SHALL provide `default_name`, `default_size`, `default_bold`,
`default_italic` and `default_color` with their writers, which new fonts copy, and
`Font.exist?(name)`, which answers from the guest's text backend.

#### Scenario: Defaults are copied
- **WHEN** `Font.default_size = 30; f = Font.new; Font.default_size = 22` is evaluated
- **THEN** `f.size` is `30`

#### Scenario: Name as an array
- **WHEN** `Font.default_name = ["NoSuchFont", "Liberation Sans"]; f = Font.new` is evaluated
- **THEN** `f.name` is that array

#### Scenario: Colour is the font's own
- **WHEN** `f = Font.new; f.color.red = 0` is evaluated
- **THEN** `f.color.red` is `0.0` and `Font.default_color.red` is unchanged

#### Scenario: Existence
- **WHEN** `Font.exist?("NoSuchFont")` is evaluated with a text backend that has no such family
- **THEN** the result is `false`

### Requirement: Copies are independent
`clone` and `dup` of a `Color`, `Tone`, `Rect`, `Table` or `Font` SHALL return a new object
with equal contents whose later changes do not affect the original.

#### Scenario: Cloned table
- **WHEN** `a = Table.new(2); b = a.clone; b[0] = 5` is evaluated
- **THEN** `a[0]` is `0`
