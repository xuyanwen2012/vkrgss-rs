# Spec Delta

## ADDED Requirements

### Requirement: Per-frame buffers are reused
The backend SHALL keep the buffers that carry a frame's ops, tile lists, tilemap records and
bitmap uploads from one frame to the next, replacing one only when a frame needs more room
than it has.

#### Scenario: Steady frames
- **WHEN** the same frame packet is rendered twenty times after a first render
- **THEN** no buffer is created after the first render and every frame equals the reference

#### Scenario: A larger frame
- **WHEN** a frame with many more ops than the one before is rendered, then the small one again
- **THEN** both equal the reference and the validation layer reports no error

#### Scenario: A buffer cannot be had
- **WHEN** creating or growing one of the buffers fails for one frame
- **THEN** that frame fails as an allocation failure does today and the next frame renders correctly
