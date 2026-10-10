# Spec Delta

## ADDED Requirements

### Requirement: One protected call
The crate that embeds the interpreter SHALL offer the function that runs a C-level body under
the interpreter's protection and reports a raised exception as a value, and the crate that
defines the RGSS classes SHALL use it rather than its own.

#### Scenario: Body that returns
- **WHEN** a body run through the function returns a value
- **THEN** the caller receives that value

#### Scenario: Body that raises
- **WHEN** a body run through the function raises `ArgumentError`
- **THEN** the caller receives the exception as a value, no host frame was unwound, and `$!` is nil afterwards

#### Scenario: Binding behaviour unchanged
- **WHEN** the tests of the crate that defines the RGSS classes run
- **THEN** they pass as before
