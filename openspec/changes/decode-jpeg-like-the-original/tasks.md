# Tasks

## 1. Decoder (`vkxp-image`)

- [x] 1.1 Decode baseline scans to coefficients and implement dequantisation, the float inverse DCT and greyscale output; commit the grey test files with their measured samples. Verify: the measured-grey scenario of `specs/image-decoding/spec.md`.
- [x] 1.2 Add chroma upsampling and colour conversion; commit the colour test data. Verify: the measured-colour-blocks and subsampled-chroma scenarios.
- [x] 1.3 Add progressive scans and the dispatch to the library for other layouts. Verify: the progressive-equals-baseline and other-layouts scenarios; the existing JPEG tests still pass.

## 2. Integration check

- [ ] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate decode-jpeg-like-the-original --strict` passes.
