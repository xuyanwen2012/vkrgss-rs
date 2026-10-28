## ADDED Requirements

### Requirement: One wait per shown frame
The host SHALL wait for the device once per shown frame: `render` SHALL submit without
waiting, and `present` SHALL wait once for the compose and the presentation pass together. A
freeze SHALL wait once. At most the compose of the last `render` SHALL be in use by the
device when a method returns; every method that records, reads back, replaces or destroys
SHALL wait for it first, and so SHALL dropping the compositor.

#### Scenario: One wait per shown frame
- **WHEN** a frame is rendered and then presented on the headless surface
- **THEN** the host waits for the device once and the presented picture equals the reference

#### Scenario: Restated contract
- **WHEN** the doc comment of `VkCompositor` is read
- **THEN** it says that at most the compose of the last `render` is in use when a method returns, and which methods wait for it

#### Scenario: Calls right after a render
- **WHEN** each public method of the compositor is called right after `render`, under the validation layer
- **THEN** each gives the result it gave before this change and the validation layer reports no error

#### Scenario: A freeze waits once
- **WHEN** a frame is frozen
- **THEN** its clear and copy are one submission with one wait and the held picture is as before

#### Scenario: Same frames
- **WHEN** the differential tests and `cargo xtask xarch check` run
- **THEN** every frame equals the reference and every game is identical to the baseline
