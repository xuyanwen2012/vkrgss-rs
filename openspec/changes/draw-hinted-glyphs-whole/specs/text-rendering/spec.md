## ADDED Requirements

### Requirement: Hinted glyph images are whole
Under the metric rule of RGSS3 the image of a glyph SHALL be drawn from its whole hinted
outline: its rows and columns of ink SHALL lie within its hinted box, and its rows of ink
SHALL span the plain glyph's rows within one pixel at each end, for a font of each hinting
engine.

#### Scenario: Auto-hinter
- **WHEN** the letters `H`, `g`, `j`, `M` and `!` of a test font on the auto-hinter path are drawn at 12, 20 and 24 pixels under the metric rule of RGSS3
- **THEN** each image's first and last row of ink are within one pixel of the plain image's, and its ink lies inside its hinted box

#### Scenario: Interpreter and CFF
- **WHEN** the same letters of a test font on the TrueType interpreter path and of one on the CFF path are drawn the same way
- **THEN** the same holds

#### Scenario: A line of text
- **WHEN** `Mio attacks!` is drawn at 24 pixels with the auto-hinted test font into a bitmap under the rules of RGSS3
- **THEN** the rows of ink of the line span at least two thirds of the rows that the same line drawn with the plain rule spans
