## ADDED Requirements

### Requirement: Memory of a JPEG decode
Decoding a JPEG SHALL NOT allocate memory for coefficients or samples of a component before
a scan that codes that component has been read, and the memory a decode holds at once SHALL
stay within a stated multiple of its RGBA output. When the decoder that follows the original
gives up, the reason SHALL be logged before the file is decoded another way.

#### Scenario: Header without a scan
- **WHEN** a JPEG of 200 bytes whose frame header declares 16384 by 16384 and that has no scan is decoded under a counting allocator
- **THEN** decoding fails with an error and less than one megabyte was allocated

#### Scenario: Bounded memory
- **WHEN** a valid 4:4:4 JPEG of 1024 by 1024 is decoded under a counting allocator
- **THEN** the largest amount held at once is within the stated multiple of its 4 MiB output

#### Scenario: Reason of a fallback
- **WHEN** a JPEG that the decoder of the original does not handle is decoded
- **THEN** the image is returned and a debug log line names why that decoder gave up

#### Scenario: Pixels unchanged
- **WHEN** the committed JPEG fixtures are decoded
- **THEN** every pixel equals the pixel decoded before this change
