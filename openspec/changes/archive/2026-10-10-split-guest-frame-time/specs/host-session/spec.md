# Spec Delta

## ADDED Requirements

### Requirement: Guest time is split
When frame time statistics are requested, the guest SHALL report, per frame, the time it
spends in bitmap operations, in text drawing, in frame extraction and in the rest of its
frame, as rows with count, mean, smallest and largest. Without the request the guest SHALL
measure nothing, and the measurement SHALL NOT change anything a script observes or any frame.

#### Scenario: Rows of a headless run
- **WHEN** a game that draws text into a bitmap every frame runs headless for 50 frames with the statistics requested
- **THEN** the error stream has rows for the guest's scripts, bitmap operations, text and extraction, each counting the frames measured

#### Scenario: Not requested
- **WHEN** the same run is made without the request
- **THEN** no such row is written

#### Scenario: Same frames
- **WHEN** the same run dumps a frame with and without the request
- **THEN** the two dumps are equal
