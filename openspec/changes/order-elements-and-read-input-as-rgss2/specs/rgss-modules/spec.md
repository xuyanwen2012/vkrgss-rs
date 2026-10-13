## ADDED Requirements

### Requirement: Small rules of RGSS2 in the modules and elements
In RGSS2 the vagueness of `Graphics.transition` SHALL default to 40, and `blend_type=` of a
sprite and of a plane SHALL raise `ArgumentError` with the message `bad value for blend_type`
for a value that, truncated, is not 0, 1 or 2. In RGSS1 both SHALL stay as they are.

#### Scenario: Default vagueness, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.freeze; Graphics.transition(5, "Graphics/System/Mask")` and the picture exists
- **THEN** the transition message carries the vagueness 40

#### Scenario: Blend type, measured on the original
- **WHEN** a game of RGSS2 evaluates `s.blend_type = 1.9; s.blend_type` and then `s.blend_type = 3`
- **THEN** the value is `1` and the assignment raises `ArgumentError`

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `s.blend_type = 3`
- **THEN** no error is raised
