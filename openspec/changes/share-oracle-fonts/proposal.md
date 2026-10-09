# Proposal

## Why

Text cannot be compared between the original and vkxp today. The harness maps common game font
names to whatever CJK font the machine has; on the development machine that is a variable font,
which Wine's GDI renders at its thinnest instance, so the original's text comes out small and
faint while vkxp draws a regular weight. To compare where text sits and how wide it runs, both
runtimes must draw with the same font file.

## What Changes

- `cargo xtask oracle --font FILE` (repeatable): each file is installed into the run's Wine
  prefix and given to vkxp as a font directory, and the common game font names are mapped to
  that file's family on the original side.
- Without `--font` the harness behaves as before and says in its output that text is not
  comparable.

Out of scope: any font shipped in the repository, tuning of vkxp's text metrics.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `oracle-harness`: shared fonts.

## Impact

- `xtask` only, and a paragraph in `README.md`.
