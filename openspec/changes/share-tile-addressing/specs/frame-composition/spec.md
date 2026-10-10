# Spec Delta

## ADDED Requirements

### Requirement: One definition of the tile numbering
The rules that map a tile number to a sheet and a texel (the tile and quarter sizes, the
ranges of autotile and tileset numbers, the autotile quarter table, the single-tile sheet
rule) SHALL have one Rust definition, in the crate of the shared protocol types, which the
guest's extraction and the reference compositor both use. The shader's copy of the quarter
table SHALL be compared with it by a test.

#### Scenario: Shader table
- **WHEN** the shader's autotile quarter table is compared with the shared definition
- **THEN** the 192 entries are equal

#### Scenario: No guest-side composition
- **WHEN** the guest crates are built for tests
- **THEN** no code path composes a tilemap into a bitmap
