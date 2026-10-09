# Spec Delta

## ADDED Requirements

### Requirement: Audio commands
The session SHALL pass every audio command of the guest to its audio sink in the order
received and without waiting for it. A headless run SHALL use a sink that plays nothing and
reports nothing.

#### Scenario: Order
- **WHEN** the guest sends play, fade and stop commands between frames
- **THEN** a recording sink has received the same commands in the same order

#### Scenario: Headless
- **WHEN** a game that plays music runs headless
- **THEN** no audio device is opened and no audio report is printed
