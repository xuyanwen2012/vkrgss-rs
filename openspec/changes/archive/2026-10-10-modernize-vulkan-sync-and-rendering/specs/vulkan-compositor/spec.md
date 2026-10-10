# Spec Delta

## MODIFIED Requirements

### Requirement: Device requirements
The Vulkan backend SHALL run on any device that supports Vulkan 1.3 with the descriptor indexing
features it uses, and SHALL NOT require any other optional feature or extension. Of the
features that Vulkan 1.3 makes mandatory it enables synchronization2 and dynamic rendering. It
SHALL NOT require a window, surface or display to produce a frame.

#### Scenario: Software rasteriser
- **WHEN** the only Vulkan device is a CPU implementation that supports Vulkan 1.3
- **THEN** the backend initialises and produces frames

#### Scenario: Unsuitable device
- **WHEN** no available device meets the requirements
- **THEN** initialisation returns an error that names each device and what it lacks

#### Scenario: No display
- **WHEN** the process has no window system connection
- **THEN** the backend still initialises and produces frames

## ADDED Requirements

### Requirement: Barriers name the stages that use an image
Each change of an image's layout or use SHALL be recorded with the pipeline stages and access
kinds of the commands before and after it, in the synchronization2 form. The composed frame's
passage from the compute dispatch to the presentation pass, and the presentation pass's reads
of the held frame and of a transition mask, SHALL be covered by such barriers.

#### Scenario: Compose, freeze, transition and present
- **WHEN** frames are composed and presented through a freeze and a masked transition with the validation layer active
- **THEN** the validation layer reports no error

#### Scenario: Same pixels
- **WHEN** the differential tests of the backend run
- **THEN** every frame equals the reference compositor's, as before
