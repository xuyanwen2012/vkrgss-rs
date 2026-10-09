# Proposal

## Why

Every frame that shows a JPEG file differs from the original by 1 to 3 per channel in a large
share of its pixels: a game's title picture (51743 pixels of one frame), an opening star field,
a scrolling picture, a panorama. The cause was measured: the original decodes with IJG libjpeg
6b and its floating-point inverse DCT, evaluated with x87 precision rules, truncating toward
zero before the final rounding shift, with the library's triangle chroma upsampling and its
colour tables. vkxp uses a Rust decoder with another inverse DCT and colour conversion. A
model of the original's arithmetic reproduces whole decoded frames with no differing pixel.

## What Changes

- `vkxp-image` decodes baseline and progressive Huffman JPEG files itself: entropy decoding to
  coefficients, the measured floating-point inverse DCT, the measured chroma upsampling and
  colour conversion.
- Files outside that (other component counts or sampling layouts, arithmetic coding, 12-bit
  samples) are decoded by the library as before.
- Tests pin samples measured on the original.

Out of scope: one rebuilt DLL build that differs from this arithmetic at exact ties (143 and
17 pixels of two measured files); stricter treatment of damaged files than today.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `image-decoding`: JPEG arithmetic of the original.

## Impact

- Crate `vkxp-image` only.
