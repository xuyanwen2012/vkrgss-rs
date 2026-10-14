# Compatibility

vkxp aims at the behaviour of the original runtimes of RPG Maker XP, RGSS 1.04, and of RPG
Maker VX, RGSS2: both are supported. Games that use the default scripts or common
community scripts are meant to run unmodified. VX Ace is planned and not supported yet. What
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

## Limits of save states

Save states are described under [Save states](running.md#save-states). Their limits:

- States live in memory and end with the run. Nothing is written to disk.
- Files the game wrote after a state was saved stay as they are, its own saves included; a
  file a script holds open across frames shares its position with the copies.
- Each state costs the memory pages the game has changed since it was saved.
- States need the guest in a process, and no state is saved with
  `--allow-process-functions`.
- The keys only show a notice while input is recorded or replayed.
