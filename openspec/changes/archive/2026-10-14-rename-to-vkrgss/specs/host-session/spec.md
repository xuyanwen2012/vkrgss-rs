## ADDED Requirements

### Requirement: Recordings under the former name
A recording SHALL be written with the first line `vkrgss-input 2 seed=<n>`. A recording whose
first line is `vkxp-input 1 seed=<n>` or `vkxp-input 2 seed=<n>`, as versions 0.1.0 to 0.3.0
wrote them, SHALL replay as the same recording with `vkrgss-input` in the place of
`vkxp-input`, without a warning.

#### Scenario: Written
- **WHEN** a headless run with an input script is recorded with seed 7
- **THEN** the first line of the file is `vkrgss-input 2 seed=7`

#### Scenario: Recording of version 0.3.0
- **WHEN** that recording, its first line changed to `vkxp-input 2 seed=7`, is replayed in a headless run
- **THEN** the frame dumps equal those of the replay of the recording as written, and the error stream has no warning

#### Scenario: Format 1 of version 0.1.0
- **WHEN** a recording whose first line is `vkxp-input 1 seed=0` is parsed
- **THEN** it parses as the same recording with the first line `vkrgss-input 1 seed=0`, with no raw keys

## MODIFIED Requirements

### Requirement: Reports carry a severity
A report of the session or of a headless run SHALL state whether it is an error, a warning, or
text an option asked for. A rejected frame and a display that cannot be frozen SHALL be
errors.

#### Scenario: Rejected frame at error level
- **WHEN** a headless run whose guest sends a frame the backend rejects is made with `VKRGSS_LOG=error`
- **THEN** the error stream names the rejected frame

#### Scenario: Degradation is a warning
- **WHEN** a transition names a mask bitmap that does not exist
- **THEN** the report is a warning and the transition is a plain cross-fade

#### Scenario: Text unchanged
- **WHEN** the existing tests that read report lines run
- **THEN** the lines they expect are the lines written
