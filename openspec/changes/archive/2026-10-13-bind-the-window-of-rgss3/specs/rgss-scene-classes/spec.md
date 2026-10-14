## ADDED Requirements

### Requirement: Window construction in RGSS3
In RGSS3 `Window.new` SHALL take up to four Integers, x, y, width and height, each 0 when
not given, truncated, negative values kept, and no viewport: "Construction" holds for
`Window.new` in RGSS1 and RGSS2 only. A fifth argument SHALL raise `ArgumentError` and a
`Viewport` `TypeError`. The window SHALL be the window of RGSS3 of the scene with its
defaults, and SHALL have a contents bitmap of 1 by 1 of its own.

#### Scenario: Without arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `w = Window.new; [w.x, w.y, w.width, w.height, w.z, w.padding, w.padding_bottom, w.back_opacity, w.openness, w.viewport, [w.contents.width, w.contents.height]]`
- **THEN** the result is `[0, 0, 0, 0, 100, 8, 8, 192, 255, nil, [1, 1]]`

#### Scenario: The other defaults, measured on the original
- **WHEN** a game of RGSS3 evaluates `w = Window.new; [w.ox, w.oy, w.opacity, w.contents_opacity, w.visible, w.active, w.pause, w.windowskin, w.cursor_rect.to_s, w.arrows_visible, w.tone.to_s, w.open?, w.close?]`
- **THEN** the result is `[0, 0, 255, 255, true, true, false, nil, "(0, 0, 0, 0)", true, "(0.000000, 0.000000, 0.000000, 0.000000)", true, false]`

#### Scenario: One to four arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `[w.x, w.y, w.width, w.height]` for `Window.new(1)`, `Window.new(1, 2)`, `Window.new(1, 2, 3)`, `Window.new(10, 20, 30, 40)`, `Window.new(10.7, 20.2, 30.9, 40.5)` and `Window.new(-10, -20, -30, -40)`
- **THEN** the results are `[1, 0, 0, 0]`, `[1, 2, 0, 0]`, `[1, 2, 3, 0]`, `[10, 20, 30, 40]`, `[10, 20, 30, 40]` and `[-10, -20, -30, -40]`, and each of these windows has z 100 and contents of 1 by 1

#### Scenario: No viewport argument, measured on the original
- **WHEN** a game of RGSS3 evaluates, with `v = Viewport.new(0, 0, 10, 10)`, `Window.new(v)`, `Window.new(nil, 0, 0, 0)`, `Window.new(10, 20, 30, 40, 5)`, `Window.new(v, 1, 2, 3, 4)` and `Window.new(1, 2, 3, 4, v)`
- **THEN** the first raises `TypeError` with `can't convert Viewport into Integer`, the second `TypeError` with `no implicit conversion from nil to integer`, and the last three `ArgumentError` with `wrong number of arguments (4 for 5)`

#### Scenario: Contents, measured on the original
- **WHEN** a game of RGSS3 evaluates `a = Window.new; b = Window.new(5, 6, 100, 80); [a.contents.equal?(b.contents), a.contents.equal?(a.contents), b.contents.width, b.contents.height, a.contents.get_pixel(0, 0).to_s, a.contents.disposed?]`
- **THEN** the result is `[false, true, 1, 1, "(0.000000, 0.000000, 0.000000, 0.000000)", false]`

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `Window.new(Viewport.new(0, 0, 10, 10)).viewport.nil?` and `Window.new(1, 2, 3, 4)`
- **THEN** the first is `false` and the second raises `ArgumentError`

### Requirement: Window methods added in RGSS3
In RGSS3 `Window` SHALL define `padding` and `padding_bottom`, Integers truncated and not
limited, `padding=` setting both; `arrows_visible`, read as true or false;
`move(x, y, width, height)`, which assigns the four and returns the window; and `open?` and
`close?`, true at openness 255 and at openness 0 alone. Each SHALL reach the window of the
scene. In RGSS1 and RGSS2 none of them SHALL be defined.

#### Scenario: Paddings, measured on the original
- **WHEN** a game of RGSS3 evaluates, on one window, `[w.padding, w.padding_bottom]` after each of `w.padding = 20`, `w.padding_bottom = 5`, `w.padding = 7`, and after `w.padding_bottom = 30; w.padding = 7`
- **THEN** the results are `[20, 20]`, `[20, 5]`, `[7, 7]` and `[7, 7]`

#### Scenario: Values of the paddings, measured on the original
- **WHEN** a game of RGSS3 assigns -5, 100000, 10.7 and -10.7 to `padding` and to `padding_bottom`, reading the property after each, and then `nil` to each
- **THEN** the values read are -5, 100000, 10 and -10 for both, and `nil` raises `TypeError` with `no implicit conversion from nil to integer`

#### Scenario: Arrows flag, measured on the original
- **WHEN** a game of RGSS3 assigns `nil`, `0`, `1` and `"x"` to `arrows_visible`, reading it after each
- **THEN** the values are `false`, `true`, `true` and `true`

#### Scenario: Move, measured on the original
- **WHEN** a game of RGSS3 evaluates `[w.move(1, 2, 3, 4).equal?(w), w.x, w.y, w.width, w.height]`, the same with `move(-1, -2, -3, -4)` and with `move(1.9, 2.9, 3.9, 4.9)`, then `w.move(1, 2, 3)` and `w.move(nil, 2, 3, 4)`
- **THEN** the results are `[true, 1, 2, 3, 4]`, `[true, -1, -2, -3, -4]` and `[true, 1, 2, 3, 4]`, then an `ArgumentError` with `wrong number of arguments(3 for 4)` and a `TypeError` with `no implicit conversion from nil to integer`

#### Scenario: Open and closed, measured on the original
- **WHEN** a game of RGSS3 sets `openness` to 255, 254, 128, 1 and 0, evaluating `[w.open?, w.close?]` after each
- **THEN** the results are `[true, false]`, `[false, false]`, `[false, false]`, `[false, false]` and `[false, true]`

#### Scenario: Paddings move the contents
- **WHEN** a game of RGSS3 gives a window of 100 by 100 at (0, 0) without a skin the green contents of 50 by 50 and reads the frame, then assigns `padding = 20` and reads it, then `padding_bottom = 50` and reads it
- **THEN** in the first frame (8, 8) is green and (7, 7) black; in the second (8, 8) is black and (20, 20) and (30, 60) are green; in the third (30, 49) is green and (30, 50) black, and `w.padding` is still 20

#### Scenario: Arrows hidden and shown
- **WHEN** a game of RGSS3 gives a window of 100 by 100 at (0, 0) the arrow skin and transparent contents of 200 by 200, reads the pixel (50, 92), assigns `arrows_visible = false` and reads it, assigns `arrows_visible = true` and reads it
- **THEN** the second pixel differs from the first and the third is the same as the first

#### Scenario: Move reaches the scene
- **WHEN** a game of RGSS3 gives a window made by `Window.new(0, 0, 20, 20)` the grey skin and `back_opacity = 255`, and then calls `move(40, 30, 68, 68)`
- **THEN** the pixel (70, 70) is black before the call and not black after it, and the pixel (10, 10) is not black before and black after

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `w = Window.new; [:padding, :padding_bottom, :arrows_visible, :move, :open?, :close?, :tone].map { |name| w.respond_to?(name) }`
- **THEN** every element is `false`

### Requirement: Window tone in RGSS3
In RGSS3 `Window#tone` SHALL return the window's own `Tone`, the same object at every call.
`tone=` SHALL take a `Tone` only, copy its values into that object and leave the given one
unconnected; anything else SHALL raise `TypeError`. A change of the object through its own
methods SHALL change the window as an assignment does, and the frame drawn next SHALL show
it on the background.

#### Scenario: The object, measured on the original
- **WHEN** a game of RGSS3 evaluates `w.tone.equal?(w.tone)`, then `t = Tone.new(1, 2, 3, 4); w.tone = t; r = w.tone; [r.red, r.green, r.blue, r.gray, r.equal?(t)]`, and the same with `Tone.new(300, -300, 255.5, 300)`
- **THEN** the results are `true`, `[1.0, 2.0, 3.0, 4.0, false]` and `[255.0, -255.0, 255.0, 255.0, false]`

#### Scenario: Not a tone, measured on the original
- **WHEN** a game of RGSS3 evaluates `w.tone = nil`, `w.tone = 5` and `w.tone = Color.new(1, 2, 3, 4)`
- **THEN** each raises `TypeError`, with the messages `can't convert NilClass into Tone`, `can't convert Fixnum into Tone` and `can't convert Color into Tone`, and `w.tone` is as it was

#### Scenario: Changed through its own writer, measured on the original
- **WHEN** a game of RGSS3 evaluates `t = w.tone; t.red = 50; w.tone.red`
- **THEN** the result is `50.0`

#### Scenario: Seen in the frame
- **WHEN** a game of RGSS3 gives a window of 68 by 68 at (0, 0) the grey skin and `back_opacity = 255`, and reads the pixel (30, 40) at once, after `w.tone.set(Tone.new(50, -30, 0, 0))`, after `w.tone.red = -255`, and after `w.tone = Tone.new`
- **THEN** the first pixel is the grey of the skin, (100, 100, 100); the second has 50 more red and 30 less green than the first and its blue; the third has red 0 and the green and blue of the second; the fourth equals the first

#### Scenario: The given object stays unconnected
- **WHEN** a game of RGSS3 evaluates `t = Tone.new(1, 2, 3, 4); w.tone = t; t.red = 99; w.tone.red`
- **THEN** the result is `1.0`

### Requirement: Assignments of a window in RGSS3 are calls of the scene
In RGSS3 every assignment of a property of a window, `move` and `update` SHALL be handed to
the window of the scene as one call, also when the value is the one the window has, and so
SHALL every change of `cursor_rect` through the methods of its `Rect`, also to equal
values. What a call recomputes is the scene's. The properties RGSS2 has SHALL take and
return values as in RGSS2.

#### Scenario: The cursor is placed through the Rect, measured on the original
- **WHEN** a game of RGSS3 gives a window of 40 by 40 at (0, 0) the cursor skin and `cursor_rect = Rect.new(0, 0, 100, 100)`, reads the pixel (20, 20) as the colour of the cursor, assigns `width = 120` and `height = 120` and reads the pixel (50, 50), then evaluates `w.cursor_rect.x = 0` and reads it again
- **THEN** the pixel (50, 50) differs from the colour of the cursor after the two assignments and has it after the change through the `Rect`

#### Scenario: Every way of changing the Rect, measured on the original
- **WHEN** the window of that scenario, grown to 120 by 120 with its cursor still cut to the old size, gets in turn, each on a window prepared anew, `w.cursor_rect.set(0, 0, 100, 100)`, `w.cursor_rect.set(Rect.new(0, 0, 100, 100))`, `w.cursor_rect = Rect.new(0, 0, 100, 100)`, `w.ox = 0` and `w.padding = 8`
- **THEN** after each of them the pixel (50, 50) has the colour of the cursor

#### Scenario: Calls that do not place it, measured on the original
- **WHEN** that window gets `w.move(0, 0, 120, 120)`, `w.x = 0`, `w.update` and `w.z = 100` in place of them
- **THEN** the pixel (50, 50) still differs from the colour of the cursor

#### Scenario: The cursor rectangle is the window's own, measured on the original
- **WHEN** a game of RGSS3 evaluates `r = Rect.new(1, 2, 3, 4); w.cursor_rect = r; c = w.cursor_rect; [c.x, c.y, c.width, c.height, c.equal?(r), c.equal?(w.cursor_rect)]`, then `w.cursor_rect = nil` and `w.cursor_rect = 5`
- **THEN** the result is `[1, 2, 3, 4, false, true]`, and the two others raise `TypeError`

#### Scenario: Limits of the opacities, measured on the original
- **WHEN** a game of RGSS3 assigns 300, 256, -5, 100.7, 254.9 and -0.5 to each of `opacity`, `back_opacity`, `contents_opacity` and `openness`, reading it after each
- **THEN** the values are 255, 255, 0, 100, 254 and 0 for each of the four

#### Scenario: Positions and flags, measured on the original
- **WHEN** a game of RGSS3 assigns -5, 100000, 10.7 and -10.7 to each of `x`, `y`, `z`, `width`, `height`, `ox` and `oy`, reading it after each, then `nil` to each, and `nil`, `0` and `"x"` to each of `visible`, `active` and `pause`
- **THEN** the numbers read are -5, 100000, 10 and -10, `nil` raises `TypeError` with `no implicit conversion from nil to integer`, and the flags read `false`, `true` and `true`

#### Scenario: Bitmaps and update, measured on the original
- **WHEN** a game of RGSS3 evaluates `b = Bitmap.new(8, 8); w.windowskin = b; w.contents = b; [w.windowskin.equal?(b), w.contents.equal?(b)]`, `w.windowskin = nil; w.contents = nil; [w.windowskin, w.contents]`, `w.contents = 5`, `w.windowskin = 5` and `w.update`
- **THEN** the results are `[true, true]`, `[nil, nil]`, two `TypeError`s and `nil`

#### Scenario: Viewport, measured on the original
- **WHEN** a game of RGSS3 evaluates `v = Viewport.new(0, 0, 10, 10); w.viewport = v; a = w.viewport.equal?(v); w.viewport = nil; [a, w.viewport]` and `w.viewport = 5`
- **THEN** the result is `[true, nil]`, and the second raises `TypeError` with `can't convert Fixnum into Viewport`

### Requirement: Disposal and disposed bitmaps of a window in RGSS3
In RGSS3 every method of a disposed window but `dispose` and `disposed?` SHALL raise
`RGSSError` with the message `disposed window`, and `dispose` SHALL return `nil` each time
and SHALL NOT dispose the window's bitmaps. A window whose skin or contents bitmap is
disposed SHALL raise nothing, SHALL go on returning that object, and SHALL be drawn as the
scene draws a window of RGSS3 without it.

#### Scenario: A disposed window, measured on the original
- **WHEN** a game of RGSS3 evaluates `c = Bitmap.new(10, 10); w.contents = c; w.dispose`, and then `[c.disposed?, w.disposed?, w.dispose]`, each of `x`, `width`, `contents`, `windowskin`, `tone`, `cursor_rect`, `open?`, `close?`, `update`, `padding`, `viewport` and `visible`, `w.x = 1` and `w.move(1, 2, 3, 4)`
- **THEN** the result is `[false, true, nil]`, and every one of the other fourteen raises `RGSSError` with the message `disposed window`

#### Scenario: A disposed skin, measured on the original
- **WHEN** a game of RGSS3 gives a window of 100 by 100 at (0, 0) the grey skin and `back_opacity = 255`, reads the pixel (50, 50), disposes the skin, reads it, then evaluates `w.update; w.width = 100; w.ox = 0; w.cursor_rect = Rect.new(0, 0, 8, 8); w.contents_opacity = 100; w.windowskin.equal?(skin)` and reads it again
- **THEN** the first pixel is not black, the second and the third are black, nothing is raised and the value is `true`

#### Scenario: Disposed contents, measured on the original
- **WHEN** a game of RGSS3 gives a window of 100 by 100 at (0, 0) without a skin the green contents, reads the pixel (10, 10), disposes the contents bitmap, evaluates `w.update; w.contents_opacity = 100` and reads the pixel again
- **THEN** the first pixel is green, the second is black, and nothing is raised

### Requirement: Calls at which the original of RGSS3 faults, through the class
Where the original runtime of RGSS3 ends with a page fault, a script of a game of RGSS3
SHALL go on: the assignments that place the cursor in a window on which `windowskin=` was
never called SHALL return and take effect, and so SHALL the assignment of contents after a
skin of 8 by 8 and a window or cursor rectangle of 100000 by 100000.

#### Scenario: No skin was ever assigned
- **WHEN** a game of RGSS3 evaluates `w = Window.new(0, 0, 96, 96); w.padding = 8; w.padding_bottom = 4; w.ox = 0; w.oy = 0; w.cursor_rect = Rect.new(0, 0, 0, 0); w.cursor_rect.set(0, 0, 0, 0); w.cursor_rect.x = 0; w.cursor_rect.empty; [w.padding, w.padding_bottom, w.cursor_rect.to_s]` and reads the pixel (48, 48)
- **THEN** the result is `[8, 4, "(0, 0, 0, 0)"]`, nothing is raised and the pixel is black

#### Scenario: A small skin and contents
- **WHEN** a game of RGSS3 evaluates `w = Window.new; w.windowskin = Bitmap.new(8, 8); w.contents = Bitmap.new(8, 8); w.contents.width`
- **THEN** the result is 8 and nothing is raised

#### Scenario: Sizes
- **WHEN** a game of RGSS3 evaluates `w = Window.new(0, 0, 100000, 100000); w.cursor_rect = Rect.new(0, 0, 100000, 100000); [w.width, w.cursor_rect.width]` and reads the pixel (0, 0)
- **THEN** the result is `[100000, 100000]` and nothing is raised

### Requirement: Method surface of Window in RGSS3
In RGSS3 `Window` SHALL define every method of the committed method list of RGSS3 with its
arity and no public instance method beside them: `stretch` and `stretch=` of RGSS1 SHALL
NOT be defined. The test of that list SHALL check `Window` beside the classes it checked
before.

#### Scenario: Every listed method exists
- **WHEN** the test of the list of RGSS3 runs after this change
- **THEN** the 53 entries of `Window` exist with their arities, among them `initialize` with -1, `move` with 4 and `open?`, `close?`, `padding`, `padding_bottom`, `arrows_visible`, `tone` and `viewport` with 0, and `Window` is in the list of bound classes

#### Scenario: Nothing beside the list
- **WHEN** `Window.public_instance_methods(false)` is compared with the entries of the list in a game of RGSS3, and `[Window.method_defined?(:stretch), Window.method_defined?(:stretch=)]` is evaluated
- **THEN** the class has no method that the list lacks, and the value is `[false, false]`

#### Scenario: No copies
- **WHEN** a game of RGSS3 evaluates `Window.new.clone` and `Window.new.dup`
- **THEN** each raises `TypeError`

### Requirement: The window of RGSS1 and RGSS2 is unchanged by RGSS3
The class `Window` of a game of RGSS1 or RGSS2 SHALL NOT change with that of RGSS3: every
test of the two binding packages that passed before SHALL pass under its name with its
expected values unedited, over both interpreters. The tests of RGSS3 SHALL be a file of the
package over Ruby 1.9.2 alone, and the scenarios named "XP and VX" SHALL be held by tests of
the shared files.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkxp-binding` and of `vkxp-binding-rgss3` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: Existing tests
- **WHEN** the difference of `crates/vkxp-binding/tests` and `crates/vkxp-binding-rgss3/tests` against the start of the change is read
- **THEN** it holds the new file of RGSS3, one new test in each of `tests/scene.rs` and `tests/scene_rgss2.rs` of `vkxp-binding`, and `Window` added to the bound classes of `arity_rgss3.rs`; no test that existed, no expected value and no line of the allow-list changed

#### Scenario: Other crates
- **WHEN** the difference of `crates` against the start of the change is taken
- **THEN** it names files of `crates/vkxp-binding` and `crates/vkxp-binding-rgss3` only
