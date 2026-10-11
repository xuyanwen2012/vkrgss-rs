## ADDED Requirements

### Requirement: Help option
`vkxp --help` and `vkxp -h` SHALL print the usage text to the standard output and exit with
status 0, without opening a device or a game. An unknown option SHALL keep ending with a
status other than 0 and the usage on the error stream.

#### Scenario: Help
- **WHEN** `vkxp --help` is run with no Vulkan driver selectable and no game directory
- **THEN** the standard output holds the usage text, which lists `--help` and `--version`, the error stream is empty and the exit status is 0

#### Scenario: Unknown option
- **WHEN** `vkxp --no-such-option` is run
- **THEN** the exit status is 2 and the error stream names the option
