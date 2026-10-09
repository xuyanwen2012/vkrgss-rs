# Spec Delta

## ADDED Requirements

### Requirement: Class redefinition with another superclass as in Ruby 1.8.1
A `class` statement that names an existing class together with an explicit superclass that is
not that class's superclass SHALL NOT raise: it SHALL create a new class with the given
superclass and bind the constant to it, leaving the old class object and its instances as they
were. A `class` statement without a superclass, or with the class's own superclass, SHALL
reopen the existing class. Naming a module in a `class` statement SHALL still raise
`TypeError`.

#### Scenario: Measured on the original
- **WHEN** `class KA; def x; end; end; class KB; end; old = KA; class KA < KB; def y; end; end` is evaluated
- **THEN** no error is raised, `KA` is not the same object as `old`, `KA.superclass` is `KB`, a new `KA` instance responds to `y` and not to `x`, and `old.name` is still `"KA"`

#### Scenario: Explicit Object
- **WHEN** `class KC < KB; end; class KC < Object; end` is evaluated
- **THEN** no error is raised and `KC.superclass` is `Object`

#### Scenario: Reopening
- **WHEN** `class KA; def z; end; end` is evaluated for an existing `KA`
- **THEN** `KA` is the same class object as before with the new method

#### Scenario: Module
- **WHEN** `module KM; end; class KM; end` is evaluated
- **THEN** a `TypeError` is raised
