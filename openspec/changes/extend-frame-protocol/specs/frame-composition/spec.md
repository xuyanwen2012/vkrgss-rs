## MODIFIED Requirements

### Requirement: Quad op
A quad op SHALL draw a source rectangle of a bitmap through a fixed-point affine transform,
sampling the nearest source texel. The sampled texel SHALL first be changed by the quad's tone,
then blended with the quad's colour by that colour's alpha, both leaving the texel's own alpha
unchanged. The result SHALL then be combined with the pixel underneath at an opacity from 0 to
255 in normal, additive or subtractive mode.

#### Scenario: Zero opacity draws nothing
- **WHEN** a quad has opacity 0
- **THEN** the output is unchanged

#### Scenario: Transparent source texel draws nothing
- **WHEN** a source texel has alpha 0
- **THEN** the corresponding output pixel is unchanged in every blend mode, whatever the quad's
  tone and colour

#### Scenario: Additive and subtractive blending saturate
- **WHEN** an additive or subtractive quad would move a channel past 255 or below 0
- **THEN** the channel is clamped to 255 or 0 respectively

#### Scenario: Identity transform copies texels
- **WHEN** an opaque, fully opaque-alpha quad with a neutral tone and a colour of alpha 0 is
  drawn with an identity transform at an integer position in normal mode
- **THEN** each covered output pixel equals the corresponding source texel

#### Scenario: Neutral tone and zero-alpha colour change nothing
- **WHEN** a quad has a neutral tone and a colour whose alpha is 0, for any texel value
- **THEN** the frame equals the one produced with the texel drawn as it is

#### Scenario: Full-alpha colour replaces the texel's colour
- **WHEN** a quad's colour has alpha 255
- **THEN** every covered texel is drawn with the red, green and blue of that colour and with
  the texel's own alpha

#### Scenario: Tone is applied before colour
- **WHEN** a quad has both a non-neutral tone and a colour with an alpha between 1 and 254
- **THEN** the drawn texel equals the tone applied to the sampled texel followed by the colour
  blended over that result

#### Scenario: Tone and colour do not reach the pixels underneath
- **WHEN** a quad with a non-neutral tone and a colour is drawn at opacity 0, or a pixel inside
  its clip rectangle is not covered by its source rectangle
- **THEN** that pixel is unchanged
