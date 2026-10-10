# Spec Delta

## ADDED Requirements

### Requirement: Reports carry a severity
A report of the session or of a headless run SHALL state whether it is an error, a warning, or
text an option asked for. A rejected frame and a display that cannot be frozen SHALL be
errors.

#### Scenario: Rejected frame at error level
- **WHEN** a headless run whose guest sends a frame the backend rejects is made with `VKXP_LOG=error`
- **THEN** the error stream names the rejected frame

#### Scenario: Degradation is a warning
- **WHEN** a transition names a mask bitmap that does not exist
- **THEN** the report is a warning and the transition is a plain cross-fade

#### Scenario: Text unchanged
- **WHEN** the existing tests that read report lines run
- **THEN** the lines they expect are the lines written
