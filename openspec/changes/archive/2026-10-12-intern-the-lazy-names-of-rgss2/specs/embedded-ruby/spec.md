## ADDED Requirements

### Requirement: Names the interpreter interns as Ruby 1.8.1 does
The vendored interpreter SHALL intern `recv` when method dispatch first raises a
`NoMethodError` or `NameError`, and SHALL keep the members of a `Struct` class under the name
`__member__`, as the start-up symbol tables of the original runtimes show.

#### Scenario: Name error, measured on the original
- **WHEN** a script notes the number of a new name, rescues `nil.undefined_method_of_the_test`, and notes the number of another new name, and then does the same again
- **THEN** the second name is 2 after the first the first time and 1 after it the second time, counted in the sequence

#### Scenario: Struct, measured on the original
- **WHEN** a script notes the number of a new name, evaluates `Struct.new(:left, :top)` with member names it used before, and notes the number of another new name
- **THEN** the second name follows the first directly in the sequence
