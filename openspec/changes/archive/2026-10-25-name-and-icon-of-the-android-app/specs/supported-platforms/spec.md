## ADDED Requirements

### Requirement: Name and icon of the Android app
The Android app SHALL be labelled `RGSS Player` and SHALL carry the project's launcher icon as
an adaptive icon with a monochrome layer and as legacy and round icons per density; its
package name SHALL stay `io.github.xuyanwen2012.vkrgss`.

#### Scenario: Name and icon
- **WHEN** `aapt2 dump badging` reads an APK that `cargo xtask apk` built
- **THEN** the application label is `RGSS Player`, the package is `io.github.xuyanwen2012.vkrgss`, and the application icon is a resource of the APK
