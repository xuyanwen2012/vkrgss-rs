## ADDED Requirements

### Requirement: Tilemap planes of RGSS3 match the reference
The Vulkan backend SHALL evaluate a tilemap plane op by the rule set it names and SHALL
produce, for ops of the rule set of RGSS3, frames byte-identical to the reference
compositor's. Frames of ops of the rule set of RGSS2 and of tilemap layer ops SHALL stay
byte-identical to the reference's as before.

#### Scenario: Topics of RGSS3
- **WHEN** every frame of the topics `tables2`, `shadow3`, `edge1` and `rand4` that the reference compositor is tested with is rendered by both backends
- **THEN** every frame of the Vulkan backend equals the reference's byte for byte

#### Scenario: Generated planes of RGSS3
- **WHEN** randomly generated packets with plane ops of the rule set of RGSS3 are rendered by both backends, with maps of one to five layers, flags tables of any size and none in which the bits 0x10 and 0x80 are common, elements of layer 3 of any value, numbers of every range, sheets of any size and none, any clip, corner, first cell, animation step and flash level
- **THEN** every frame equals the reference compositor's byte for byte

#### Scenario: Clipped planes of RGSS3
- **WHEN** plane ops of the rule set of RGSS3 with tables, edges and shadows are rendered with clips that cover a few pixels of a few tiles of the binning grid
- **THEN** every frame equals the reference compositor's byte for byte

#### Scenario: RGSS1 and RGSS2 unchanged
- **WHEN** the tests of `crates/vkxp-vk/src/tests/tilemap.rs` and `crates/vkxp-vk/src/tests/tilemap_rgss2.rs` that existed before this change run
- **THEN** they pass, the generated test of RGSS2 with the seed and the cases it had

#### Scenario: Record layout
- **WHEN** the test of the record layouts and the test of the shared constants run after the change
- **THEN** both pass, and the tilemap record has the number of words it had
