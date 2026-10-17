## ADDED Requirements

### Requirement: The interpreters build and are tested on Android
Both interpreter crates SHALL build for `aarch64-linux-android` at API level 34 or later,
and their tests SHALL pass on a phone over adb, except those that need the host, each of
which names the reason. A build below API level 34 SHALL fail with a message that names
the minimum.

#### Scenario: Upstream suites on a phone
- **WHEN** the upstream suites of both interpreters run on the Pixel 7a and on the Galaxy S24+ through the adb runner
- **THEN** they pass, outside the cases listed for Android with the reason

#### Scenario: IO at the end of a file
- **WHEN** Ruby 1.8.7 on a phone reads an empty file and a closed pipe with `eof?` and `readpartial`
- **THEN** `eof?` is true and `readpartial` raises `EOFError` without blocking

#### Scenario: crypt
- **WHEN** `"x".crypt("ab")` is evaluated on Android
- **THEN** it raises `NotImplementedError`

#### Scenario: Too old an API level
- **WHEN** an interpreter crate is built for Android with API level 31
- **THEN** the build fails and names API level 34
