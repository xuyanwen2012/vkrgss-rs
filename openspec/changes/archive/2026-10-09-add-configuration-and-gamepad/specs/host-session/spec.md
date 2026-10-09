# Spec Delta

## ADDED Requirements

### Requirement: Configurable key map
The key map SHALL be replaceable per button: a configuration that lists keys for a button
SHALL make exactly those keys hold it, and buttons it does not mention SHALL keep their default
keys. An unknown button or key name SHALL be an error that names it.

#### Scenario: Remapped button
- **WHEN** the configuration maps the button C to the key J only
- **THEN** J holds C, Space no longer holds C, and the arrow keys still hold the directions

#### Scenario: Unknown key name
- **WHEN** the configuration names a key that does not exist
- **THEN** loading it fails with an error naming that key

### Requirement: Gamepads
A connected gamepad SHALL hold buttons in the input snapshot together with the keyboard: its
directional pad and its left stick beyond a dead zone SHALL be the directions, and its buttons
SHALL map to RGSS buttons by a default table that the configuration can replace per button. A
pad connected or removed during a run SHALL be picked up without a restart, and a removed pad
SHALL release everything it held. A machine without gamepad support SHALL run normally.

#### Scenario: Directional pad
- **WHEN** a pad reports its directional pad held to the right
- **THEN** the snapshot has RIGHT held

#### Scenario: Stick dead zone
- **WHEN** the left stick is at 0.1 and then at 0.9 on the horizontal axis
- **THEN** RIGHT is not held and then held

#### Scenario: Keyboard and pad together
- **WHEN** the keyboard holds LEFT and a pad holds its button mapped to C
- **THEN** the snapshot has both held

#### Scenario: Removal
- **WHEN** a pad that holds a button is removed
- **THEN** the next snapshot no longer has that button held
