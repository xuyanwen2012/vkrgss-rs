## ADDED Requirements

### Requirement: The macOS app needs no newer macOS than it declares
Every executable and library in `vkrgss.app` SHALL be built for the minimum macOS that its
`Info.plist` declares, macOS 14, and `cargo xtask app` SHALL fail when one is not.

#### Scenario: Minimum of every file
- **WHEN** `cargo xtask app` has built the bundle
- **THEN** the minimum macOS of every Mach-O file in it is at most 14.0

#### Scenario: A file built for a newer macOS
- **WHEN** the check is given a library whose minimum is 27.0
- **THEN** it fails and names the file and its minimum
