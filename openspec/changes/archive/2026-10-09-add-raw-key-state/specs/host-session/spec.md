# Spec Delta

## ADDED Requirements

### Requirement: Raw keys and pointer from the window
A windowed session SHALL set in each snapshot the virtual-key bits of the keys and mouse
buttons that are held, by the Windows virtual-key code of each physical key, and the pointer
position mapped from the window into the frame, or none when the pointer is outside the
picture. Headless runs SHALL deliver neither unless a replayed recording holds them.

#### Scenario: Letter and modifier
- **WHEN** the keys A and left Shift are held
- **THEN** the snapshot has the virtual keys 0x41, 0x10 and 0xA0 set

#### Scenario: Pointer in a letterboxed window
- **WHEN** a 640 by 480 frame is shown at scale 2 with bars of 100 pixels left and right and the pointer is at window position (300, 200)
- **THEN** the snapshot's pointer is (100, 100), and at window position (50, 200) there is none

### Requirement: Raw keys in recordings
Recordings SHALL store the raw keys and the pointer of every frame, and recordings made before
this change SHALL still replay.

#### Scenario: Old recording
- **WHEN** a recording of format version 1 is replayed
- **THEN** it replays as before with no raw keys
