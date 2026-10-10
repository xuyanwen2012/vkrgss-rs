# Spec Delta

## ADDED Requirements

### Requirement: Constants shared with shaders have one definition
The numeric constants that the backend's Rust code and its shaders must agree on SHALL be
defined once, in Rust, and reach the shaders through a file the build generates.

#### Scenario: A constant changes
- **WHEN** the value of a shared constant is changed in its Rust definition and the crate is rebuilt
- **THEN** the shaders are compiled with the new value without an edit to a shader file

#### Scenario: Frames unchanged
- **WHEN** the differential tests run after the change
- **THEN** every frame equals the reference
