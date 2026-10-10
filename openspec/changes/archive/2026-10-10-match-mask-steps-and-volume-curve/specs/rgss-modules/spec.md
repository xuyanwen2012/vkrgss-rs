# Spec Delta

## MODIFIED Requirements

### Requirement: Freeze and transition
`Graphics.freeze` SHALL send the host a freeze message. `Graphics.transition(duration = 8,
filename = nil, vague = 20)` SHALL send a transition message with that duration, the bitmap
loaded from `filename` as mask when one is given, and the vagueness, and SHALL then send one
frame and wait for its tick `duration` times before returning. A filename that is not found
SHALL raise `Errno::ENOENT`.

#### Scenario: Default transition
- **WHEN** `Graphics.freeze; Graphics.transition` is evaluated
- **THEN** the host receives a freeze message, a transition message with duration 8 and no mask, and then eight frame messages

#### Scenario: Zero duration
- **WHEN** `Graphics.transition(0)` is evaluated
- **THEN** a transition message with duration 0 is sent and no frame message

#### Scenario: Transition picture
- **WHEN** `Graphics.transition(40, "Graphics/Transitions/004-Blind04")` is evaluated and the picture exists
- **THEN** the transition message names a mask bitmap, and the first frame message that follows carries the creation of that bitmap

#### Scenario: Default vagueness
- **WHEN** `Graphics.transition(5, "Graphics/Transitions/004-Blind04")` is evaluated and the picture exists
- **THEN** the transition message carries the vagueness 20
