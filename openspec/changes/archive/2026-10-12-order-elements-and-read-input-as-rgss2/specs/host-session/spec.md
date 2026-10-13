## ADDED Requirements

### Requirement: Keys of VX
A windowed session of a game of RGSS2 SHALL map the keyboard as the original runtime of VX
does: C from Enter, Space and Z; B from Esc, Insert, X and numpad 0; A from Shift, which also
sets SHIFT; X, Y, Z from A, S, D; L from Q and Page Up; R from W and Page Down; the directions
from the arrows and numpad 2, 4, 6, 8; CTRL, ALT and F5 to F9 from their keys. A game of
RGSS1 SHALL keep its map.

#### Scenario: Z confirms in VX
- **WHEN** the key Z is held in a session of RGSS2, and in a session of RGSS1
- **THEN** the snapshot of the first has C and that of the second has A

#### Scenario: Shift, measured on the original
- **WHEN** Shift is held in a session of RGSS2
- **THEN** the snapshot has A and SHIFT

### Requirement: Mask thresholds of RGSS2
For a masked transition with the rule of RGSS2 the threshold of frame k below N SHALL be
`T = 256 * k / N - 1`, floored at 0, as measured on the original runtime of VX, and the code
SHALL name that measurement as its provenance.

#### Scenario: Measured samples
- **WHEN** each committed sample of a masked transition measured on the original runtime of VX is evaluated at its frame, mask level and vagueness
- **THEN** the shown value equals the measured value
