## ADDED Requirements

### Requirement: Values held only by an inner pointer survive an allocation
Where the code of Ruby 1.9.2 reads an object through a pointer into it after a call that can
allocate, the object SHALL be kept alive until that read, on every supported architecture.

#### Scenario: Arithmetic of two-digit Bignums under stress
- **WHEN** `(2**63 - 1) + 1` and `(2**63 - 1) - (-1)` are evaluated with `GC.stress = true`
- **THEN** both equal `2**63`

#### Scenario: Time at a file under stress
- **WHEN** `File.utime` is called with `Time.local(2000, 1, 1)` for a file, with `GC.stress = true`
- **THEN** it succeeds

#### Scenario: Marshal of a Time with a fraction under stress
- **WHEN** `Marshal.dump` of `Time.at(0, Rational(1, 3))` and of `Time.at(0, 0.123)` runs with `GC.stress = true`
- **THEN** each returns a string that `Marshal.load` turns back into an equal time

#### Scenario: Patches are recorded
- **WHEN** the vendored source is checked against its manifest
- **THEN** every guard added is part of a patch recorded in `PATCHES.md`
