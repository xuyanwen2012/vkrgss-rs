## ADDED Requirements

### Requirement: Built-in scripts of RGSS3 are bytes
In a game of RGSS3 the built-in scripts file SHALL be evaluated from a string of bytes, so
that a string literal of it is ASCII-8BIT, while a literal of a game script SHALL be UTF-8.

#### Scenario: Two literals
- **WHEN** a built-in scripts file assigns `''` to a global and a game script of RGSS3 does the same to another
- **THEN** the first string's encoding is ASCII-8BIT and the second's UTF-8

#### Scenario: A dump
- **WHEN** the built-in scripts file dumps `['', 'a']` with `Marshal.dump`
- **THEN** the dump is `04085b072200220661` in hexadecimal: neither string carries an encoding
