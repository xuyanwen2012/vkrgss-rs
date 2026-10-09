# JPEG test files

`make.py` computes every picture here and writes the `.jpg` files; none comes from a game.

Each `.png` holds the pixels the original runtime (RGSS104E.dll under Wine, through
`cargo xtask oracle`) showed for the `.jpg` of the same name, drawn as a sprite at the top left
of the screen, on 2026-10-09. The progressive and the restart-interval variants of a file
showed the pixels of the plain file, so they have no `.png` of their own.

| File | Content |
|---|---|
| `flat` | 640 flat 8x8 blocks of known Y, Cb, Cr, quantisation tables of ones, chroma at full resolution |
| `grey-q90`, `grey-q1`, `grey-q50` | one greyscale picture at quality 90, with quantisation tables of ones, at quality 50 |
| `grey-q90-progressive` | `grey-q90` stored progressively |
| `colour-444`, `colour-422`, `colour-420` | one 67x45 colour picture with chroma at full resolution, halved horizontally, halved in both directions |
| `colour-420-progressive`, `colour-420-restart` | `colour-420` stored progressively, and with a restart interval of 3 MCUs |
| `colour-420-narrow` | 3x5 pixels with chroma halved in both directions |
