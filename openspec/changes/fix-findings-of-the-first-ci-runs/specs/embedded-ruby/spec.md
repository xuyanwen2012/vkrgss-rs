## ADDED Requirements

### Requirement: Hyphen after a class in a character class
In a character class of a regular expression, a hyphen that follows a class such as `\d`,
`\w` or `\s` SHALL be the literal character `-`, whichever compiler built the interpreter.

#### Scenario: Class, hyphen, letter
- **WHEN** `"1-z"`, `"A"` and `"5"` are matched against `/\A[\d-z]+\z/`
- **THEN** the first and the third match and the second does not

#### Scenario: Measured sample
- **WHEN** the samples of regular expressions measured on the original runtime are evaluated
- **THEN** every result equals the measured one

#### Scenario: Patch is recorded
- **WHEN** the manifest test of the vendored Oniguruma runs
- **THEN** it passes, and the change to `regparse.c` is described with its diff
