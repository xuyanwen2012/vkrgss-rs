## ADDED Requirements

### Requirement: A range that starts with a class is an error
In a character class of a regular expression, a range whose start is a class such as `\d`,
`\w` or `\s` SHALL be a `SyntaxError` with the message of the original runtime, "mismatch
multibyte code length in char-class range", whichever compiler built the interpreter. A
hyphen that ends the character class SHALL stay a literal hyphen.

#### Scenario: Class, hyphen, letter
- **WHEN** `/[\d-z]+/` is evaluated
- **THEN** a `SyntaxError` is raised whose message holds "mismatch multibyte code length in char-class range"

#### Scenario: Hyphen at the end
- **WHEN** `"1-"` is matched against `/\A[\d-]+\z/`
- **THEN** it matches

#### Scenario: Measured sample
- **WHEN** the samples of regular expressions measured on the original runtime are evaluated
- **THEN** every result equals the measured one, the one for `/[\d-z]+/` without an adjustment of its message

#### Scenario: Patch is recorded
- **WHEN** the manifest test of the vendored Oniguruma runs
- **THEN** it passes, and the change to `regparse.c` is described with its diff
