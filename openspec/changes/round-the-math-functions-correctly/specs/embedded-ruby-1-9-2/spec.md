## ADDED Requirements

### Requirement: Mathematical functions are correctly rounded
For a regular argument, the functions of `Math` and the power of floats SHALL return the
correctly rounded result, computed by the vendored CORE-MATH, on every platform. Special
arguments and errors SHALL behave as before.

#### Scenario: Test vectors
- **WHEN** each line of the committed test vectors for a function the interpreter has is evaluated through `Math` or `**`
- **THEN** the result has exactly the bits of the line's result

#### Scenario: Special arguments
- **WHEN** `Math.log(-1)`, `Math.sqrt(-1)`, `Math.acos(2)`, `Math.atanh(1)`, `Math.log(0)`, `Math.exp(1000)` and `0.0 ** -1` are evaluated
- **THEN** each raises the same error or returns the same value as before this requirement

### Requirement: No contraction of floating-point operations
The interpreter and the vendored CORE-MATH SHALL be compiled with `-ffp-contract=off`.

#### Scenario: Flags of the build
- **WHEN** the flags of the build script are read
- **THEN** they contain `-ffp-contract=off` for the interpreter and for CORE-MATH
