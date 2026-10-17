## ADDED Requirements

### Requirement: The workspace builds and is tested on macOS
The workspace SHALL build for `aarch64-apple-darwin`, and its tests SHALL pass there on
MoltenVK, except those kept to Linux, each of which names the reason.

#### Scenario: Vulkan backend on the Apple GPU
- **WHEN** the tests of `vkrgss-vk` run on a Mac with MoltenVK and the Apple GPU as the test device
- **THEN** they pass, the differential tests against the CPU reference included

#### Scenario: Interpreters
- **WHEN** the tests of `vkrgss-ruby-sys` and `vkrgss-ruby192-sys` run on the Mac
- **THEN** they pass, and `RUBY_PLATFORM` is `aarch64-darwin`

#### Scenario: Errors of Math on macOS
- **WHEN** Ruby 1.8.7 evaluates `Math.log(0)` and `Math.atanh(1)` on the Mac
- **THEN** each raises `Errno::ERANGE`, as on Linux

#### Scenario: Linux unchanged
- **WHEN** the gate runs on Linux
- **THEN** it passes as before
