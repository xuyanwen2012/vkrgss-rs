# Spec Delta

## MODIFIED Requirements

### Requirement: Volume, pitch, stop and fade
The volume of a play command (0 to 100) SHALL set the track's gain to
`10 ^ (-0.0175 * (100 - v))`, 0.35 dB below full for every unit under 100, and to silence for 0,
where `v` is the volume, lowered in proportion during a fade. The pitch (50 to 150) SHALL
change the playback speed by that percentage. A stop command SHALL silence its channel at once.
A fade command SHALL lower `v` evenly to 0 over the given duration and then stop the channel.

#### Scenario: Half speed
- **WHEN** a 440 Hz tone is played with pitch 50
- **THEN** the output tone is 220 Hz and lasts twice as long

#### Scenario: Fade
- **WHEN** music is faded over 1000 ms
- **THEN** the output amplitude after 500 ms is about 0.133 of the amplitude before the fade, and after 1000 ms the channel is silent

#### Scenario: Volume curve
- **WHEN** the same tone is played at volumes 100, 80, 50 and 0
- **THEN** the output amplitudes are 1, about 0.447 and about 0.133 of the track's own, and silence
