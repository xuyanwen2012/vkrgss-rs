# Spec Delta

## ADDED Requirements

### Requirement: Concurrent runs
Several harness runs with different output directories SHALL be able to run at the same time
on one machine: each SHALL use a virtual display of its own, chosen by the display server, and
SHALL stop only the display server and Wine server that it started.

#### Scenario: Display chosen by the server
- **WHEN** the harness starts its virtual display while another display server already runs on the number the harness would have probed first
- **THEN** it obtains another number from the server it started and connects to that

#### Scenario: Server that fails to start
- **WHEN** the display server exits without announcing a display
- **THEN** the run ends with an error that says so, within the start-up timeout
