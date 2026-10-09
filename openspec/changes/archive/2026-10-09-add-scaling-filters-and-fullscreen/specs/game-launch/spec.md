# Spec Delta

## ADDED Requirements

### Requirement: Window options
The program SHALL accept `--filter auto|nearest|linear|sharp` and `--fullscreen`, and the same
settings in the configuration files, and SHALL refuse another filter name with the usage
message.

#### Scenario: Unknown filter
- **WHEN** the program is run with `--filter cubic`
- **THEN** it prints the usage message and exits with a non-zero status

#### Scenario: From the file
- **WHEN** a configuration file sets the filter to `linear` and no option is given
- **THEN** the run uses linear, and `--filter nearest` overrides it
