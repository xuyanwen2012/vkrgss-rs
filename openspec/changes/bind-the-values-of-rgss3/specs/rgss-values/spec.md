## ADDED Requirements

### Requirement: Color and Tone of RGSS3
In RGSS3 `Color.new` and `Tone.new` SHALL take no argument, giving (0, 0, 0, 0), or three or
four numbers with the limits of RGSS2. `set` SHALL take the same forms, or one `Color` or
`Tone` whose four numbers it copies in order, and SHALL return the receiver. One argument
that is neither SHALL raise `TypeError`. In RGSS1 and RGSS2 the two classes SHALL stay as
they are.

#### Scenario: Without arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Color.new.to_s, Tone.new.to_s]`
- **THEN** both are `"(0.000000, 0.000000, 0.000000, 0.000000)"`

#### Scenario: Three and four numbers, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Color.new(1, 2, 3).alpha, Tone.new(1, 2, 3).gray, Color.new(1.5, 2.25, 3, 4).to_s]`
- **THEN** the result is `[255.0, 0.0, "(1.500000, 2.250000, 3.000000, 4.000000)"]`

#### Scenario: Limits, measured on the original
- **WHEN** a game of RGSS3 evaluates `Color.new(-300, 300, 7.6, -7).to_s`, `Color.new(2**40, 2, 3).red`, `Tone.new(-300, 300, 1000.5, -5).to_s` and `Tone.new(0, 0, 0, 300).gray`
- **THEN** the results are `"(-255.000000, 255.000000, 7.600000, 0.000000)"`, `255.0`, `"(-255.000000, 255.000000, 255.000000, 0.000000)"` and `255.0`

#### Scenario: Writers, measured on the original
- **WHEN** a game of RGSS3 evaluates `c = Color.new(1, 2, 3, 4); [(c.red = 300), c.red, (c.alpha = -1), c.alpha, (c.green = -255.9), c.green]` and `t = Tone.new(1, 2, 3, 4); [(t.gray = 300), t.gray, (t.red = -300), t.red]`
- **THEN** the results are `[300, 255.0, -1, 0.0, -255.9, -255.0]` and `[300, 255.0, -300, -255.0]`

#### Scenario: Set with numbers and without, measured on the original
- **WHEN** a game of RGSS3 evaluates `c = Color.new(1, 2, 3, 4); a = [c.set(10, 20, 30).equal?(c), c.to_s]; b = [c.set.equal?(c), c.to_s]; [a, b]`, and the same with `Tone`
- **THEN** `a` is `[true, "(10.000000, 20.000000, 30.000000, 255.000000)"]` for the colour and `[true, "(10.000000, 20.000000, 30.000000, 0.000000)"]` for the tone, and `b` is `[true, "(0.000000, 0.000000, 0.000000, 0.000000)"]` for both

#### Scenario: Set with an object, measured on the original
- **WHEN** a game of RGSS3 evaluates `c = Color.new(1, 2, 3, 4); o = Color.new(-9, 255, 2.5, 77); [c.set(o).equal?(c), c.equal?(o), c.to_s]`, and the same with `Tone`
- **THEN** both results are `[true, false, "(-9.000000, 255.000000, 2.500000, 77.000000)"]`

#### Scenario: Between the two classes, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Color.new.set(Tone.new(1, 2, 3, 4)).to_s, Tone.new.set(Color.new(1, 2, 3, 4)).to_s]`
- **THEN** both are `"(1.000000, 2.000000, 3.000000, 4.000000)"`

#### Scenario: One argument that is no such object, measured on the original
- **WHEN** a game of RGSS3 evaluates `Color.new.set(1)`, `Color.new.set(nil)` and `Tone.new.set(1)`
- **THEN** each raises `TypeError`, with the messages `wrong argument type Fixnum (expected Data)`, `wrong argument type nil (expected Data)` and `wrong argument type Fixnum (expected Data)`

#### Scenario: An object of another native class
- **WHEN** a game of RGSS3 evaluates `Color.new.set(Rect.new(1, 2, 3, 4))`
- **THEN** a `TypeError` is raised, where the original copies bytes

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `Color.new`, `Tone.new` and `Color.new(1, 2, 3).set(Color.new(1, 2, 3))`
- **THEN** each raises `ArgumentError`

### Requirement: Rect of RGSS3
In RGSS3 `Rect.new` SHALL take no argument, giving (0, 0, 0, 0), or four Integers; `set`
SHALL take four Integers or one `Rect`, whose values it copies, and SHALL return the
receiver, as `empty` does. Floats SHALL be truncated and negative sizes kept. One argument
of `set` that is no `Rect` SHALL raise `TypeError`. A change made by any form of `set` SHALL
reach the element the `Rect` belongs to. In RGSS1 and RGSS2 `Rect` SHALL stay as it is.

#### Scenario: Without arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `Rect.new.to_s`
- **THEN** the result is `"(0, 0, 0, 0)"`

#### Scenario: Floats and negative values, measured on the original
- **WHEN** a game of RGSS3 evaluates `Rect.new(1.9, -2.9, 3.5, 4.5).to_s`, `Rect.new(-1, -2, -3, -4).to_s` and `r = Rect.new(1, 2, 3, 4); [(r.x = 1.9), r.x, (r.width = -5), r.width]`
- **THEN** the results are `"(1, -2, 3, 4)"`, `"(-1, -2, -3, -4)"` and `[1.9, 1, -5, -5]`

#### Scenario: Set, measured on the original
- **WHEN** a game of RGSS3 evaluates `r = Rect.new(1, 2, 3, 4); a = [r.set(10, 20, 30, 40).equal?(r), r.to_s]; b = [r.set(Rect.new(-5, 6, -7, 8)).equal?(r), r.to_s]; c = [r.empty.equal?(r), r.to_s]; [a, b, c]`
- **THEN** the result is `[[true, "(10, 20, 30, 40)"], [true, "(-5, 6, -7, 8)"], [true, "(0, 0, 0, 0)"]]`

#### Scenario: One argument that is no Rect, measured on the original
- **WHEN** a game of RGSS3 evaluates `Rect.new.set(1)` and `Rect.new.set(nil)`
- **THEN** each raises `TypeError`, with the messages `wrong argument type Fixnum (expected Data)` and `wrong argument type nil (expected Data)`

#### Scenario: An object of another native class
- **WHEN** a game of RGSS3 evaluates `Rect.new.set(Color.new(1, 2, 3, 4))`
- **THEN** a `TypeError` is raised, where the original copies bytes

#### Scenario: Seen by the element
- **WHEN** a game of RGSS3 shows a sprite at (0, 0) with an opaque red bitmap of 4 by 4, evaluates `sprite.src_rect.set(Rect.new(0, 0, 2, 2))` and reads the frame, then `sprite.color.set(Color.new(0, 0, 255, 255))` and reads it, then `sprite.color.set` and reads it
- **THEN** the pixel (1, 1) is red, blue and red in the three frames, and the pixel (3, 3) is black in all three

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `Rect.new`, `Rect.new(1, 2, 3, 4).set(Rect.new(1, 2, 3, 4))` and `Rect.instance_method(:initialize).arity`
- **THEN** the first two raise `ArgumentError` and the third is 4

### Requirement: Argument errors of the value classes in RGSS3
In RGSS3 a count of arguments that `Color.new`, `Tone.new`, `Rect.new`, their `set`,
`Table#[]` or `Table#[]=` does not take SHALL raise `ArgumentError` with the message `wrong
number of arguments (E for G)`, G the count given and E the expected one: for `Color` and
`Tone` 3 below three and 4 above four, for `Rect` 4, for `Table` its number of dimensions.
An argument of the wrong kind SHALL raise the error of Ruby 1.9 with its message.

#### Scenario: Color and Tone, measured on the original
- **WHEN** a game of RGSS3 evaluates `Color.new(1)`, `Color.new(1, 2)`, `Color.new(1, 2, 3, 4, 5)`, `Color.new(Color.new)`, `Tone.new(1)`, `Tone.new(1, 2)`, `Tone.new(1, 2, 3, 4, 5)`, `Color.new.set(1, 2)`, `Color.new.set(1, 2, 3, 4, 5)` and `Tone.new.set(1, 2, 3, 4, 5)`
- **THEN** each raises `ArgumentError` with the message `wrong number of arguments (E for G)`, the pairs being (3 for 1), (3 for 2), (4 for 5), (3 for 1), (3 for 1), (3 for 2), (4 for 5), (3 for 2), (4 for 5) and (4 for 5)

#### Scenario: Rect, measured on the original
- **WHEN** a game of RGSS3 evaluates `Rect.new(1)`, `Rect.new(1, 2)`, `Rect.new(1, 2, 3)`, `Rect.new(1, 2, 3, 4, 5)`, `Rect.new(Rect.new)`, and for `r = Rect.new` the calls `r.set`, `r.set(1, 2)`, `r.set(1, 2, 3)` and `r.set(1, 2, 3, 4, 5)`
- **THEN** each raises `ArgumentError` with the message `wrong number of arguments (4 for G)`, G being 1, 2, 3, 5, 1, 0, 2, 3 and 5

#### Scenario: Table, measured on the original
- **WHEN** a game of RGSS3 evaluates, for `t = Table.new(2, 2, 2)`, `t[0]`, `t[0, 0]`, `t[0, 0, 0, 0]` and `t[0] = 5`, and after `t.resize(3, 3)`, `t[0]`
- **THEN** each raises `ArgumentError`, with the messages `wrong number of arguments (3 for 1)`, `(3 for 2)`, `(3 for 4)`, `(3 for 2)` and `(2 for 1)` after the same first words

#### Scenario: Numbers of the wrong kind, measured on the original
- **WHEN** a game of RGSS3 evaluates `Color.new("1", 2, 3)`, `Color.new(nil, 2, 3)`, `Color.new.red = nil`, `Rect.new("1", 2, 3, 4)`, `Rect.new(nil, 2, 3, 4)`, `Rect.new.x = "a"`, `Table.new("a")`, `Table.new(nil)` and `Table.new(1)["a"]`
- **THEN** each raises `TypeError`; the first three with `no implicit conversion to float from string`, `no implicit conversion to float from nil` and the latter again; the others with `can't convert String into Integer` for a String and `no implicit conversion from nil to integer` for `nil`

#### Scenario: Out of range
- **WHEN** a game of RGSS3 evaluates `Rect.new(2**31, 2, 3, 4)`, `Rect.new.x = 2**31` and `Rect.new(1e10, 0, 0, 0)`
- **THEN** each raises `RangeError`

#### Scenario: A method of fixed arity, measured on the original
- **WHEN** a game of RGSS3 evaluates `Color.new.to_s(1)`
- **THEN** an `ArgumentError` with the message `wrong number of arguments(1 for 0)` is raised

### Requirement: Comparison of the value classes in RGSS3
In RGSS3 `==`, `===` and `eql?` of `Color`, `Tone` and `Rect` SHALL be true for an object of
the same class with equal values and false for everything else, `nil`, a number and an
object of another of these classes among them, as version 3.0.1 of the original runtime
has it; they SHALL NOT raise, whatever version the game ships. `hash` SHALL NOT be computed
from the values.

#### Scenario: Equal values, measured on the original
- **WHEN** a game of RGSS3 evaluates `a = Color.new(1.5, 0, 0); b = Color.new(1.5, 0, 0); [a == b, a.eql?(b), a === b, a.equal?(b), a == Color.new(1.5, 0, 1)]`, `Tone.new(-255, 6, 7, 255) == Tone.new(-255, 6, 7, 255)` and `Rect.new(1, 0, -5, 0) == Rect.new(1, 0, -5, 0)`
- **THEN** the results are `[true, true, true, false, false]`, `true` and `true`

#### Scenario: Another class, measured on version 3.0.1 of the original
- **WHEN** a game of RGSS3 evaluates `c = Color.new(1, 2, 3, 4); [c == nil, c == 5, c == Tone.new(1, 2, 3, 4)]`, `t = Tone.new(1, 2, 3, 4); [t == nil, t == Color.new(1, 2, 3, 4)]` and `r = Rect.new(1, 2, 3, 4); [r == nil, r == 5, r == Color.new(1, 2, 3, 4)]`
- **THEN** every element of the three results is `false` and nothing is raised

#### Scenario: Hash, measured on the original
- **WHEN** a game of RGSS3 evaluates `Color.new(1, 2, 3).hash == Color.new(1, 2, 3).hash`
- **THEN** the result is `false`

#### Scenario: Copies, measured on the original
- **WHEN** a game of RGSS3 evaluates `a = Color.new(1, 2, 3, 4); b = a.dup; b.red = 9; [a.to_s, b.to_s, a.clone == a]`
- **THEN** the result is `["(1.000000, 2.000000, 3.000000, 4.000000)", "(9.000000, 2.000000, 3.000000, 4.000000)", true]`

### Requirement: Serialized forms of the value classes in RGSS3
In RGSS3 `_dump` SHALL take one argument, ignore it and return the bytes of RGSS2 as an
ASCII-8BIT String. `Color._load` of a String that is not 32 bytes long SHALL give
(0, 0, 0, 0). `Rect._load` of fewer than 16 bytes SHALL read the missing bytes as zeros.
`Table._load` SHALL keep the number of dimensions the String names and SHALL raise
`ArgumentError` for malformed data, where the original ends.

#### Scenario: Bytes, measured on the original
- **WHEN** a game of RGSS3 evaluates `Color.new(1, 2, 3, 4)._dump(0).unpack("H*")`, `Rect.new(1, 2, 3, 4)._dump(0).unpack("H*")` and `t = Table.new(2, 1, 1); t[0, 0, 0] = 7; t._dump(0).unpack("l5s*")`
- **THEN** the results are `["000000000000f03f000000000000004000000000000008400000000000001040"]`, `["01000000020000000300000004000000"]` and `[3, 2, 1, 1, 2, 7, 0]`

#### Scenario: Sizes of a table in its dump, measured on the original
- **WHEN** a game of RGSS3 evaluates `Table.new(0)._dump(0).unpack("l5")` and `t = Table.new(2, 2, 2); t.resize(3); t._dump(0).unpack("l5")`
- **THEN** the results are `[1, 0, 1, 1, 0]` and `[1, 3, 1, 1, 3]`

#### Scenario: The argument and the encoding
- **WHEN** a game of RGSS3 evaluates `c = Color.new(1, 2, 3, 4); [c._dump(0) == c._dump(-1), c._dump(0).encoding.name, Marshal.dump(c)[2]]`
- **THEN** the result is `[true, "ASCII-8BIT", "u"]`

#### Scenario: Loading what was dumped, measured on the original
- **WHEN** a game of RGSS3 evaluates `Color._load([1.0, 2.0, 3.0, 4.0].pack("d4")).to_s`, `Tone._load([1.0, 2.0, 3.0, 4.0].pack("d4")).to_s`, `Rect._load([1, 2, 3, 4].pack("l4")).to_s` and `t = Table._load([1, 2, 1, 1, 2, 5, 6].pack("l5s2")); [t.xsize, t.ysize, t.zsize, t[0], t[1]]`
- **THEN** the results are `"(1.000000, 2.000000, 3.000000, 4.000000)"` twice, `"(1, 2, 3, 4)"` and `[2, 1, 1, 5, 6]`

#### Scenario: A colour of another length, measured on the original
- **WHEN** a game of RGSS3 evaluates `Color._load("abc").to_s`
- **THEN** the result is `"(0.000000, 0.000000, 0.000000, 0.000000)"`

#### Scenario: A short rectangle
- **WHEN** a game of RGSS3 evaluates `Rect._load("ab").to_s`
- **THEN** the result is `"(25185, 0, 0, 0)"`: the first three values are those measured on the original, and the fourth is 0 where the original read 8199 from memory behind the String

#### Scenario: Dimensions of a loaded table, measured on the original
- **WHEN** a game of RGSS3 evaluates `t = Table._load(([1, 2, 2, 2, 8] + [0] * 8).pack("l5s8")); [t.xsize, t.ysize, t[0]]`, and then `t[0, 0]`
- **THEN** the first result is `[2, 2, 0]` and the second raises `ArgumentError`

#### Scenario: A short table
- **WHEN** a game of RGSS3 evaluates `Table._load("ab")`
- **THEN** an `ArgumentError` is raised and the interpreter goes on, where the original runtime ends with a page fault

#### Scenario: A subclass and instance variables, measured on the original
- **WHEN** a game of RGSS3 defines a subclass of `Color`, evaluates `Marshal.dump` of an instance of it, and of a `Color` after `instance_variable_set(:@zz, 5)`
- **THEN** the first dump holds the name of the subclass, and the third byte of the second is `"I"`

#### Scenario: Round trip
- **WHEN** a game of RGSS3 evaluates `Marshal.load(Marshal.dump([Color.new(1, 2, 3, 4), Tone.new(-1, 2, 3, 4), Rect.new(1, 2, 3, 4)]))`
- **THEN** the result equals the original array element by element

### Requirement: Text forms of the value classes in RGSS3
In RGSS3 `to_s` of a `Color` and of a `Tone` SHALL be its four components with six decimals
as `(r, g, b, a)`, and that of a `Rect` its four integers as `(x, y, width, height)`, each
an ASCII-8BIT String, and `inspect` of the three SHALL give the same text. `Table` SHALL
have no text form of its own: its `inspect` is that of `Object`.

#### Scenario: Text, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Color.new(1, 2.5, 3, 4).to_s, Tone.new(1, 2.5, 3, 4).to_s, Rect.new(1, -2, 3, 4).to_s]`
- **THEN** the result is `["(1.000000, 2.500000, 3.000000, 4.000000)", "(1.000000, 2.500000, 3.000000, 4.000000)", "(1, -2, 3, 4)"]`

#### Scenario: Inspect, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Tone.new.inspect, Rect.new.inspect, Color.new(1, 2, 3).inspect]`
- **THEN** the result is `["(0.000000, 0.000000, 0.000000, 0.000000)", "(0, 0, 0, 0)", "(1.000000, 2.000000, 3.000000, 255.000000)"]`

#### Scenario: Encoding, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Color.new.to_s, Tone.new.to_s, Rect.new.to_s, Color.new.inspect].map { |text| text.encoding.name }`
- **THEN** every name is `"ASCII-8BIT"`

#### Scenario: Table, measured on the original
- **WHEN** a game of RGSS3 evaluates `Table.new(1).inspect`
- **THEN** the result has the form `#<Table:0x...>` with a hexadecimal address

### Requirement: Table of RGSS3
In RGSS3 `Table` SHALL be the table of RGSS2: one to three sizes, a size below 0 counting as
0 and a Float truncated; an element keeps the low 16 bits of the Integer written and reads
as a signed number; a read outside the table gives `nil` and a write there changes nothing
and returns its value; `resize` returns the table.

#### Scenario: Sizes, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Table.new(2), Table.new(2, 3), Table.new(0), Table.new(-1), Table.new(0, 0, 0), Table.new(1.9, 2.9)].map { |t| [t.xsize, t.ysize, t.zsize] }`
- **THEN** the result is `[[2, 1, 1], [2, 3, 1], [0, 1, 1], [0, 1, 1], [0, 0, 0], [1, 2, 1]]`

#### Scenario: Elements, measured on the original
- **WHEN** a game of RGSS3 evaluates `t = Table.new(2, 2, 2); t[0, 0, 0] = 40000; t[1, 1, 1] = -40000; a = [t[0, 0, 0], t[1, 1, 1]]; t[1, 1, 1] = 1.9; [a, t[1, 1, 1], t[2, 0, 0], t[-1, 0, 0], (t[2, 0, 0] = 5), t[1.9, 0, 0]]`
- **THEN** the result is `[[-25536, 25536], 1, nil, nil, 5, 0]`

#### Scenario: Resize, measured on the original
- **WHEN** a game of RGSS3 evaluates `t = Table.new(2, 2, 2); [t.resize(3).equal?(t), [t.xsize, t.ysize, t.zsize], t.resize(-1).xsize]` and `d = Table.new(2, 1, 1).dup; [d.xsize, d.ysize, d.zsize]`
- **THEN** the results are `[true, [3, 1, 1], 0]` and `[2, 1, 1]`

### Requirement: Method surface of the value classes in RGSS3
In RGSS3 `Color`, `Tone`, `Rect` and `Table` SHALL define every method of the committed
method list of RGSS3 with its arity and no public instance method beside them. The list
SHALL be committed for every class and module of RGSS3 that has methods, names and arities
only, and its test SHALL check the classes it names as bound: after this change these four.

#### Scenario: Every listed method exists
- **WHEN** each entry of the list of RGSS3 for the four classes is checked in a game of RGSS3 with `instance_method` or `method`
- **THEN** the 17, 17, 18 and 10 entries of `Color`, `Tone`, `Rect` and `Table` exist with their arities, `Rect#initialize` and `Rect#set` with -1

#### Scenario: Nothing beside the list
- **WHEN** `public_instance_methods(false)` of each of the four classes is compared with the entries of the list in a game of RGSS3
- **THEN** no class has a method that the list lacks

#### Scenario: The whole list
- **WHEN** the committed list of RGSS3 is parsed
- **THEN** it has entries for `Graphics`, `Audio`, `Input`, `Sprite`, `Plane`, `Viewport`, `Tilemap`, `Tilemap::BitmapArray`, `Window`, `Bitmap`, `Font`, `Color`, `Tone`, `Rect`, `Table` and `Win32API`, each property with its reader of arity 0 and its writer of arity 1, and nothing but names and arities

#### Scenario: A class that is not bound yet
- **WHEN** the test of the list runs after this change
- **THEN** it passes although `Window` of a game of RGSS3 has no `move` yet, and it names the four classes it checks in one list

### Requirement: The value classes of RGSS1 and RGSS2 are unchanged by RGSS3
The value classes of a game of RGSS1 or RGSS2 SHALL NOT change with those of RGSS3: every
test of the two binding packages that passed before SHALL pass under its name with its
expected values unedited, over both interpreters. The tests of RGSS3 SHALL be files of the
package over Ruby 1.9.2 alone, and the scenarios named "XP and VX" SHALL be held by tests of
the shared files.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkxp-binding` and of `vkxp-binding-rgss3` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: Existing tests
- **WHEN** the difference of `crates/vkxp-binding/tests` and `crates/vkxp-binding-rgss3/tests` against the start of the change is read
- **THEN** it holds new files and one new test in each of `tests/ruby.rs` and `tests/scene_rgss2.rs` of `vkxp-binding`; no test that existed, no expected value and no line of the allow-list changed

#### Scenario: Other crates
- **WHEN** the difference of `crates` against the start of the change is taken
- **THEN** it names files of `crates/vkxp-binding` and `crates/vkxp-binding-rgss3` only
