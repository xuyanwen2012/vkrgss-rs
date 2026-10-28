## ADDED Requirements

### Requirement: The runtime of a window is chosen in one place
A window of `vkrgss-core` SHALL hold its runtime as one value with a variant per runtime,
each variant holding exactly the state that runtime keeps. Each entry point that behaves by
runtime (a call, the refresh of caches, the assembly of the frame) SHALL choose by that value
once, and no window SHALL exist whose runtime state is missing or of another runtime.

#### Scenario: One choice per call
- **WHEN** the source of `crates/vkrgss-core/src/window.rs` and `window/` is read
- **THEN** no window field of type `Option` holds the state of RGSS2 or RGSS3, and `Window::call` matches the runtime once before handing the call on

#### Scenario: Same pictures and behaviour
- **WHEN** the tests of `vkrgss-core`, `vkrgss-binding` and `vkrgss-binding-rgss3` and `cargo xtask xarch check` run after the change
- **THEN** every test that passed before passes with no expectation changed, none removed, and every game is identical to the baseline
