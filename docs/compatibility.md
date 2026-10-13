# Compatibility

vkxp aims at the behaviour of the original runtime of RPG Maker XP, RGSS 1.04. Games that
use the default scripts or common community scripts are meant to run unmodified. RPG Maker
VX is experimental, see [RPG Maker VX](#rpg-maker-vx); VX Ace is planned and not supported
yet. Everything else below is about XP.

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

Support for games of RPG Maker VX (RGSS2) is experimental. Of the module `Graphics`, a game
of VX has what follows, each measured on the original runtime of VX (`RGSS202E.dll`,
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
The rules reproduce 490 of the 496 pictures of that measurement without a wrong pixel; the
six others hold a skin smaller than 64 pixels. The tests compare a selection of them.

- The skin is read as 128 by 128: the background (0, 0, 64, 64) is stretched to the window
  less a margin of 2 and the pattern (0, 64, 64, 64) is tiled over it; the frame is built
  from the corners and edges at (64, 0, 64, 64); cursor, pause sign and scroll arrows come
  from their places in the skin.
- A window is one element at its `z`: contents, cursor, arrows and pause sign have no z of
  their own, so a sprite above the window's `z` covers all of it.
- `Window#openness` and `openness=`, limited to 0 to 255. Below 255 the window shows part of
  its rows around its middle, without contents, cursor, arrows and pause sign.
- The cursor blinks over 40 updates and the pause sign changes its image every 16.
- Background, frame and cursor are built when the skin, the size or the size of the cursor
  rectangle is assigned, and do not follow pixels drawn into the skin later; assigning
  `width` or `height` again builds them anew. Whether the contents show and which arrows are
  drawn is decided by `update` and by the assignments the original decides them in, so the
  contents of a window that was updated or given contents while invisible appear at its
  next `update`.

Not measured, or not there yet:

- A skin narrower or lower than 64 pixels: the original draws less of the background than
  vkxp does.
- A skin or a contents bitmap that is disposed while the window has it: vkxp goes on
  drawing the background, frame and cursor it built, and draws no contents, arrows or pause
  sign from a disposed bitmap.
- Among elements of equal `z` vkxp draws in the order of creation. The original puts a
  sprite before or behind a window of its `z` by the sprite's `y`; that order comes with the
  sprites of VX.
- A tone or a colour of a viewport over a window follows the rules of XP.
- Windows larger than 600 pixels and origins beyond 1000 were not measured.

In a game of XP the window is drawn as before and has no `openness`.

## Limits of save states

Save states are described under [Save states](running.md#save-states). Their limits:

- States live in memory and end with the run. Nothing is written to disk.
- Files the game wrote after a state was saved stay as they are, its own saves included; a
  file a script holds open across frames shares its position with the copies.
- Each state costs the memory pages the game has changed since it was saved.
- States need the guest in a process, and no state is saved with
  `--allow-process-functions`.
- The keys only show a notice while input is recorded or replayed.
