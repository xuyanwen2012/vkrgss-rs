# Design

## Context

- The rule and its verification are in `local/measurements/README.md`, section "JPEG decoding",
  with a model in Python: `jpeg-model.py`, `jpeg-coefficients.py` (a baseline Huffman
  coefficient decoder) and `jpeg-idct-float.py`; the probe material is
  `jpeg-probe-make.py`, `jpeg-probe.rb`, `jpeg-files-probe.rb`; captured original frames of the
  probes are in `local/measurements/jpeg-frames/` (all in this clone, ignored by git). Read the
  section first: it has the multiplier table, the constants, the column shortcut, the rounding
  and the upsampling in full.
- `crates/vkxp-image/src/jpeg.rs` calls a decoder crate that returns pixels and exposes no
  coefficients. The crate forbids `unsafe`.
- An earlier change in this crate grew a second, strict JPEG reader in front of the library
  under review pressure about damaged files and was withdrawn. This change is not about damaged
  files.
- IJG libjpeg 6b is the reference for the arithmetic and for the order of operations
  (`jidctflt.c`, `jdsample.c`, `jdcolor.c`, `jdhuff.c`, `jdphuff.c`); its algorithms are
  documented and freely usable, and the code here is written in Rust, not copied.

## Decisions

- A decoder in Rust inside `vkxp-image`: marker parsing for the supported subset, Huffman
  decoding of baseline and progressive scans into coefficient blocks, dequantisation with the
  float multiplier table, the float inverse DCT, upsampling, colour conversion, cropping to the
  image size.
- x87 semantics are written out, not left to the compiler: every stored temporary is rounded
  to `f32`, every expression is evaluated in `f64`, exactly as the README states; the test on
  the measured grey samples is what proves it.
- Dispatch: the own decoder handles the layouts of the requirement; anything else, and any file
  on which the own decoder gives up, goes to the library decoder as today. The own decoder
  gives up rather than guess; it adds no validation of its own beyond what decoding needs.
- Test data: small JPEG files generated for the purpose with the probe generator (flat blocks,
  grey ramps, a small colour image) together with the pixel values measured on the original
  for them, a few kilobytes each. Files from games or the RTP are not committed.

## Risks / Trade-offs

- [A second JPEG path to maintain] → It is the only way to the original's pixels; the library
  path stays for everything unusual.
- [Restart intervals, odd sizes with subsampling] → Implemented as libjpeg does; marked
  PLACEHOLDER where the README says "not measured".

## Follow-ups

- The rebuilt DLL build that differs at exact float ties.
