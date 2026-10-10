# Spec Delta

## MODIFIED Requirements

### Requirement: Colour components
A colour SHALL hold red, green, blue and alpha as floating-point numbers; whenever a component
is set, red, green and blue SHALL be clamped to the range -255 to 255 and alpha to 0 to 255.
A colour created without an alpha SHALL have alpha 255. Two colours SHALL be equal exactly
when all four components are equal.

#### Scenario: Components are clamped
- **WHEN** a colour is given red 300, green -300 and alpha -7
- **THEN** it reports red 255, green -255 and alpha 0

#### Scenario: Negative components are kept
- **WHEN** a colour is given green -5
- **THEN** it reports green -5

#### Scenario: Fractions are kept
- **WHEN** a colour is given blue 12.5
- **THEN** it reports blue 12.5

#### Scenario: Default alpha
- **WHEN** a colour is created from red, green and blue only
- **THEN** its alpha is 255

#### Scenario: Equality compares every component
- **WHEN** two colours differ only in alpha
- **THEN** they are not equal
