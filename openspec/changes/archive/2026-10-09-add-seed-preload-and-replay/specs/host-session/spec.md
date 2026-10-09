# Spec Delta

## ADDED Requirements

### Requirement: Input recording and replay
The host SHALL be able to record the input snapshot it delivers for every frame into a file
together with the run's seed, and to replay such a file: during a replay the recorded snapshot
of each frame SHALL be delivered in place of live input, and after the last recorded frame live
input SHALL apply again in a windowed run and no button SHALL be held in a headless run. A
file that is not a recording SHALL be refused with an error naming it.

#### Scenario: Round trip
- **WHEN** a headless run with an input script is recorded and the recording is replayed in a second headless run with the same seed
- **THEN** the frame dumps of both runs are byte-identical

#### Scenario: Not a recording
- **WHEN** a replay is asked for with a file of other content
- **THEN** the run fails before the first frame and names the file
