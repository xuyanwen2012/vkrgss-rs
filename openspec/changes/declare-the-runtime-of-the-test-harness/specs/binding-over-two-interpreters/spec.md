## ADDED Requirements

### Requirement: The harness is reached only through its declared runtime
The test harness of the binding SHALL give access to its interpreter only through the handle
that starting it with a runtime returns, and SHALL NOT choose a runtime of its own. Its
generated files SHALL lie in a directory of their own for each test process.

#### Scenario: No default runtime
- **WHEN** the source of the harness is read
- **THEN** it has no fallback runtime, and evaluating Ruby, running on the interpreter or asking for the game directory needs the handle that `start_as` returns

#### Scenario: Order of tests does not matter
- **WHEN** the tests of `vkrgss-binding` and `vkrgss-binding-rgss3` run with `--test-threads=1` and with the default threads
- **THEN** both runs pass

#### Scenario: Two runtimes in one process
- **WHEN** a test process starts the harness with one runtime and then with another
- **THEN** the second start panics with a message that names both runtimes

#### Scenario: Two processes of one test executable
- **WHEN** two processes of the same test executable run at the same time
- **THEN** each uses its own directory of generated files

#### Scenario: Same tests
- **WHEN** the tests of both packages are listed with `cargo test --locked -- --list` before and after the change
- **THEN** the lists are equal
