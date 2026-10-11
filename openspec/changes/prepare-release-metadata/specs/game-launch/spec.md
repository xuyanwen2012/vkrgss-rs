## ADDED Requirements

### Requirement: Version option
`vkxp --version` and `vkxp -V` SHALL print `vkxp` and the version of the package on one line
to the standard output and exit with status 0, without opening a device or a game.

#### Scenario: Version
- **WHEN** `vkxp --version` is run with no Vulkan driver selectable and no game directory
- **THEN** the standard output is `vkxp 0.1.0` followed by a line break and the exit status is 0

#### Scenario: Listed in the usage
- **WHEN** `vkxp --help` is run
- **THEN** the text lists `--version`
