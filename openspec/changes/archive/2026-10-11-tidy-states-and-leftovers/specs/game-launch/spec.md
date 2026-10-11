## ADDED Requirements

### Requirement: Socket directories of ended runs
Starting a guest process SHALL remove the socket directories that earlier runs of the same
user left behind and whose process no longer exists, and SHALL leave every other entry of
the directory they are in alone.

#### Scenario: Left by a killed run
- **WHEN** the base directory holds a socket directory named after a process that no longer exists, one named after a running process and a file of another name, and a run with `--guest process` starts
- **THEN** the first is gone and the other two are still there
