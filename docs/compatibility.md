# Compatibility

vkxp aims at the behaviour of the original runtimes of RPG Maker XP, RGSS 1.04, and of RPG
Maker VX, RGSS2: both are supported. Games that use the default scripts or common
community scripts are meant to run unmodified. VX Ace is experimental: its scripts run, and
its games do not reach their title screen yet ([RPG Maker VX Ace](#rpg-maker-vx-ace)). What
a game of VX has differently is under [RPG Maker VX](#rpg-maker-vx); everything else below is
about XP.

## What is exact

Frames are compared with those of the original runtime, which
[`cargo xtask oracle`](development.md#comparing-with-the-original-runtime) runs under Wine
beside vkxp, with the same input and the same seed. On the seven community-made games that
serve as the regression corpus, the frames compared are pixel-identical except for text
glyphs, whose rasterisation differs from that of Windows GDI.

Many rules were measured on the original runtime rather than taken from a description of it:
the arithmetic of the bitmap operations and of blending, the layout of text, the timing of
frames and more. The specifications under [`openspec/specs`](../openspec/specs) record them,
and `vkxp-pixel` lists for each of its functions whether its formula was measured.

The CPU reference compositor and the Vulkan backend must produce byte-identical frames; the
tests render every message stream with both.

## Known differences

Behaviour that is not yet checked against the original runtime is marked `PLACEHOLDER` in the
source. What remains under `crates/`:

- **Text metrics.** The original runtime was measured under Wine, so a few rules are those of
  Wine's GDI and may not be those of Windows: the choice of the pixels per em for a size, the
  rounding when text is squeezed into a narrow rectangle, and the strength of a synthesised
  bold and the slant of a synthesised italic. Most built-in line boxes of the Windows fonts
  are from memory of their metric tables.
- **Text glyphs.** Glyphs are not rasterised as Windows GDI rasterises them, so the pixels of
  text differ while its layout and advance widths are kept. A character the selected font
  lacks is drawn from another font; the original, measured under Wine, draws the missing
  glyph.
- **Win32API shims.** The toggle bits that `GetKeyState` and `GetKeyboardState` report
  start at 0: the lock keys of the machine start off, whatever their state is.
  `mouse_event` presses and releases buttons but does not move the pointer or turn the
  wheel. A key injected with `keybd_event` or `mouse_event` is seen one `Input.update`
  later than on Windows when a script injects and updates in the same frame. These rules
  and those of `ShowCursor` and `SetKeyboardState` follow the documentation of Windows and
  were not measured on the original.

The level of a transition mask pixel follows the original's rule, which was measured with PNG
masks of 24 and 32 bits; masks with a palette, in grey of 8 or 16 bits, and JPEG masks were
not measured.

Besides these, WMA sounds are decoded by the `ffmpeg` program, which has to be installed:
without it they stay silent. vkxp runs on Linux, and x86_64 is the supported architecture;
the job `gate-aarch64` of the [CI workflow](../.github/workflows/ci.yml) shows what works on
aarch64.

## RPG Maker VX

Games of RPG Maker VX (RGSS2) are supported. Two games were compared with the original
runtime, Visions & Voices and The Witch's House; the rows V0 to V3 of the milestone table in
[decisions.md](decisions.md#milestones) say which frames and which save, and what the
comparison does not cover.

Of the module `Graphics`, a game of VX has what follows, each measured on the original runtime of VX (`RGSS202E.dll`,
versions 2.0.0.1 and 2.0.2.1) under Wine:

- The frame is 544 by 416 pixels and the frame rate starts at 60. `Graphics.width` and
  `Graphics.height` return the size of the frame.
- `Graphics.brightness` and `Graphics.brightness=`, limited to 0 to 255. Every frame is
  darkened by it.
- `Graphics.wait(n)` shows n frames, none for n below 1.
- `Graphics.fadeout(n)` and `Graphics.fadein(n)` show n frames at the brightness
  `b0 - b0 * k / n` and `b0 + (255 - b0) * k / n` and leave 0 and 255, also for n = 0.
- `Graphics.transition` defaults to 10 frames. With nothing frozen it shows nothing and
  returns at once. After `Graphics.freeze` it sets the brightness to 255 and steps the
  transition by the weights of RGSS2.
- `Graphics.snap_to_bitmap` returns a bitmap of the size of the frame with the scene as its
  objects are at that moment, darkened by the brightness, with an alpha of 255 everywhere.
  It shows no frame. The scene is drawn on the CPU for it.
- `wait`, `fadeout`, `fadein` and `transition` do not call a `Graphics.update` that a
  script redefined.
- `Graphics.resize_screen(width, height)` sets the size of the frame, the width limited to
  1 to 640 and the height to 1 to 480, and shows no frame. Elements keep their coordinates.
  The window on the desktop keeps its size and shows the frame scaled into it.

Not there or not measured yet:

- What the window of the original does when the size of the frame changes, and a reset
  with F12 after a resize: the size set stays.
- A transition with a picture reads its arguments as in XP: the default vagueness and the
  thresholds of RGSS2 are not measured.
- A fade with a count below 0 is taken as one of 0 frames, and a transition with nothing
  frozen does not look for its picture, so a missing file raises nothing. Neither case was
  measured.

In a game of XP none of `brightness`, `brightness=`, `wait`, `fadeout`, `fadein`,
`snap_to_bitmap`, `resize_screen`, `width` and `height` is defined, as on the original.

### The window of VX

In a game of VX a `Window` is drawn by the rules of RGSS2, measured on the same two files.
The rules of that measurement reproduced 490 of its 496 pictures without a wrong pixel; the
six others hold a skin smaller than 64 pixels, and the rule for their background was found by
the window measurement of VX Ace of 2026-10-13 and is followed. The tests compare a selection
of them.

- The skin is read as 128 by 128: the background (0, 0, 64, 64) is stretched to the window
  less a margin of 2 and the pattern (0, 64, 64, 64) is tiled over it; the frame is built
  from the corners and edges at (64, 0, 64, 64); cursor, pause sign and scroll arrows come
  from their places in the skin.
- Of a skin narrower or lower than 64 pixels the background block is cut and its target
  shortened, as the original does: a skin of 32 by 32 fills 35 columns and 62 rows of a
  background of 72 by 128, and a skin of 1 by 1 none.
- A window is one element at its `z`: contents, cursor, arrows and pause sign have no z of
  their own, so a sprite above the window's `z` covers all of it.
- `Window#openness` and `openness=`, limited to 0 to 255. Below 255 the window shows part of
  its rows around its middle, without contents, cursor, arrows and pause sign.
- The cursor blinks over 40 updates and the pause sign changes its image every 16.
- Background, frame and cursor are bitmaps of the window and do not follow pixels drawn into
  the skin later. Background and frame are built when another `windowskin` object is
  assigned and at every assignment of `width` and `height`, also of the value they have. The
  cursor is built when another `windowskin` object is assigned and when the width or height
  of the cursor rectangle changes; an assignment of `width` or `height` does not build it.
  Assigning the skin the window already has builds none of them.
- Whether the contents show and which arrows are drawn is decided by `update` and by the
  assignments the original decides them in, so the contents of a window that was updated or
  given contents while invisible appear at its next `update`.

Not measured, or not there yet:

- The cursor from a skin that cuts the cursor block (a skin less than 96 pixels wide or
  high) was not measured on VX, nor was an arrow or a pause sign that the skin cuts to an
  odd width: vkxp draws them from the skin with transparent pixels where it has none.
- A skin or a contents bitmap that is disposed while the window has it: vkxp goes on
  drawing the background, frame and cursor it built, and draws no contents, arrows or pause
  sign from a disposed bitmap.
- Windows larger than 600 pixels and origins beyond 1000 were not measured.

In a game of XP the window is drawn as before and has no `openness`.

### The sprite of VX

In a game of VX a `Sprite` is drawn by the rules of RGSS2, measured on the same two files:
325 frames and a held-out set of random scenes without a wrong pixel. The blit, the blend
types, tone, colour, flash, the stretch and the rotation are those of XP. What differs:

- A sprite that is not turned starts at `x - ox * zoom_x` rounded toward zero and ends at
  the floor of that start plus `width * zoom_x`, in double precision and in the coordinates
  of its viewport; likewise in y. XP rounds down, in single precision. A sprite 90 wide at a
  zoom of 0.7 is 62 pixels wide at x 0 and 63 at x 272.
- `Sprite#bush_opacity`, 128 at first and kept as it is written. The bush rows are drawn at
  the opacity `opacity * bush_opacity / 255`, and not at all when that is below 0 or above
  255. They get no tone, no colour and no flash colour, and on a sprite that is not turned
  no mirror. A bush depth beyond the height of the sprite stretches rows from above the
  source rectangle.
- `Sprite#wave_amp`, `wave_length`, `wave_speed` and `wave_phase` (0, 180, 360 and 0.0 at
  first; the phase is kept modulo 360). `Sprite#update` adds `wave_speed / wave_length` to
  the phase, also at an amplitude of 0 and when the sprite is invisible. A sprite that is
  not turned and has a positive amplitude has every screen row moved horizontally, by the
  same number of pixels over a band of 8 screen rows. A negative amplitude squeezes the
  sprite by twice the amplitude instead.
- A mirrored sprite that is not stretched and whose source rectangle leaves the bitmap is
  cut first and mirrored inside what is left.
- `bitmap=` with the bitmap the sprite has already keeps `src_rect`.

Not measured, or different from the original:

- A wave on a turned sprite: the original draws shifted bands that no rule was found for;
  vkxp draws the turned sprite without its wave.
- A wave on a sprite that is stretched in x and whose source rectangle leaves the bitmap:
  the columns can be one pixel off.
- A negative `wave_length`. With a `wave_length` of 0 `Sprite#update` of the original does
  not return; vkxp leaves the phase and draws the sprite without a wave.

In a game of XP a sprite is drawn as before and has none of `bush_opacity`, `wave_amp`,
`wave_length`, `wave_speed` and `wave_phase`.

### Order, viewports, planes and input of VX

Measured on the same two versions of the original runtime and reproduced without a wrong
pixel or value:

- Elements are drawn in ascending order of (`z`, `y`, creation). `y` is that of a sprite and
  0 for a window, a plane, a viewport and the planes of a tilemap; a viewport takes its place
  by its own `z` and creation.
- A flashing viewport applies its tone, its colour and then the flash colour. A flash
  without a colour draws nothing of the viewport and applies neither tone nor colour.
- The tile of a plane is sized in double precision.
- `blend_type=` of a sprite and a plane raises `ArgumentError` for a value other than 0, 1
  and 2.
- `Input` repeats a held button after 24 updates and then every 6. The keys are those of VX
  (`docs/running.md`).
- The vagueness of a masked transition defaults to 40; the thresholds are those of XP,
  stepped from 1.

Not measured: gamepads and the F1 dialog of VX; a negative vagueness is taken as 0.

Known difference: `Viewport#dispose` disposes the sprites, planes, windows and tilemaps of
the viewport, as in XP, where the original of VX keeps them alive and their `viewport`
returns the disposed viewport.

### Bitmap and Font of VX

Of `Bitmap` and `Font`, a game of VX has what follows, measured on the same two versions of
the original runtime. Every sample of the measurement was reproduced by the rule given.

- `Bitmap#blur`: one pass of a 3 by 3 box, each of red, green, blue and alpha the sum of
  nine divided by 9, the edge pixel repeated beyond the edges. A transparent pixel counts
  with the colour it holds. A bitmap narrower or lower than 3 pixels is left as it is.
- `Bitmap#radial_blur(angle, division)`: the mean of `division` samples turned about the
  centre over `angle` degrees, nearest pixels, mirrored at the edges. The angle is limited
  to 0 to 360 and the division to 2 to 100.
- `Bitmap#gradient_fill_rect` with a `Rect` or four integers, two colours and a flag for a
  vertical gradient. It overwrites, alpha included. With seven arguments the gradient is
  vertical whatever the seventh is, `false` and `nil` included, as on the original.
- `Bitmap#clear_rect` with a `Rect` or four integers.
- `Bitmap.new(width, height)` raises `RGSSError` above 4194304 pixels. `Bitmap.new(path)`
  raises `RGSSError` for a file that is empty or holds no picture.
- The font defaults are the names "Verdana", "Arial" and "Courier New", size 20 and a
  shadow. `Font#shadow` and `Font.default_shadow` take any object and read true or false.
- `Bitmap#draw_text` draws the shadow first: the text in black with the alpha of the font
  colour, one pixel to the right and one down, clipped by the bitmap and not by the
  rectangle. `Bitmap#text_size` does not count it.
- Text wider than its rectangle is squeezed in five steps, to 50 percent, where XP has four.
- `Bitmap#draw_text` and `Bitmap#text_size` take a text of any class and use its `to_s`.
- `Bitmap#font=` copies the name, the size, bold, italic and the colour into the bitmap's
  own font and leaves its shadow: `bitmap.font.shadow = false` turns the shadow off. `dup`
  and `clone` of a bitmap copy the shadow.

Outside the standard, or not there:

- The glyphs of the shadow are those of the text, so they differ from Windows as the text
  does. "Verdana" and "Arial" are looked up and replaced like every other font name.
- The widths a font gets at a squeeze step are those of the rule of XP, measured under Wine.
- `Bitmap#radial_blur` computes in double precision with the sine and cosine of the system.
  No measured pixel depends on their last bit; one that did could differ from the original.
- A call of `clear_rect` or `gradient_fill_rect` with no argument ends the original with a
  page fault; here it raises `ArgumentError`.
- `Font.default_shadow`, like `Font.default_bold`, reads true or false and not the object
  it was given. The default names are those of the English runtime whatever `Library=`
  says; the Japanese runtime was not measured.
- The pixels without ink that `draw_text` touches follow the text cell of the draw, as in
  XP, and not the work area the original keeps between calls. They are transparent, but a
  blur mixes their colour into its result.

In a game of XP none of `blur`, `radial_blur`, `gradient_fill_rect`, `clear_rect`,
`Font#shadow` and `Font.default_shadow` is defined, and text, the font defaults and
`Bitmap.new` are as before.

The class `Tilemap` of a game of VX is the one of RGSS2, measured on the same two versions
and drawn pixel for pixel as the original draws it on every measured frame:

- `bitmaps` is a `Tilemap::BitmapArray` of nine slots (the sheets A1 to A5 and B to E) and
  `passages` a table; `tileset`, `autotiles`, `priorities` and the class `TilemapAutotiles`
  are not defined. A slot outside 0 to 8 reads `nil` and stores nothing. Once a table is
  set, `map_data = nil` and `passages = nil` raise `TypeError`. `bitmaps` goes on working on
  a disposed tilemap.
- A tilemap is two planes of 672 by 512 pixels from the corner of its viewport, the lower at
  z 0 and the upper at z 200. The upper plane shows the tiles of the third layer whose entry
  of `passages` has the bit 0x10; such a tile in the first or second layer is not drawn.
- The three kinds of autotiles (floors, walls, waterfalls), the animation of the sheet A1
  every 30 calls of `update`, the tables with their lower edge, and the shadows beside walls
  are drawn by the class, as on the original.
- `visible = false` hides nothing, as on the original. The ox and oy of the viewport add to
  those of the tilemap, as they were when the tilemap was last written to or updated or when
  they last changed; giving the tilemap another viewport does not take the new ones.
- `flash_data` adds its colour to a cell once with each plane.

Where the original reads outside its arrays, vkxp draws nothing: for the tile numbers 1024
to 1535, for negative tile numbers, and for the layers a map with fewer than three does not
have. The original shows garbage or ends with a page fault there.

Not measured: a tilemap under `Viewport#flash`, a freeze or a transition beyond what follows
from its being an element; how a plane and a sprite of the same z are ordered by the y of
the sprite, where vkxp orders by creation alone; frame sizes other than 544 by 416 and 640
by 480.

In a game of XP `Tilemap` is as it was: none of `bitmaps`, `passages` and
`Tilemap::BitmapArray` is defined.

## RPG Maker VX Ace

Support for RPG Maker VX Ace (RGSS3) is experimental, and a run of it says so once. The
scripts of a game of VX Ace run over Ruby 1.9.2, the interpreter of the original runtime, in
the guest executable `vkxp-guest-rgss3`, which `vkxp` starts from beside itself, always in a
process ([Guest process](running.md#guest-process)). The start of a game is the one measured
on the original runtime 3.0.1: the built-in scripts file, `$RGSS_SCRIPTS`, each script
evaluated as `{0000}`, `{0001}` and so on and compiled as UTF-8, `RGSSReset` with
`Graphics.__reset__` and `Audio.__reset__`, the endings, the error report, `msgbox` and
`msgbox_p`, `p`, `print` and `puts` on the standard output, `load_data` that labels the
strings of a data file UTF-8, `save_data`, and `RGSS_VERSION`, which follows the DLL the game
ships: `"3.0.0"` for a DLL of a version 3.0.0.x, `"3.0.1"` for any other and for none. The
glyph rule of text follows the DLL by the same table (see *Font and text of VX Ace*).

Of the classes, `Window`, `Tilemap`, `Sprite`, `Plane`, `Viewport`, `Font`, the text and
creation of `Bitmap`, the modules `Graphics`, `Input` and `Audio` and the value classes `Color`,
`Tone`, `Rect` and `Table` are those of RGSS3, as the sections below say. A game made with
the standard scripts of VX Ace shows its title screen and its menu and starts a new game; no
frame of it was compared with the original.

Known differences of the start, each against the measurements of the original:

- An uncaught exception ends the run with status 1; the original's code follows the line
  number in most cases. A report of more than 1023 UTF-16 units is shown, where the
  original shows an empty box.
- A script archive that cannot be loaded is an error that names it, with a failure status;
  the original shows `Failed to load script.` and ends with code 0.
- `at_exit` handlers are not run, as on the original, and `exit!` is not followed.
- After a reset by the host a script that calls no function of `Graphics` is not
  interrupted; `Graphics.wait`, `fadeout`, `fadein` and `transition` are not cut short, and
  `Graphics.__reset__` does not wait for F12 to be released.
- `load_data` does not freeze strings as the original does in two of 71 samples, and its
  errors for a loose file and its lookup rules are those of RGSS2.
- `Win32API` and `DL` are not those of RGSS3; `require` and `load` are those of vkxp.
- A state is not saved while a script thread is alive: a thread of Ruby 1.9.2 is a thread
  of the system, which a copy of the process does not have.

What follows of the classes was measured on the original runtime of VX Ace (DLL versions
3.0.0.1 and 3.0.1.1) under Wine.

### The tilemap of VX Ace

The class `Tilemap` of a game of VX Ace is the one of VX with `flags` and `flags=` in the
place of `passages` and `passages=`:

- `bitmaps` is a `Tilemap::BitmapArray` of nine slots, as in VX. `flags` is a table kept by
  reference, under the rules of `passages`: once a table is set, `map_data = nil` and
  `flags = nil` raise `TypeError` and the table stays; `flash_data = nil` is always
  accepted. `passages`, `tileset`, `autotiles`, `priorities` and the class
  `TilemapAutotiles` are not defined.
- A map has four layers, the fourth being the shadows, one bit for each quarter of a cell.
  A tile whose entry of `flags` has the bit 0x10 is drawn on the upper plane from the third
  layer and on the lower plane from the first two. `visible = false` hides the tilemap.
- A change made through the `Table` object of `map_data` or `flags` shows in the next frame
  without `update`.

Known differences:

- A write to a slot of `bitmaps` of a disposed tilemap ends the original with a page fault.
  vkxp accepts it and keeps the object in the slot, and the tilemap draws nothing.
- Where the original reads outside its arrays, vkxp draws nothing: the tile numbers 1024 to
  1535 (from 1245 on the original ends with a page fault), negative tile numbers (the
  original draws a block of the sheets A1 to A5 for -256 to -1280), and the layers that a
  map of fewer than four does not have. A map of three layers is drawn as on the original;
  one of fewer layers draws its layers, where the original draws other things or ends.
- A tilemap of a disposed viewport stays alive and usable, as on the original, and vkxp
  draws it nowhere while it is in that viewport; what the original draws of it was not
  measured ([Sprites, planes, viewports and Graphics of VX
  Ace](#sprites-planes-viewports-and-graphics-of-vx-ace)).

### Sprites, planes, viewports and Graphics of VX Ace

A game of VX Ace has the `Sprite`, `Plane`, `Viewport` and `Graphics` of VX with what
follows, measured on the original runtime of VX Ace:

- The scene is drawn by the rules of VX but for a turned sprite in a viewport, which stands
  at its own position plus the position of the viewport's rectangle less the viewport's
  origin, its bounding box and its texels both taken from there, as for a sprite on the
  screen.
- `Viewport.new` without arguments covers the screen as it is then, (0, 0, `Graphics.width`,
  `Graphics.height`), and keeps that rectangle when the screen is resized. One `Rect` is
  copied, four numbers are truncated. Another count, or one argument that is no `Rect`,
  raises `ArgumentError` with the message of the original, `wrong number of arguments (4 for
  1)`, or `(1 for 2)` after a `Rect`.
- `Viewport#dispose` disposes none of the sprites, planes, windows and tilemaps of the
  viewport: they stay alive and usable, their `viewport` returns the disposed viewport,
  `Graphics.update` goes on, and their own `dispose` disposes them.
- `Plane#viewport` is a reader without an argument and `Plane#viewport=` a writer that moves
  the plane and keeps its place among the elements of equal z, as `Sprite#viewport=` does.
- `Sprite.new`, `Plane.new` and `Tilemap.new` take nothing, `nil` or a `Viewport`; another
  count raises `ArgumentError` (`wrong number of arguments (1 for 2)`) and another object
  `TypeError` (`can't convert Fixnum into Viewport`). An argument of `Sprite`, `Plane` or
  `Viewport` that must be a `Color`, `Tone`, `Rect` or `Bitmap` raises `TypeError` worded as
  the original words it: `can't convert NilClass into Color`.
- `Graphics` has `play_movie` and `__reset__`, and `_reset` is not public.
  `Graphics.freeze` keeps the frozen picture in the buffer of 640 by 480 pixels of the
  original ([decisions.md](decisions.md#vx-ace)). `Graphics.snap_to_bitmap` gives all of
  (0, 0, 0, 0) at brightness 0.
- `Graphics.transition` with nothing frozen returns at once without looking at its
  arguments. After a freeze a fourth argument raises `ArgumentError` (`wrong number of
  arguments (3 for 4)`), a name that is no `String`, `nil` among them, raises `TypeError`
  (`can't convert nil into String`), and each of these errors keeps the freeze; an empty
  name is no mask; a name whose picture is not found raises `Errno::ENOENT` and ends the
  freeze.
- The plain values of sprites, planes and viewports, the disposed errors, and the waits,
  fades, brightness, frame rate, frame count, `width`, `height` and `resize_screen` of
  `Graphics` with its limits are those of VX, and so is the order of elements. A resize
  keeps the brightness, the frame count, the frame rate, a freeze and the rectangles of
  viewports.

Known differences:

- A disposed viewport given to `Sprite#viewport=` or `Plane#viewport=` ends the original
  with a page fault. vkxp raises `RGSSError` (`disposed viewport`) and moves nothing.
- The elements of a disposed viewport are drawn nowhere in vkxp until they are moved to the
  screen or to another viewport; what the original draws of them, and of an element moved
  out of such a viewport, was not measured ([decisions.md](decisions.md#vx-ace)).
- `Graphics.play_movie` plays nothing: for a file of the game directory under the name, or
  under the name with `.ogv` appended, it returns `nil` at once and shows no frame; for no
  such file it raises `Errno::ENOENT` as the original does. How the original looks a movie
  up was not measured ([decisions.md](decisions.md#vx-ace)).
- What depends on the word size ([decisions.md](decisions.md#vx-ace), "Word size"):
  `Graphics.frame_count` goes on beyond 2^31 - 1 where the original wraps to -2147483648,
  `Graphics.frame_count = 2**31` is kept where the original raises `RangeError`, and the
  `RangeError` of an integer of 2^31 or more given to a writer has another message.

Not measured: what `Graphics.visible=` does; what `Plane#visible` reads after `visible =
false`, which vkxp reads `true` as in VX; the pixels of `transition` with an empty name,
which vkxp shows as a plain transition.

In a game of XP or VX these classes and `Graphics` are as they were.

### Input of VX Ace

The module `Input` of a game of VX Ace is the one of VX with what follows, measured on the
original runtime of VX Ace (versions 3.0.0.1 and 3.0.1.1) under Wine:

- The twenty constants are the Symbols of their names: `Input::DOWN` is `:DOWN`, and so on to
  `Input::F9`.
- `press?`, `trigger?` and `repeat?` take exactly one argument; none or two raise
  `ArgumentError`. The Symbol of a button's name, written as the constant is (`:DOWN`, not
  `:down`), or the number of the button in VX (2, 4, 6, 8, 11 to 18, 21 to 23, 25 to 29)
  names that button. Anything else gives `false` and raises nothing: another Symbol, a
  String, a Float, another Integer, `nil`, `true`, `false`, an Array, any other object. No
  method of the argument is called, so an object that defines `to_int` or `to_sym` gives
  `false` too; that case was not measured.
- An `Input.update` sees the input that had arrived when `Graphics` last showed a frame, which
  `Graphics.update` and each frame of `wait`, `fadeout`, `fadein` and `transition` do. So
  `Input.update` alone never sees a new key, and a key pressed before a `Graphics.update` is
  seen by the `Input.update` after it. A call of `Graphics` that shows no frame
  (`frame_reset`, `brightness=`, `snap_to_bitmap`, `freeze`, `resize_screen`, a `wait`, fade
  or transition of no frame) makes no key visible, and in vkxp neither does a frame shown
  while a message box is open, which was not measured on the original.
- `trigger?`, `repeat?`, `dir4` and `dir8` are those of VX, without a mismatch in 12084
  updates: one button repeats at a time, the one triggered last, at the 1st, 24th and 30th
  update and every 6th after; of buttons triggered together the one with the highest number
  repeats, so with Shift `SHIFT` repeats and `A` never.
- The keys are those of VX, F5 to F9 included ([The keys of a game of
  VX](running.md#the-keys-of-a-game-of-vx)); the original of VX Ace has the table of VX line
  for line.

Known differences, which follow from vkxp having no queue of key messages: the host sends
whole input states, and every frame shown makes the state that has arrived visible
([decisions.md](decisions.md#vx-ace)).

- A frame of the original takes exactly one key message when it has no time to wait and
  more, up to all, when it waits; how many is not a rule, and vkxp imitates no count. Several
  key changes between two frames are seen together in vkxp, where frames of the original
  without time to spare spread them over the following updates.
- A key pressed and released between two frames is never seen in vkxp; the original saw it
  for one update when its frame took only the key-down.
- The original takes a key message now and then during `Kernel#sleep`, so that a key can
  become visible to `Input.update` after a sleep; vkxp takes none.
- `GetKeyState`, `GetAsyncKeyState` and `GetKeyboardState` of `Win32API` follow the rules of
  VX; on the original `GetKeyState` changes when `Input.press?` does.

Not measured: gamepads, the F1 dialog, F12 and Alt+Enter of the original of VX Ace.

In a game of XP or VX `Input` is as it was: its constants are numbers, and an `Input.update`
sees the newest input that has arrived.

### The value classes of VX Ace

Of `Color`, `Tone`, `Rect` and `Table`, a game of VX Ace has what follows beside what a game
of VX has, measured on the original runtime of VX Ace (`RGSS301.dll` and `RGSS300.dll`,
versions 3.0.1.1 and 3.0.0.1) under Wine:

- `Color.new` and `Tone.new` without arguments give (0, 0, 0, 0). `set` takes no argument,
  which sets (0, 0, 0, 0), three or four numbers, or one `Color` or `Tone` whose four numbers
  it copies in order, a tone's grey amount becoming a colour's alpha and back. One argument
  of `new` is never an object to copy.
- `Rect.new` takes no argument, giving (0, 0, 0, 0), or four integers. `Rect#set` takes four
  integers or one `Rect` whose values it copies. Both have the arity -1.
- `set` returns the receiver in every form, and a change made by any form reaches the
  sprite, plane, viewport or window the object belongs to.
- One argument of `set` that no native class made, a number or `nil` for instance, raises
  `TypeError` with the message of the original, `wrong argument type Fixnum (expected
  Data)`, naming `nil`, `true` and `false` as such.
- A count of arguments these methods, `Table#[]` and `Table#[]=` do not take raises
  `ArgumentError` with the message of the original, `wrong number of arguments (3 for 1)`:
  the expected count first. It is 3 below three arguments and 4 above four for `Color` and
  `Tone`, 4 for `Rect`, and the number of dimensions of the table for `Table#[]` and for a
  `Table#[]=` with fewer arguments than that. A number of the wrong kind raises the
  `TypeError` of Ruby 1.9.2, such as `can't convert String into Integer`.
- `Color._load` of a string that is not 32 bytes long gives (0, 0, 0, 0), and so does
  `Tone._load`, for which it was not measured. `Rect._load` of a string shorter than 16
  bytes reads zeros for what is missing.
- `==`, `===` and `eql?` of `Color`, `Tone` and `Rect` are false for `nil`, a number and an
  object of another class, and raise nothing, as version 3.0.1.1 has it.
- The limits of the components, `_dump`, `to_s` and `Table` are those of VX. `inspect`
  gives the text of `to_s`, the serialized forms and texts are ASCII-8BIT strings, and
  `hash` does not depend on the values.

Known differences:

- `==`, `===` and `eql?` with an object of another class raise `TypeError` on version
  3.0.0.1 of the original (`can't convert NilClass into Color`), which one game of the corpus
  ships; vkxp returns false whatever the game ships
  ([decisions.md](decisions.md#vx-ace), "Text and bitmaps of RGSS3").
- `set` with an object of another native class, `rect.set(color)` for instance, copies the
  bytes of that object on the original; vkxp raises `TypeError`.
- `Color._load` of 32 bytes keeps a component outside its limits on the original, and `to_s`
  of a component such as 1e300 ends it; vkxp limits the components when it loads them.
- `Rect._load` of a short string reads the memory behind the string on the original, where
  vkxp reads zeros. `Table._load` of a short or malformed string ends the original with a
  page fault; vkxp raises `ArgumentError` and the game goes on.
- What depends on the word size, the guest being a 64-bit build
  ([decisions.md](decisions.md#vx-ace), "Word size"): the original keeps integers from 2^30
  to 2^31 - 1 in 31 bits (`Rect.new(2**30, 0, 0, 0)` has the x -1073741824), where vkxp keeps
  them as they are; the message of the `RangeError` of an integer of 2^31 or more is
  ``bignum too big to convert into `long'`` on the original and
  ``integer 2147483648 too big to convert to `int'`` in vkxp; and `t[1, 1, 1] = 2**31`
  raises `RangeError` on the original, where vkxp stores the low 16 bits.
- `Table#[]=` with one index too few, `t[0, 0] = 5` on a table of three dimensions, wrote
  the element (0, 0, 0) on the original in the one sample taken; vkxp raises
  `ArgumentError`.

In a game of XP or VX the value classes are as they were: none of these forms is taken.

### The window of VX Ace

The class `Window` of a game of VX Ace is the one of VX with these differences, measured on
the original runtime of VX Ace (versions 3.0.0.1 and 3.0.1.1) under Wine:

- `Window.new(x = 0, y = 0, width = 0, height = 0)` takes up to four integers, a `Float`
  truncated and negative values kept, and no viewport: a `Viewport` among them raises
  `TypeError` (`can't convert Viewport into Integer`), a fifth argument `ArgumentError`
  with the message of the original, `wrong number of arguments (4 for 5)`. `viewport=`
  moves a window into a viewport, as in VX. A new window has z 100, a back opacity of 192,
  paddings of 8, shows scroll arrows, has a neutral tone and a contents bitmap of 1 by 1 of
  its own, which is not disposed with the window.
- The class adds `padding`, whose writer sets `padding_bottom` too, and `padding_bottom`,
  neither of them limited; `arrows_visible`; `move(x, y, width, height)`, which returns the
  window; `open?` and `close?`, true at the openness 255 and 0 alone; and `tone`. It has no
  `stretch`.
- `tone` is a `Tone` object of the window, the same at every read. `tone=` takes a `Tone`
  only and copies its values into that object, and anything else raises `TypeError` (`can't
  convert NilClass into Tone`). A change of the object, through `tone=` or its own methods,
  tones the background from the next frame on.
- The window is drawn by the rules measured on the original, which reproduced 1185 captures
  without a wrong pixel: the cursor lies at the padding plus the distance between the
  cursor rectangle and the origin, the contents lie at the padding and are cut by both
  paddings, and the background is toned and drawn at every openness. Where the cursor lies,
  whether the contents show, which scroll arrows are drawn and what is built from the skin
  are decided at the calls the original decides them at and kept until the next of them: a
  cursor rectangle that is assigned or changed through its own methods, also to the values
  it has, places the cursor, and so do `ox`, `oy` and the paddings; `width`, `height` and
  `move` do not.
- A skin or a contents bitmap that is disposed while the window has it raises nothing, the
  reader goes on returning the object, and the window is drawn as without it: nothing of a
  disposed skin shows, not even the background and frame built from it before.

Known differences:

- The original ends with a page fault where vkxp goes on and the calls take effect: when the
  cursor is placed (by `ox=`, `oy=`, `padding=`, `padding_bottom=` or a cursor rectangle of 0
  by 0) in a window on which `windowskin=` was never called; at a window or a cursor
  rectangle of 100000 by 100000 and a window of 20000 by 20000 with a skin, of which vkxp
  leaves out what cannot be built; and at `contents=` after `windowskin=` with a bitmap of
  8 by 8 on a new window.
- A window whose viewport is disposed stays alive on the original; vkxp disposes it with
  the viewport, as in XP and VX. `viewport=` with a disposed viewport raises `RGSSError`
  in vkxp, as in VX; it was not measured on VX Ace.
- Some errors are worded otherwise: `cursor_rect = nil` says `can't convert NilClass into
  Rect` on the original and `wrong argument type NilClass (expected Rect)` in vkxp, and
  `windowskin = 5` and `contents = 5` say `can't convert Fixnum into Bitmap` there and
  `wrong argument type Fixnum (expected Bitmap)` here. The message of the `RangeError` of a
  large integer depends on the word size, as for the value classes.

Not measured: which of the four values `move` assigns before an argument that cannot be
converted raises (vkxp converts all four first and assigns none), whether the contents
bitmap a window was made with is disposed with it, sizes above 600 pixels and origins
beyond 1000, `flash` of a viewport over a window, `Graphics.__reset__` with windows alive,
and text in the contents.

In a game of XP or VX the window is as it was: `Window.new` takes a viewport and none of
these methods is defined.

### The audio of VX Ace

`Audio` of a game of VX Ace is the module of RGSS3, measured on the original runtime of VX
Ace (versions 3.0.1.1 and 3.0.0.1) under Wine:

- `bgm_play` and `bgs_play` take a file name and up to three of volume, pitch and `pos`;
  `me_play` and `se_play` a file name and up to two of volume and pitch. `bgm_pos`,
  `bgs_pos` and `setup_midi` exist, and every function returns `nil` but the two positions.
  A wrong count raises `ArgumentError` with the expected count first, `wrong number of
  arguments (1 for 0)`; a Float is truncated, and `pos` may be `nil`, which is 0.
- Volume and pitch are not limited as in XP and VX: a volume of 100 or more and one below 0
  is full volume; a pitch below 1 is the normal rate, and so is one that asks for more than
  200000 Hz of a WAV or Ogg Vorbis file (454 at 44100 Hz).
- `pos` is a place in bytes of 16-bit PCM, whatever the file holds: a WAV file of 8-bit or
  16-bit PCM and an Ogg Vorbis file start at the sample frame it names; other files start
  at their beginning.
- A `bgm_play` or `bgs_play` with the name and the pitch of what its channel plays goes on
  where the sound is, whatever `pos` says; only its volume changes. After a stop or a fade
  the sound starts anew.
- `bgm_pos` and `bgs_pos` follow the rule measured on the original: the position of its
  decoder, which reads 15 chunks of 187.5 ms at once and then one chunk per 187.5 ms at
  pitch 100, less the 16 chunks of its buffer, and which jumps at the end of the file, or
  at the loop end that `LOOPSTART` and `LOOPLENGTH` in upper case make, to the loop start.
  A sound of at most 529200 bytes of 16-bit PCM is a cursor that runs through it. The
  position is 0 before the first play and for a file that is not WAV or Ogg Vorbis, keeps
  its value after a stop and goes on during a fade.
- A music effect holds the position of the music, not that of the background sound, for
  its length, `me_stop` ends the hold and `me_fade` does not; when the effect has ended the
  music returns to its volume over one second.
- A sound is looked for with the extensions `ogg`, `mid`, `mp3`, `wav` and `wma`, in this
  order, after the name as given.

Known differences:

- The position follows the frames the game has shown, at their frame rate, not real time:
  it stands still during `sleep`, while a message box is shown and while a script computes,
  where the original's goes on, and it does not follow fast-forward. The original lags
  behind its own rule at a pitch above 100 and when the machine is busy; vkxp does not.
  The two copies of the position in a save of a game are therefore those of the original
  at that frame, or apart from them by whole chunks, as two runs of the original are.
- The position of a short sound is that of the rule, not the play cursor of the original,
  which moves in steps of 10 ms of real time; where between 529200 and 530964 bytes the
  original begins to stream a sound is not known, and vkxp streams from 529201.
- The position of MIDI, MP3 and WMA is always 0; that of MIDI was not measured. A WAV file
  that is not PCM of 8 or 16 bits reads 0 and starts at its beginning.
- Tags whose loop start is not before their loop end count as no tags, and a tag whose
  value is not decimal digits as no tag; neither was measured.
- The sound is not looped at `LOOPSTART` and `LOOPLENGTH`: the whole file repeats, so after
  the first loop what is heard is not where the position says.
- A music effect that is not WAV or Ogg Vorbis does not hold the position, because its
  length is not known to the game's side. After two music effects in a row the original's
  music began at 0 again, once; vkxp goes on.
- The same file under another name, `Theme` and `Theme.ogg`, is one file to the original,
  which goes on, and two to vkxp, which starts again.
- For a file that is not WAV or Ogg Vorbis the pitch is at most 65535, a limit of vkxp: the
  original asks for `pitch / 100` of the playback rate and what follows was not measured. A
  MIDI file at a pitch that the synthesiser cannot render (below 25 or above 300 at an
  output of 48000 Hz) is reported and stays silent.
- The lookup of the original asks for `name.*` before the name as given, which also finds a
  file such as `Theme.v1.ogg` for `Theme`; vkxp does not. A name with a NUL byte or in
  UTF-16LE is not cut as the original cuts it.
- One sound effect per file and frame, a fade that follows real time and the volume set
  again in every frame are not followed; a faded music is ended by the mixer, while its
  position goes on in the game as on the original.
- The message of the `RangeError` of a value of 2^31 is that of a 64-bit guest.

### Font and text of VX Ace

Of `Font` and `Bitmap`, a game of VX Ace has what follows, measured on the original runtime
of VX Ace (versions 3.0.0.1 and 3.0.1.1) under Wine. The pixel operations of `Bitmap` are
those of VX.

- The font defaults are the name `"VL Gothic"`, a String, size 24, neither bold nor italic,
  no shadow, an outline, white, and the out colour (0, 0, 0, 128). `Font` has `outline`,
  `out_color`, `Font.default_outline` and `Font.default_out_color`. A new font and the font
  of a new bitmap take the defaults of the moment they are made.
- A font keeps the objects it is given: `name=` any object, `nil` and an Array included,
  and `size=` the number, a `Float` too. The size is from 6 to 96 after its fraction is
  dropped (`ArgumentError`, `bad value for size`), and text is measured and drawn with that
  integer. The `default_` writers keep their objects in the same way, the flags included,
  and `Font.default_color=` and `Font.default_out_color=` keep the `Color` given, so that a
  later change of it changes the default; `color=` and `out_color=` copy a `Color` into the
  font's own object. Another class raises `TypeError` (`can't convert NilClass into Color`).
  `Font.new` takes at most a name and a size.
- The font is drawn with the first element of an Array that exists exactly, else its first
  element, else the object given; by its `to_s`, cut to 31 characters. A font without a name
  (`""`, `[]`, `nil`) draws nothing. `Font.exist?` is exact and case-sensitive over the family
  names of the fonts vkxp has; a Symbol is taken by its name and any other object that is no
  String is false.
- A game of VX Ace takes fonts from the `Fonts` directory of each of its run-time packages,
  after its own `Fonts` directory, as the original loads those of the package.
- `Bitmap#font=` copies every attribute, the shadow, the outline and the out colour
  included, and `dup` and `clone` of a bitmap give the copy a font of its own with all of
  them.
- `Bitmap#draw_text` follows the text rules of RGSS3: the text is placed with the outline's
  two pixels, squeezed in one step by the metrics of the System font, drawn in the passes of
  the outline, the shadow and the text, and blended over the whole work rectangle, also
  where no glyph is, as the original blends it. `Bitmap#text_size` counts
  neither the outline nor the shadow. The glyph masks are those of the DLL the game ships:
  where glyphs overlap, a glyph of a DLL 3.0.0.x replaces earlier values over its whole box,
  and one of every other DLL only where it has ink.
- Text is measured and placed from glyphs hinted as Wine's FreeType hints them: the
  advances, the overhangs and the placement of the glyphs follow the hinted glyphs, and the
  glyph shapes still differ from those of the original.
- `Bitmap.new(width, height)` has no limit of 4194304 pixels, and a negative height is
  taken by its size: `Bitmap.new(10, -5)` is 10 by 5.
- The methods of `Bitmap` that take several argument counts word a wrong count with the
  expected count first, `wrong number of arguments (5 for 3)`, and a call without arguments
  raises `ArgumentError`, as on the original; an argument that is no `Color`,
  `Rect`, `Bitmap` or `Font` raises `TypeError` with `can't convert NilClass into Rect` and
  the like, and `Bitmap.new` and `Font.new` with three arguments say `(3 for 1..2)` and
  `(3 for 0..2)`.

Known differences:

- The largest bitmap is the one vkxp has for every runtime, 268435456 pixels. The original
  has no rule but the free address space of its process: it created 8192 by 44954 in one
  run, which vkxp refuses.
- Text wider than 640 pixels ends at the work rectangle; the original writes the part right
  of column 640 into its surface 640 columns to the left and one row up.
- `text_size` of a font without a name is (0, 0, 0, 0); the original returns uninitialised
  numbers.
- The vertical names that begin with `@`, `"@VL Gothic"` among them, do not exist in vkxp;
  on the original `Font.exist?` is true for them.
- A name of the substitution table counts as existing when a font that replaces it is
  there: `Font.exist?("Arial")` is true on a system with Liberation Sans.
- When "VL Gothic" is missing, the original shows "Unable to find VL Gothic font." at the
  start; vkxp shows nothing and draws with its fallback families.
- The original adds the name in the other language of the two fonts of the run-time package
  ("VL ゴシック" for "VL Gothic"); vkxp knows such a name only if the font file declares it.
- What Windows' GDI answers in the measurement (the advances and metrics of a font, the
  width of a squeezed font, the font chosen for a missing name, the glyph shapes) was Wine's
  answer, and vkxp supplies its own for it, as for XP and VX.

Not measured: `Font.new` with a `Float` size, and whether a new font shares the name and
size objects of the defaults (in vkxp it does); `Font.exist?` of an object of another
class than a String, a Symbol, `nil`, an Integer and an Array; `draw_text` with two or three
arguments whose first is no `Rect`, `fill_rect` whose first is no `Rect`, and the counts of
`clear_rect` and `gradient_fill_rect` other than none and two.

In a game of XP or VX `Font` and `Bitmap` are as they were: neither `outline` nor
`out_color` is defined.

## Limits of save states

Save states are described under [Save states](running.md#save-states). Their limits:

- States live in memory and end with the run. Nothing is written to disk.
- Files the game wrote after a state was saved stay as they are, its own saves included; a
  file a script holds open across frames shares its position with the copies.
- Each state costs the memory pages the game has changed since it was saved.
- States need the guest in a process, and no state is saved with
  `--allow-process-functions`, nor in a game of VX Ace while a script thread is alive.
- The keys only show a notice while input is recorded or replayed.
