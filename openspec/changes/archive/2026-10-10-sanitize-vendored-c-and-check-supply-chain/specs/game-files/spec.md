# Spec Delta

## ADDED Requirements

### Requirement: Arbitrary archive bytes
Opening an archive and reading its entries SHALL end in a result or an error for arbitrary
input bytes, without a panic, and without allocating for a declared length that exceeds the
bytes present.

#### Scenario: Random tail
- **WHEN** a valid header is followed by arbitrary bytes
- **THEN** opening returns a directory or an error, and reading each listed entry returns bytes or an error

#### Scenario: Damaged valid archive
- **WHEN** a valid archive has bytes flipped or is cut at an arbitrary length
- **THEN** opening and reading return results or errors and do not panic

#### Scenario: Huge declared length
- **WHEN** an entry declares a length of four gigabytes in a file of one kilobyte
- **THEN** an error is returned
