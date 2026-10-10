# vulkan-compositor Specification

## Purpose
Defines the Vulkan compositor backend: what it needs from a device, that its frames and errors
match the CPU reference compositor, and how it behaves at device limits.

## Requirements

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

### Requirement: Device selection
The Vulkan backend SHALL let the caller choose the device by index or by a substring of its
name, and SHALL otherwise prefer a discrete GPU, then an integrated GPU, then any other suitable
device.

#### Scenario: Explicit choice
- **WHEN** the caller names a suitable device
- **THEN** that device is used even if a preferred kind is present

#### Scenario: Named device is missing
- **WHEN** the caller names a device that does not exist or is unsuitable
- **THEN** initialisation fails and lists the available devices, without falling back

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

### Requirement: Errors match the reference
For every packet the CPU reference compositor rejects, the Vulkan backend SHALL reject it with
an error of the same kind that names the same op or delta index, and SHALL keep the same bitmap
state afterwards.

#### Scenario: Reserved op
- **WHEN** a packet contains a reserved op kind
- **THEN** the backend returns an unsupported-op error naming the kind and index, and no frame

#### Scenario: Stale bitmap reference
- **WHEN** an op refers to a destroyed bitmap whose slot has been reused
- **THEN** the backend returns an invalid-reference error and does not draw the new occupant

#### Scenario: Generated invalid packets
- **WHEN** randomly generated packets that contain invalid deltas or ops are given to both
  backends, followed by a valid packet
- **THEN** both reject the same packets with the same error kind and index, and the frames of
  the valid packet are byte-identical

### Requirement: Tile binning is not observable
The division of the screen into tiles and the assignment of ops to tiles SHALL NOT change any
output pixel, including for logical sizes that are not a multiple of the tile size and for ops
that span, touch or lie entirely outside tile boundaries.

#### Scenario: Logical size not a multiple of the tile size
- **WHEN** a frame is rendered at a logical size such as 641 by 479
- **THEN** it equals the reference compositor's frame byte for byte

#### Scenario: Many ops over one pixel
- **WHEN** several thousand ops cover the same pixel
- **THEN** all of them are applied in order and the frame equals the reference's

### Requirement: Device limits are reported
When a packet is valid for the reference compositor but exceeds a limit of the device or of the
backend, the Vulkan backend SHALL return a typed limit error that names the limit and the
requested value, and SHALL NOT produce a partial or different frame.

#### Scenario: Bitmap larger than the device supports
- **WHEN** a bitmap is created whose width or height exceeds the device's image size limit
- **THEN** the backend returns a limit error naming the limit and the requested size

#### Scenario: Recovery after a limit error
- **WHEN** a packet fails with a limit error and a later packet stays within the limits
- **THEN** the later packet renders normally

### Requirement: Clean under validation
The Vulkan backend SHALL NOT cause any Vulkan validation error. Its tests SHALL run with the
validation layer enabled when the layer is installed and SHALL fail when a validation error is
reported.

#### Scenario: Validation error fails a test
- **WHEN** the validation layer reports an error during a test
- **THEN** that test fails and shows the message

#### Scenario: Resources are released
- **WHEN** a backend that has rendered frames and destroyed bitmaps is dropped
- **THEN** the validation layer reports no leaked object

### Requirement: Shaders are built and validated with the workspace
Building the workspace SHALL compile the backend's shaders from source and validate the result,
and SHALL fail with a message naming the missing tool when the shader compiler or validator is
not installed. Compiled shader binaries SHALL NOT be committed.

#### Scenario: Shader error stops the build
- **WHEN** a shader source has a compile error
- **THEN** the workspace build fails and shows the compiler's message

#### Scenario: Missing tool
- **WHEN** the shader compiler is not installed
- **THEN** the build fails with a message that names the tool

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
