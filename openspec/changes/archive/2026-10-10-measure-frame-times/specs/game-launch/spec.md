# Spec Delta

## ADDED Requirements

### Requirement: Frame statistics option
The program SHALL accept `--frame-stats`, which requests the frame time statistics of the
host session, in windowed and in headless mode.

#### Scenario: Option accepted
- **WHEN** the program is run with `--frame-stats` in headless mode
- **THEN** it runs the game and writes the summary to the error stream
