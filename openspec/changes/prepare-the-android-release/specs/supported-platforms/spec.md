## ADDED Requirements

### Requirement: A release APK
`cargo xtask apk --release` SHALL build an APK signed with the keystore that the environment
names, with the workspace's version, and SHALL refuse without it; the release workflow SHALL
attach that APK to the release of a version tag when the signing secrets exist.

#### Scenario: Signed with the given key
- **WHEN** `cargo xtask apk --release` runs with a throwaway keystore in the environment
- **THEN** `apksigner verify --print-certs` names that key and the manifest's `versionName` is the workspace version

#### Scenario: No keystore
- **WHEN** it runs without the variables
- **THEN** it fails and names them

### Requirement: Player-side pieces on Android
The app SHALL import the built-in scripts through the picker, pause sound while in the
background, and run without validation errors.

#### Scenario: Built-in scripts imported
- **WHEN** an `rpg_builtin.rb` is imported through the library screen and a game of RGSS1 starts
- **THEN** the game runs with it, without a file in the external files directory

#### Scenario: Sound in the background
- **WHEN** the app goes to the background while music plays
- **THEN** no sound is output until the app returns

#### Scenario: Validation on the phones
- **WHEN** `cargo xtask android-run --validation` runs the comparison games on both phones
- **THEN** no validation error is logged
