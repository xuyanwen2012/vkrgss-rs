## MODIFIED Requirements

### Requirement: The window of RGSS1 and RGSS2 is unchanged by RGSS3
The class `Window` of a game of RGSS1 or RGSS2 SHALL NOT change with that of RGSS3: every
test of the two binding packages that passed before SHALL pass under its name with its
expected values unedited, over both interpreters. The tests of RGSS3 SHALL be a file of the
package over Ruby 1.9.2 alone, and the scenarios named "XP and VX" SHALL be held by tests of
the shared files.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkrgss-binding` and of `vkrgss-binding-rgss3` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: Existing tests
- **WHEN** the difference of `crates/vkrgss-binding/tests` and `crates/vkrgss-binding-rgss3/tests` against the start of the change is read
- **THEN** it holds the new file of RGSS3, one new test in each of `tests/scene.rs` and `tests/scene_rgss2.rs` of `vkrgss-binding`, and `Window` added to the bound classes of `arity_rgss3.rs`; no test that existed, no expected value and no line of the allow-list changed

#### Scenario: Other crates
- **WHEN** the difference of `crates` against the start of the change is taken
- **THEN** it names files of `crates/vkrgss-binding` and `crates/vkrgss-binding-rgss3` only

### Requirement: The scene classes of RGSS1 and RGSS2 are unchanged by RGSS3
The scene classes of a game of RGSS1 or RGSS2 SHALL NOT change with those of RGSS3: every
test of the two binding packages that passed before SHALL pass under its name with its
expected values unedited, over both interpreters.

#### Scenario: Names of the tests
- **WHEN** `cargo test --locked -- --list` of `vkrgss-binding`, `vkrgss-binding-rgss3` and `vkrgss-core` is compared before and after
- **THEN** every name listed before is listed after

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `Plane.new.respond_to?(:viewport=)`, `Viewport.new` without arguments, and `v = Viewport.new(0, 0, 8, 8); s = Sprite.new(v); v.dispose; s.disposed?`
- **THEN** the results are `false`, an `ArgumentError`, and `true`, over both interpreters
