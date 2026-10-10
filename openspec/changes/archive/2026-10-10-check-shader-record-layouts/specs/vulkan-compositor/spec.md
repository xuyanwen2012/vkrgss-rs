# Spec Delta

## ADDED Requirements

### Requirement: Record layouts are checked
A test SHALL show, for every field of the op record and of the tilemap record, that the
compute shader reads it from the word the backend writes it to, and that the fields together
cover every word of each record.

#### Scenario: Every field
- **WHEN** records whose words hold distinct values are read by a shader that uses the declarations of the compose shader
- **THEN** each field has the value of the word named for it on the Rust side

#### Scenario: A field moved on one side
- **WHEN** two fields are swapped in the shader's declaration only
- **THEN** the test fails and names the fields

#### Scenario: Frames unchanged
- **WHEN** the differential tests run after the change
- **THEN** every frame equals the reference
