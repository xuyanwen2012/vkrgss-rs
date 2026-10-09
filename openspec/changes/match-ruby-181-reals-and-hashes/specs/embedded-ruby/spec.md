# Spec Delta

## MODIFIED Requirements

### Requirement: Random numbers as in Ruby 1.8.1
`srand(seed)` with an integer seed SHALL initialise the Mersenne Twister with that seed's low
32 bits by the generator's single-integer initialisation, and SHALL return the previous seed.
The generator's real number SHALL be one 32-bit output divided by 2^32. `rand(n)` for an
integer `n` other than zero SHALL return that real number multiplied by the absolute value of
`n` and truncated, and `rand` and `rand(0)` SHALL return that real number. The deviation from
the vendored version SHALL be recorded with the other patches.

#### Scenario: Values measured on the original
- **WHEN** `srand(1234); (1..12).map { rand(1000) }` is evaluated
- **THEN** the result is `[191, 497, 622, 817, 437, 612, 785, 771, 779, 860, 272, 150]`

#### Scenario: Float
- **WHEN** `srand(1234); 12.times { rand(1000) }; rand` is evaluated
- **THEN** the result is `0.276464259717613` to fifteen digits

#### Scenario: Reference values
- **WHEN** `srand(5489); [rand(1000), rand(1000), rand(1000)]` is evaluated
- **THEN** the result equals the values computed in the test from a reference MT19937 initialised with 5489 by `init_genrand`, each 32-bit output divided by 2^32, times 1000, truncated

#### Scenario: Previous seed
- **WHEN** `srand(3); srand(4)` is evaluated
- **THEN** the second call returns `3`

#### Scenario: Recorded
- **WHEN** the manifest test of the vendored tree runs
- **THEN** `random.c` matches the hash recorded for its patched state

## ADDED Requirements

### Requirement: String hash as in Ruby 1.8.1
`String#hash` SHALL be computed as the original does: starting from 0, for each byte taken as
a signed 8-bit value, `key = key * 65599 + byte`; then `key = key + (key >> 5)` with an
arithmetic shift; all in wrapping signed 32-bit arithmetic; the result reduced to a 31-bit
signed Fixnum as on a 32-bit build. No random seed SHALL enter it, so that hashes with string
keys iterate in the original's order.

#### Scenario: String hashes measured on the original
- **WHEN** the hashes of `""`, `"a"`, `"ab"`, `"abc"`, `"foo"`, `"Graphics/Characters/001-Fighter01"`, `"\xe6\xb5\x8b\xe8\xaf\x95"`, `"\xff"` and `"\x80\x7f"` are evaluated
- **THEN** they are `0`, `100`, `6562051`, `833038373`, `876516207`, `-1017680531`, `97144166`, `-2` and `-8658938`

#### Scenario: Iteration order measured on the original
- **WHEN** a hash is filled with the keys `hp sp str dex agi int atk pdef mdef eva name id x y z` in that order and its keys are listed
- **THEN** the list is `name dex x mdef atk y pdef agi z int id eva str sp hp`

#### Scenario: Stable between runs
- **WHEN** `"abc".hash` is evaluated in two separate processes
- **THEN** both results are equal

### Requirement: Float hash as in Ruby 1.8.1
`Float#hash` SHALL be the sum of the eight bytes of the double, each taken as a signed 8-bit
value and multiplied by 971, made non-negative.

#### Scenario: Float hashes measured on the original
- **WHEN** `1.5.hash` and `0.0.hash` are evaluated
- **THEN** they are `53405` and `0`
