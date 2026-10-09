# Spec Delta

## ADDED Requirements

### Requirement: Audio options
A windowed run SHALL play audio through the default output device. `--no-audio` SHALL disable
it. `--soundfont FILE` SHALL name the SoundFont for MIDI; without it the program SHALL look in
the usual system locations and, finding none, play MIDI as silence with one report.

#### Scenario: Disabled
- **WHEN** the program is run with `--no-audio`
- **THEN** no audio device is opened

#### Scenario: SoundFont missing
- **WHEN** `--soundfont` names a file that does not exist
- **THEN** the program ends with an error naming the file before any window is opened
