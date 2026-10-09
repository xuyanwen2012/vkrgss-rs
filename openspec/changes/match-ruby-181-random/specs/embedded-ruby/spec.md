# Spec Delta

## ADDED Requirements

### Requirement: Random numbers as in Ruby 1.8.1
`srand(seed)` with an integer seed SHALL initialise the Mersenne Twister with that seed's low
32 bits by the generator's single-integer initialisation, and SHALL return the previous seed.
`rand(n)` for an integer `n` other than zero SHALL return the generator's 53-bit real number
multiplied by the absolute value of `n` and truncated, and `rand` and `rand(0)` SHALL return
that real number. The deviation from the vendored version SHALL be recorded with the other
patches.

#### Scenario: Reference values
- **WHEN** `srand(5489); [rand(1000), rand(1000), rand(1000)]` is evaluated
- **THEN** the result equals the values computed in the test from a reference implementation of MT19937 initialised with 5489 by `init_genrand`, taking `(a >> 5, b >> 6)` pairs of 32-bit outputs as 53-bit reals

#### Scenario: Float
- **WHEN** `srand(1); rand` is evaluated
- **THEN** the result is `0.417022004702574` to fifteen digits

#### Scenario: Previous seed
- **WHEN** `srand(3); srand(4)` is evaluated
- **THEN** the second call returns `3`

#### Scenario: Recorded
- **WHEN** the manifest test of the vendored tree runs
- **THEN** `random.c` matches the hash recorded for its patched state
