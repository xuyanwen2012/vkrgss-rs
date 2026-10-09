# Spec Delta

## ADDED Requirements

### Requirement: Text argument errors of the original
`Font#size=` SHALL raise `ArgumentError` with the message `bad value for size` for a value
below 6 or above 96 and accept fractional values inside that range. `Bitmap#draw_text` and
`Bitmap#text_size` SHALL raise `TypeError` when the text argument is not a string.

#### Scenario: Size out of range
- **WHEN** `Font.new.size = 5` and `Font.new.size = 97` are evaluated
- **THEN** each raises `ArgumentError` with the message `bad value for size`, and `96.5` is accepted

#### Scenario: Not a string
- **WHEN** `bitmap.draw_text(0, 0, 100, 32, 123)` is evaluated
- **THEN** a `TypeError` is raised
