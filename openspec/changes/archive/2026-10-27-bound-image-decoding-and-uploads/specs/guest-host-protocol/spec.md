## ADDED Requirements

### Requirement: Bitmap changes ahead of a frame
A guest SHALL be able to send bitmap changes in messages of their own before a frame message.
The host SHALL apply them, in the order sent, together with the next frame and before that
frame's own changes, and SHALL NOT answer them with a tick. Every bitmap the bitmap arena
accepts SHALL reach the host this way, each message under the bound of the framing.

#### Scenario: Largest bitmap
- **WHEN** a guest over a stream socket creates a bitmap of 16384 by 16384, fills it and draws it in one frame
- **THEN** the host receives every pixel, renders the frame and the session goes on

#### Scenario: No tick for changes
- **WHEN** a guest sends two messages of bitmap changes and then one frame
- **THEN** it receives exactly one tick

#### Scenario: Order
- **WHEN** a message of changes ahead of a frame updates a bitmap and the frame's own changes update the same pixels again
- **THEN** the host shows the pixels of the frame's own changes

### Requirement: One definition of the largest bitmap
The largest bitmap SHALL be defined once in `vkrgss-proto` and used by the core, the
reference compositor and the Vulkan compositor; the image decoders' limit SHALL be checked
equal to it by a test.

#### Scenario: Limits agree
- **WHEN** the tests of `vkrgss-core` run
- **THEN** a test asserts that the limit of `vkrgss-image` equals `vkrgss_proto::MAX_BITMAP_PIXELS`
