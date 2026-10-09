# Spec Delta

## ADDED Requirements

### Requirement: Own font metrics option
The program SHALL accept `--font-own-metrics`, and the configuration key `font_own_metrics`,
which make the game's text use fonts' own line boxes.

#### Scenario: Option
- **WHEN** a generated game draws text with a replaced font name and is run headless with and without `--font-own-metrics` and a font directory holding a test font
- **THEN** the two dumps differ in the text's size
