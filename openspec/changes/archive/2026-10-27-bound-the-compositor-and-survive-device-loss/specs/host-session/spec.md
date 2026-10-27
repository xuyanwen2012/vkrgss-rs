## ADDED Requirements

### Requirement: A lost device ends the session
When the compositor reports a lost device, the host SHALL report it once on the error stream and
end the session with a failure status, in a windowed and in a headless run, instead of treating
each later frame as rejected.

#### Scenario: Windowed
- **WHEN** the compositor of a windowed session reports a lost device for a frame
- **THEN** one error is reported and the session ends with a failure status

#### Scenario: Headless
- **WHEN** the compositor of a headless run reports a lost device
- **THEN** the run ends with a failure status and one error on the error stream
