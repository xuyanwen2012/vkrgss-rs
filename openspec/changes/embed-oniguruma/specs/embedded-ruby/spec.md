# Spec Delta

## ADDED Requirements

### Requirement: Regular expressions as in the original runtime
Regular expressions SHALL be compiled and matched by Oniguruma with Ruby syntax, in the
encoding that `$KCODE` and the expression's own encoding flag select, so that patterns the
original runtime accepts are accepted and match as they do there. The engine's source SHALL be
vendored with a manifest like the interpreter's, and the interpreter's former engine SHALL NOT
be built.

#### Scenario: Results measured on the original
- **WHEN** each committed pattern and subject measured on the original runtime is evaluated with `$KCODE` set to `UTF8`
- **THEN** the match position, the captures, the result of `gsub` and the number of `scan` results equal the measured ones, and a pattern that is an error on the original is a `SyntaxError` with the same message

#### Scenario: The pattern that stopped a game
- **WHEN** `/\[Vv][([0-9]+)]/` is evaluated
- **THEN** no error is raised

#### Scenario: Multi-byte characters
- **WHEN** `"测试".scan(/./).length` and `"测试".split(//).length` are evaluated with `$KCODE` set to `UTF8`
- **THEN** both are 2

#### Scenario: Interpreter suites
- **WHEN** the upstream test suites of the interpreter run
- **THEN** they pass, except cases that pin behaviour of the former engine, which are adjusted and listed with the patch

#### Scenario: Vendored source is pinned
- **WHEN** the manifest test of the vendored engine runs
- **THEN** every file matches its recorded hash
