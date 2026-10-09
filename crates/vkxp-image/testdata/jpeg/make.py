#!/usr/bin/env python3
"""Generates the JPEG test files of this directory (needs Pillow). Every picture is computed
here; none comes from a game.

usage: make.py OUT_DIR

The `*.png` files next to the JPEG files are not made by this script: they hold the pixels the
original runtime showed for each file (see README.md).
"""
import math, os, random, sys

from PIL import Image

out = sys.argv[1]
random.seed(5)
ONES = [1] * 64


def save(image, name, **options):
    image.save(os.path.join(out, name + ".jpg"), "JPEG", **options)


# Flat 8x8 blocks of known Y, Cb, Cr, stored without quantisation loss.
sweep = [i * 255 // 63 for i in range(64)]
blocks = [(i, 128, 128) for i in sweep] + [(128, i, 128) for i in sweep]
blocks += [(128, 128, i) for i in sweep] + [(0, i, 128) for i in sweep]
blocks += [(255, 128, i) for i in sweep] + [(128, i, i) for i in sweep]
blocks += [(128, i, 255 - i) for i in sweep]
while len(blocks) < 640:
    blocks.append(tuple(random.randrange(256) for _ in range(3)))
flat = Image.new("YCbCr", (256, 160))
for index, value in enumerate(blocks):
    x, y = index % 32 * 8, index // 32 * 8
    flat.paste(value, (x, y, x + 8, y + 8))
save(flat, "flat", qtables=[ONES, ONES], subsampling=0)

# Grey: ramps, a dark half (where truncation toward zero shows) and some texture.
grey = Image.new("L", (96, 64))
for y in range(64):
    for x in range(96):
        ramp = x * 255 // 95 if y < 32 else (x + y) * 60 // 158
        wave = 24 * math.sin(x * 0.7) * math.cos(y * 0.45)
        grey.putpixel((x, y), max(0, min(255, int(ramp + wave + random.randrange(-6, 7)))))
save(grey, "grey-q90", quality=90)
save(grey, "grey-q1", qtables=[ONES])
save(grey, "grey-q50", quality=50)
save(grey, "grey-q90-progressive", quality=90, progressive=True)

# Colour, of odd size so that the last chroma column and row are only half used.
colour = Image.new("RGB", (67, 45))
for y in range(45):
    for x in range(67):
        red = 128 + 120 * math.sin(x * 0.31 + y * 0.05)
        green = 128 + 120 * math.sin(y * 0.37 - x * 0.11)
        blue = (x * 5 + y * 3) % 256 if (x // 9 + y // 7) % 2 else 255 - x * 3
        colour.putpixel((x, y), tuple(max(0, min(255, int(v))) for v in (red, green, blue)))
save(colour, "colour-444", quality=85, subsampling=0)
save(colour, "colour-422", quality=85, subsampling=1)
save(colour, "colour-420", quality=85, subsampling=2)
save(colour, "colour-420-progressive", quality=85, subsampling=2, progressive=True)
save(colour, "colour-420-restart", quality=85, subsampling=2, restart_marker_blocks=3)
# Chroma of two samples a row, which libjpeg replicates instead of interpolating.
save(colour.crop((20, 10, 23, 15)), "colour-420-narrow", quality=85, subsampling=2)
