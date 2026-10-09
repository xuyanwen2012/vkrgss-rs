# Spec Delta

## ADDED Requirements

### Requirement: JPEG arithmetic of the original
A baseline or progressive Huffman-coded JPEG in greyscale or in three-component YCbCr with
chroma at full resolution or halved horizontally or in both directions SHALL be decoded to the
pixels the original runtime produces: the floating-point inverse DCT of IJG libjpeg 6b with
its x87 evaluation rules and truncation toward zero before the rounding shift, that library's
triangle chroma upsampling, and its colour conversion tables. Other JPEG files SHALL be decoded
as before.

#### Scenario: Measured colour blocks
- **WHEN** each committed flat block of known Y, Cb and Cr measured on the original runtime is decoded
- **THEN** red, green and blue equal the measured values

#### Scenario: Measured grey samples
- **WHEN** the committed grey test images at three qualities are decoded
- **THEN** every sample equals the measured one

#### Scenario: Subsampled chroma
- **WHEN** the committed small colour image with chroma halved in both directions is decoded
- **THEN** every pixel equals the measured one

#### Scenario: Progressive equals baseline
- **WHEN** the same coefficients are stored as a baseline and as a progressive file
- **THEN** both decode to the same pixels

#### Scenario: Other layouts still decode
- **WHEN** a JPEG with chroma halved vertically only is decoded
- **THEN** it decodes as before this change
