## MODIFIED Requirements

### Requirement: Frames match the reference
For every sequence of messages that the CPU reference compositor renders, the Vulkan backend
SHALL produce frames that are byte-identical to the reference's, on every supported device.

#### Scenario: Committed fixtures
- **WHEN** each fixture message stream is rendered by the Vulkan backend
- **THEN** every frame equals the reference compositor's frame byte for byte

#### Scenario: Generated packets
- **WHEN** randomly generated valid packets are rendered by both backends, covering all three
  op kinds, all blend modes, quads with and without a tone and a colour, clipping, non-identity
  transforms and bitmap reuse across packets
- **THEN** every frame equals the reference compositor's frame byte for byte

#### Scenario: Arithmetic sweep
- **WHEN** frames are rendered that pair every source channel value with every destination
  channel value under each blend mode, colour blend and tone
- **THEN** every frame equals the reference compositor's frame byte for byte

#### Scenario: Quad tone and colour sweep
- **WHEN** frames are rendered in which quads carry every grey amount, the extreme and zero
  tone offsets, and every colour alpha, over every source channel value
- **THEN** every frame equals the reference compositor's frame byte for byte
