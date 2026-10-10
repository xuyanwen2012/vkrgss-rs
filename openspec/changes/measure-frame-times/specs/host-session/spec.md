# Spec Delta

## ADDED Requirements

### Requirement: Frame time statistics
On request the host SHALL report a summary of where frames spend their time: one row per
stage the backend measures and one for the time spent waiting for the guest, each with count,
mean, smallest and largest. A windowed run SHALL report periodically, a headless run once at
its end. The measurement SHALL NOT change pacing or anything the guest observes.

#### Scenario: Headless summary
- **WHEN** a game runs headless for 50 frames with the statistics requested
- **THEN** the error stream has one summary with a row for the guest and a row for each processor stage, each counting the frames measured

#### Scenario: Not requested
- **WHEN** the same run is made without the request
- **THEN** no summary is written

#### Scenario: Same frames
- **WHEN** the same headless run dumps a frame with and without the request
- **THEN** the two dumps are equal
