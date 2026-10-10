# Spec Delta

## ADDED Requirements

### Requirement: Saving a state
A guest that is asked to save a state SHALL have itself copied at a wait for a tick whose
frame was sent while the display was not frozen and no modal message was shown, and SHALL go
on unchanged. A guest that cannot be copied SHALL answer that the request failed.

#### Scenario: Unchanged run
- **WHEN** a game is run twice with the same seed and input, once with a save request at frame 50
- **THEN** the frames the guest sends are the same in both runs

#### Scenario: Frozen display
- **WHEN** the request arrives between `Graphics.freeze` and the end of the transition that follows
- **THEN** the copy is made at the first frame after the transition

#### Scenario: No copier
- **WHEN** the guest runs without a way to copy itself, as a thread does
- **THEN** it answers the request with the failure message and goes on

### Requirement: Resuming a state
A resumed copy SHALL send its title, the play commands of the music and background sound it
remembers and the frame it was waiting on, with every live bitmap and table in full, and SHALL
then continue from that wait.

#### Scenario: Complete frame
- **WHEN** a copy made while three bitmaps and a tilemap's tables were live is resumed
- **THEN** the first frame it sends creates and fills the three bitmaps and sets the tables, and a compositor that held nothing renders it like the frame of the original wait

#### Scenario: Music
- **WHEN** the game was playing a music file when the copy was made
- **THEN** the resumed copy sends a play command for that file before its frame
