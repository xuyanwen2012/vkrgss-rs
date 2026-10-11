## ADDED Requirements

### Requirement: The release package does not depend on the name of libcrypt
The executable in the package that `cargo xtask dist` writes SHALL NOT need a libcrypt
shared library at run time. Other builds SHALL link it as before. When the static library
is not installed, `cargo xtask dist` SHALL fail before building and name it.

#### Scenario: Needed libraries of the package
- **WHEN** the dynamic section of the packaged executable lists `libcrypt.so.1`
- **THEN** `cargo xtask dist` fails and names that library

#### Scenario: Static library missing
- **WHEN** `cargo xtask dist` runs on a machine without `libcrypt.a`
- **THEN** it fails before building, with a message that names the file and the packages that provide it

#### Scenario: Development build
- **WHEN** the workspace is built without `VKXP_STATIC_CRYPT`
- **THEN** libcrypt is linked dynamically and no static library is needed
