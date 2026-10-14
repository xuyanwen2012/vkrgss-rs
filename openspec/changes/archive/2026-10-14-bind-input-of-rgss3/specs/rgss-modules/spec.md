## MODIFIED Requirements

### Requirement: Method surface of the modules
`Graphics`, `Input` and `Audio` SHALL define every module function the original runtime defines
for them with the same arity, as listed in the committed method list, and `Input` SHALL define
the twenty button constants with the original's numeric values in RGSS1 and RGSS2, and with
the Symbols of their names in RGSS3.

#### Scenario: Every listed function exists
- **WHEN** each entry of the method list for the three modules is checked with `method`
- **THEN** the function exists and its arity equals the listed arity

#### Scenario: Button constants
- **WHEN** `[Input::DOWN, Input::LEFT, Input::RIGHT, Input::UP, Input::A, Input::C, Input::F9]` is evaluated
- **THEN** the result is `[2, 4, 6, 8, 11, 13, 29]`

#### Scenario: Button constants of RGSS3, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Input::DOWN, Input::LEFT, Input::RIGHT, Input::UP, Input::A, Input::C, Input::F9, Input::DOWN == :DOWN]`
- **THEN** the result is `[:DOWN, :LEFT, :RIGHT, :UP, :A, :C, :F9, true]`

### Requirement: Input
In RGSS1 and RGSS2 `Input.update` SHALL latch the most recent input snapshot received from
the host and advance the state that `press?`, `trigger?`, `repeat?`, `dir4` and `dir8`
report, as `core-input` defines. A button number that is not one of the twenty constants
SHALL be reported as not pressed. In RGSS3 `Input` SHALL follow the requirements of `Input`
of RGSS3.

#### Scenario: Trigger
- **WHEN** the host sends a snapshot with C held, and the script calls `Input.update` twice
- **THEN** `Input.trigger?(Input::C)` is `true` after the first call and `false` after the second, and `Input.press?(Input::C)` is `true` after both

#### Scenario: Unknown button
- **WHEN** `Input.press?(99)` is evaluated
- **THEN** the result is `false`

## ADDED Requirements

### Requirement: Arguments of Input of RGSS3
In a game of RGSS3 `press?`, `trigger?` and `repeat?` SHALL take exactly one argument and
SHALL NOT raise for its class: a Symbol of a button's name, exactly as the constant is
named, or the Integer of that button in RGSS2 SHALL name the button, and any other value
SHALL give `false`. No method of the argument SHALL be called.

#### Scenario: Names and numbers, measured on the original
- **WHEN** in a game of RGSS3 each of the twenty buttons is held alone, a frame is shown and `Input.update` runs, and `press?`, `trigger?` and `repeat?` are asked for the button's constant, for its Symbol and for every Integer from -300 to 300
- **THEN** the constant and the Symbol give `true` for the three, and of the Integers exactly the button's number of RGSS2 gives `true`, for each of the three

#### Scenario: Anything else, measured on the original
- **WHEN** a game of RGSS3 with every button held, a frame shown and `Input.update` run evaluates `press?`, `trigger?` and `repeat?` of `:down`, `:Down`, `:NOPE`, `:""`, `"DOWN"`, `2.0`, `2.9`, `258`, `2**31 - 1`, `2**31`, `2**40`, `nil`, `true`, `false`, `[2]`, `Object.new` and `Input`
- **THEN** every answer is `false` and nothing is raised

#### Scenario: Count, measured on the original
- **WHEN** a game of RGSS3 evaluates `Input.press?`, `Input.trigger?(:A, :B)` and `Input.repeat?`
- **THEN** each raises `ArgumentError`

#### Scenario: The call of the standard scripts
- **WHEN** a game of RGSS3 evaluates `Input.update; Input.repeat?(:DOWN)` with nothing held
- **THEN** the result is `false` and nothing is raised

### Requirement: When Input of RGSS3 sees a key
In a game of RGSS3 the input state SHALL follow the rules of RGSS3 of `core-input`, and the
guest SHALL mark as shown every frame it shows: `Graphics.update` and each frame of `wait`,
`fadeout`, `fadein` and `transition`, and nothing else. An `Input.update` SHALL see what had
arrived when a frame was last shown.

#### Scenario: An update alone sees nothing new, measured on the original
- **WHEN** a game of RGSS3 gets a snapshot with C held before it does anything, calls `Input.update` twelve times, then `Graphics.update` and `Input.update`
- **THEN** `Input.press?(:C)` is `false` after each of the twelve and `true` after the last, and `Input.trigger?(:C)` is `true` after the last

#### Scenario: Seen after the next frame, measured on the original
- **WHEN** a game of RGSS3 gets a snapshot with DOWN held while its first `Graphics.update` waits for the tick, and then calls `Input.update`
- **THEN** `Input.press?(:DOWN)` and `Input.trigger?(:DOWN)` are `true`

#### Scenario: Calls that show no frame, measured on the original
- **WHEN** a game of RGSS3 gets a snapshot with C held before it does anything and evaluates `Graphics.frame_reset; Graphics.brightness = 255; Graphics.snap_to_bitmap.dispose; Graphics.freeze; Graphics.transition(0); Graphics.wait(0); Graphics.fadeout(0); Graphics.resize_screen(544, 416); Input.update; Input.press?(:C)`
- **THEN** the result is `false`

#### Scenario: Frames of waits and transitions, measured on the original
- **WHEN** a game of RGSS3 gets a snapshot with C held while the third frame of `Graphics.wait(3)` waits for its tick and then calls `Input.update`; and another game gets it while the second frame of `Graphics.freeze; Graphics.transition(2)` waits and then calls `Input.update`
- **THEN** after each `Input.update` `Input.press?(:C)` and `Input.trigger?(:C)` are `true`

### Requirement: Repeat and directions of Input of RGSS3
In a game of RGSS3 `trigger?`, `repeat?`, `dir4` and `dir8` SHALL follow "Input of RGSS3"
of `core-input` as measured: one button repeats at a time, at the 1st, 24th, 30th and
every 6th update after, and `dir4` resolves two axes by the axis held alone last.

#### Scenario: Repeat, measured on the original
- **WHEN** a game of RGSS3 holds DOWN from its start and runs 36 times `Graphics.update; Input.update`, collecting the numbers of the updates at which `Input.repeat?(:DOWN)` is `true`
- **THEN** they are `[1, 24, 30, 36]`

#### Scenario: The newest button repeats, measured on the original
- **WHEN** a game of RGSS3 holds DOWN from its start, holds DOWN and RIGHT from the frame of the 11th update on, and runs 40 updates, each after a `Graphics.update`
- **THEN** `Input.repeat?(:RIGHT)` is `true` at the 11th update, and `Input.repeat?(:DOWN)` is `false` at every update from the 11th to the 40th

#### Scenario: A and SHIFT, measured on the original
- **WHEN** a game of RGSS3 holds A and SHIFT from its start and runs 36 updates, each after a `Graphics.update`
- **THEN** `Input.repeat?(:SHIFT)` is `true` at the 1st, 24th, 30th and 36th, and `Input.repeat?(:A)` at none

#### Scenario: Directions, measured on the original
- **WHEN** a game of RGSS3 holds LEFT and UP as its very first input and asks `[Input.dir4, Input.dir8]` after a frame and an update; and, in another game, holds DOWN, then DOWN and RIGHT at a later frame; and holds LEFT and RIGHT
- **THEN** the answers are `[4, 7]`, `[6, 3]` after RIGHT was added, and `[0, 0]`

### Requirement: Method surface of Input in RGSS3
In a game of RGSS3 the public module functions of `Input` SHALL be exactly those of the
committed method list of RGSS3, with their arities, and its constants exactly the twenty of
the list, and the test of the list SHALL name `Input` as bound.

#### Scenario: Functions and constants, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Input.singleton_methods(false).sort, Input.constants.sort]`
- **THEN** the result is `[[:dir4, :dir8, :press?, :repeat?, :trigger?, :update], [:A, :ALT, :B, :C, :CTRL, :DOWN, :F5, :F6, :F7, :F8, :F9, :L, :LEFT, :R, :RIGHT, :SHIFT, :UP, :X, :Y, :Z]]`

#### Scenario: Bound
- **WHEN** the test of the list of RGSS3 runs after this change
- **THEN** it checks `Input` beside the classes and modules bound before, and passes

### Requirement: Input of RGSS1 and RGSS2 is unchanged by RGSS3
`Input` of a game of RGSS1 or RGSS2 SHALL NOT change with that of RGSS3: marking shown
frames SHALL change nothing there, and every test of the two binding packages that passed
before SHALL pass under its name with its expected values unedited, over both interpreters.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkxp-binding` and of `vkxp-binding-rgss3` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: XP and VX
- **WHEN** a game of RGSS2 gets a snapshot with C held before it does anything and evaluates `Input.update; [Input::DOWN, Input::F9, Input.press?(Input::C)]`
- **THEN** the result is `[2, 29, true]`, over both interpreters
