# Tasks

## 1. The class of RGSS3

- [x] 1.1 Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding` and of `vkxp-binding-rgss3` under `local/`. The class `Win32API` of RGSS3 in `win32api.rs` with its state in a `Data` object, the constructor's arguments, the libraries found and the count (design decisions 1 to 4); `prelude_win32api.rb` for RGSS1 and RGSS2 with `prelude_shared.rb` keeping its line numbers; `Win32API` among the bound classes of `arity_rgss3.rs`; `crates/vkxp-binding-rgss3/tests/win32api_rgss3.rs`. Verify: the scenarios of "Win32API of RGSS3", "Libraries of Win32API in RGSS3" and "Declarations and count of Win32API in RGSS3"; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes with every name of the kept lists.

## 2. Conversions, values and the measured calls

- [x] 2.1 The conversions of the parameters, the values by the export and the buffers that keep their label (decisions 5 to 7). Verify: the scenarios of "Parameters of Win32API in RGSS3" and "Values of Win32API in RGSS3"; the test command of task 1.1.
- [x] 2.2 The bare profile name of RGSS3 (decision 8), the tests of the measured calls and of the address trick (decision 9), and the game of decision 12 in `crates/vkxp/tests/rgss3.rs`. Verify: the scenarios of "Measured calls of Win32API in RGSS3"; `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp -p vkxp-guest-rgss3 -p vkxp-binding-rgss3 --locked` passes.

## 3. DL and the shared tests

- [ ] 3.1 `prelude_dl.rb` for RGSS3 (decision 10). Verify: the scenarios of "DL of RGSS3"; the test command of task 1.1.
- [ ] 3.2 The Ruby snippets of the three tests of the keyboard state in `crates/vkxp-binding/tests/win32api.rs` with `unpack` and `pack`, their markers and their entry of `ruby192.allow` removed (decision 11). Verify: the scenarios of "Win32API of XP and VX over Ruby 1.9.2"; `git diff` of `tests/win32api.rs` against the start of the change changes no expected value; the test command of task 1.1.

## 4. Documents

- [ ] 4.1 `docs/compatibility.md` (the section "RPG Maker VX Ace"), `CHANGELOG.md` under "Unreleased", the documentation of `win32api.rs` and of `lib.rs` where it describes `Win32API`; the `PLACEHOLDER` of `ShowCursor` names the measurement of RGSS3. Verify: the scenarios of "Win32API and DL of VX Ace in the documents"; the text names nothing that the code does not do; `cargo test -p xtask --locked` passes.

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, every name of the kept lists is listed again, and `openspec validate emulate-win32api-and-dl-over-ruby-1-9-2 --strict` passes.
