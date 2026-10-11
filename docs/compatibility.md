# Compatibility

vkxp aims at the behaviour of the original runtime of RPG Maker XP, RGSS 1.04. Games that
use the default scripts or common community scripts are meant to run unmodified.

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
- **Win32API shims.** `ShowCursor` keeps its count but does not hide the pointer,
  `GetKeyState` never reports the toggle state of a lock key, and `keybd_event`,
  `mouse_event` and `SetKeyboardState` inject nothing.
- **Blending onto a translucent bitmap.** Addition and subtraction onto a destination whose
  alpha is below 255 use the textbook formula.
- **Transition masks.** The level of a coloured mask pixel is one lower than the original's
  for a few colours; a grey mask is exact.

Besides these, the loop markers of MIDI files are not followed: a track loops from its
beginning. vkxp runs on Linux x86_64 only.

## Limits of save states

Save states are described under [Save states](running.md#save-states). Their limits:

- States live in memory and end with the run. Nothing is written to disk.
- Files the game wrote after a state was saved stay as they are, its own saves included; a
  file a script holds open across frames shares its position with the copies.
- Music and background sound start over after a load.
- Each state costs the memory pages the game has changed since it was saved.
- States need the guest in a process, and no state is saved with
  `--allow-process-functions`.
- The keys only show a notice while input is recorded or replayed.
