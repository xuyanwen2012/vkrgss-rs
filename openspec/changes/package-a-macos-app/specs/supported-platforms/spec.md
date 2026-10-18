## ADDED Requirements

### Requirement: A self-contained macOS app
`cargo xtask app` SHALL build `vkrgss.app`, which runs on a Mac without Homebrew's Vulkan
loader or MoltenVK installed and without `DYLD_*` variables, and SHALL ask for a game
directory when started without one.

#### Scenario: No Homebrew needed
- **WHEN** `otool -L` is run on every executable and library of the bundle
- **THEN** none names a path under `/opt/homebrew`

#### Scenario: Headless frame from the bundle
- **WHEN** the bundle's `vkrgss` runs a generated game headless with a frame to dump, from a shell without `DYLD_*` variables
- **THEN** the dumped frame is identical to that of the development build

#### Scenario: No game directory
- **WHEN** the app starts without a game directory and the folder chooser is cancelled
- **THEN** the app ends without an error dialog and with status 0
