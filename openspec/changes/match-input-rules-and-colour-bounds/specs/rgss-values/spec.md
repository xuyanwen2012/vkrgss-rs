# Spec Delta

## MODIFIED Requirements

### Requirement: Color and Tone
`Color.new(red, green, blue, alpha = 255)` and `Tone.new(red, green, blue, gray = 0)` SHALL
create values whose components are Floats, clamped as `vkxp-core` defines, readable and
writable by name, settable together with `set`, comparable with `==`, `===` and `eql?`, and
printable with `to_s` as `(r, g, b, a)` with six decimals. Passing something that is not a
number SHALL raise `TypeError`, and a wrong number of arguments `ArgumentError`.

#### Scenario: Construction and clamping
- **WHEN** `c = Color.new(300, -300, 12.5, -7)` is evaluated
- **THEN** `c.red`, `c.green`, `c.blue` and `c.alpha` are `255.0`, `-255.0`, `12.5` and `0.0`

#### Scenario: Setters
- **WHEN** `t = Tone.new(0, 0, 0); t.gray = 300; t.set(-300, 1, 2)` is evaluated
- **THEN** `t.red` is `-255.0` and `t.gray` is `0.0`

#### Scenario: Equality and printing
- **WHEN** `Color.new(1, 2, 3) == Color.new(1, 2, 3, 255)` and `Color.new(1, 2, 3).to_s` are evaluated
- **THEN** the results are `true` and `"(1.000000, 2.000000, 3.000000, 255.000000)"`

#### Scenario: Wrong argument
- **WHEN** `Color.new("red", 0, 0)` is evaluated
- **THEN** a `TypeError` is raised
