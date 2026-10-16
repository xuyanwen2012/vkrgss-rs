## RENAMED Requirements

- FROM: `### Requirement: Release package holds both executables`
- TO: `### Requirement: Release package holds the executables`

## MODIFIED Requirements

### Requirement: Release package holds the executables
`cargo xtask dist` SHALL build `vkrgss`, `vkrgss-guest-rgss12` and `vkrgss-guest-rgss3` for
release in one build with libcrypt linked statically into each, SHALL fail when any needs a
shared libcrypt, and SHALL put the executables side by side into the archive, with the licence texts of the
bundled Ruby 1.9.2.

#### Scenario: Package
- **WHEN** `cargo xtask dist` has run
- **THEN** the archive unpacks to one directory that holds the executables `vkrgss`, `vkrgss-guest-rgss12` and `vkrgss-guest-rgss3`, the files it held before, and `licenses/ruby-1.9.2/COPYING`, `licenses/ruby-1.9.2/GPL` and `licenses/ruby-1.9.2/LEGAL`

#### Scenario: Missing guest executable
- **WHEN** the package is put together from a tree without `target/release/vkrgss-guest-rgss3`, or without `target/release/vkrgss-guest-rgss12`
- **THEN** the command fails and names that file

#### Scenario: Shared libcrypt in the guest executable
- **WHEN** the check of needed libraries is given `libcrypt.so.1` for the guest executable
- **THEN** the command fails and names the library and the executable

#### Scenario: The two files are enough
- **WHEN** the executables `vkrgss` and `vkrgss-guest-rgss3` of a build are copied into an empty directory and the `vkrgss` there runs a generated game of RGSS3 headless
- **THEN** the exit status is 0

#### Scenario: The three files are enough
- **WHEN** the three executables of a build are copied into an empty directory and the `vkrgss` there runs a generated game of RGSS1, one of RGSS2 and one of RGSS3 headless with `--guest process`
- **THEN** each exit status is 0

## ADDED Requirements

### Requirement: Guest executable of RGSS1 and RGSS2
The package `vkrgss-guest-rgss12` SHALL be one binary of that name that depends on
`vkrgss-guest-main` and on no other package of the workspace directly. Its `main` SHALL call
the function of the glue that makes an executable a guest process, with the package
version, and SHALL do nothing else as a guest. The package SHALL take the version of the
workspace.

#### Scenario: Direct dependencies
- **WHEN** the dependencies that the manifest of `vkrgss-guest-rgss12` declares, its development dependencies left out, are listed
- **THEN** `vkrgss-guest-main` is the only package of the workspace among them

#### Scenario: Nothing of the host is linked
- **WHEN** the packages that `vkrgss-guest-rgss12` reaches are listed
- **THEN** none of `vkrgss-host`, `vkrgss-vk`, `vkrgss-audio`, `ash`, `winit` and `cpal` is among them

#### Scenario: Started by anything else
- **WHEN** `vkrgss-guest-rgss12` is started without the hidden option of the guest role
- **THEN** it says what it is and that `vkrgss <game directory>` starts it, and exits with status 2
