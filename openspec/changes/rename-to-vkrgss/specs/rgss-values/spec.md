## MODIFIED Requirements

### Requirement: Color and Tone
`Color.new(red, green, blue, alpha = 255)` and `Tone.new(red, green, blue, gray = 0)` SHALL
create values whose components are Floats, clamped as `vkrgss-core` defines, readable and
writable by name, settable together with `set`, comparable with `==`, `===` and `eql?`, and
printable with `to_s` as `(r, g, b, a)` with six decimals. Passing something that is not a
number SHALL raise `TypeError`, and a wrong number of arguments `ArgumentError`.

#### Scenario: Construction and clamping
- **WHEN** `c = Color.new(300, -300, 12.5, -7)` is evaluated
- **THEN** `c.red`, `c.green`, `c.blue` and `c.alpha` are `255.0`, `-255.0`, `12.5` and `0.0`

#### Scenario: Setters
- **WHEN** `t = Tone.new(0, 0, 0); t.gray = 300; t.set(-300, 1, 2)` is evaluated
- **THEN** `t.red` is `-255.0` and `t.gray` is `0.0`

#### Scenario: Equality and printing
- **WHEN** `Color.new(1, 2, 3) == Color.new(1, 2, 3, 255)` and `Color.new(1, 2, 3).to_s` are evaluated
- **THEN** the results are `true` and `"(1.000000, 2.000000, 3.000000, 255.000000)"`

#### Scenario: Wrong argument
- **WHEN** `Color.new("red", 0, 0)` is evaluated
- **THEN** a `TypeError` is raised

### Requirement: Table
`Table.new(xsize, ysize = 1, zsize = 1)` SHALL create a table of zeros with the behaviour of
`vkrgss-core`'s table. `[]` with as many indices as the table has dimensions SHALL return the
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
byte layouts of `vkrgss-core`, so that `Marshal.dump` writes and `Marshal.load` reads them as the
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

### Requirement: The value classes of RGSS1 and RGSS2 are unchanged by RGSS3
The value classes of a game of RGSS1 or RGSS2 SHALL NOT change with those of RGSS3: every
test of the two binding packages that passed before SHALL pass under its name with its
expected values unedited, over both interpreters. The tests of RGSS3 SHALL be files of the
package over Ruby 1.9.2 alone, and the scenarios named "XP and VX" SHALL be held by tests of
the shared files.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkrgss-binding` and of `vkrgss-binding-rgss3` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: Existing tests
- **WHEN** the difference of `crates/vkrgss-binding/tests` and `crates/vkrgss-binding-rgss3/tests` against the start of the change is read
- **THEN** it holds new files and one new test in each of `tests/ruby.rs` and `tests/scene_rgss2.rs` of `vkrgss-binding`; no test that existed, no expected value and no line of the allow-list changed

#### Scenario: Other crates
- **WHEN** the difference of `crates` against the start of the change is taken
- **THEN** it names files of `crates/vkrgss-binding` and `crates/vkrgss-binding-rgss3` only
