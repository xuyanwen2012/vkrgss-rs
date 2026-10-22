## ADDED Requirements

### Requirement: Interrupt of a script
In script mode SIGINT SHALL raise `Interrupt` in the script, as `ruby` does. A script SHALL be
able to rescue it. When it is not rescued, `ensure` clauses and `at_exit` blocks SHALL run, its
report SHALL be written to the error stream, and the process SHALL end by SIGINT. Games SHALL
handle SIGINT as before.

#### Scenario: Rescued
- **WHEN** a script that rescues `Interrupt` and prints `caught` gets SIGINT
- **THEN** it prints `caught` and goes on

#### Scenario: Not rescued
- **WHEN** a script with an `ensure` clause that prints `ensure` and an `at_exit` block that prints `exit` gets SIGINT
- **THEN** both lines are printed and the process ends by SIGINT
