# Spec Delta

## ADDED Requirements

### Requirement: Own line box on request
When the engine is configured to use fonts' own metrics, a font found for a requested name,
whether by that name or as a replacement, SHALL be sized so that its own line box (ascent plus
descent) equals the requested size, and no scaling towards the line box of a replaced font
SHALL be applied. Without that configuration the behaviour SHALL be unchanged.

#### Scenario: Replacement font at its own size
- **WHEN** a replaced Windows font name resolves to a test font and text of size 22 is measured with own metrics on
- **THEN** the measured height is 22 and the width equals the width measured when the test font is requested by its own name

#### Scenario: Default unchanged
- **WHEN** the same text is measured without the configuration
- **THEN** the result equals what it was before this change
