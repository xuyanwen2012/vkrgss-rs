## Why

`docs/decisions.md` says version 0.3.0 is released when A3 is reached. On 2026-10-14 the
comparisons with the original runtime of VX Ace under Wine reached A1, A2 and A3 (numbers in
`design.md`): the frames of the title, the menu, maps, walking and a battle of one game equal
the original's except three antialiased pixels of one glyph, and a second game's route that
saves through its own menu, returns to the title and loads equals the original's in every
compared frame and writes both saves byte for byte as the original does; repeated runs of
the original give the same pictures and saves. The owner accepted on 2026-10-13 that
milestone frames of VX Ace may differ inside text as those of XP and VX do.

## What Changes

- `docs/decisions.md`: the rows A1, A2 and A3 "Done" with their records; the experimental
  state of VX Ace ends.
- VX Ace is supported: the README (first paragraph, table of runtimes, banner and its
  alternative text), `AGENTS.md`, `docs/compatibility.md`, `docs/running.md`, the usage
  message. A run of RGSS3 no longer says that its support is experimental.
- The version of the workspace is 0.3.0; `CHANGELOG.md` gets the section 0.3.0, dated
  2026-10-14, from what "Unreleased" holds.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `supported-runtimes`: VX Ace supported, the milestones done, version 0.3.0.
- `game-launch`: no run says that its runtime is experimental.

## Impact

`docs/decisions.md`, `README.md`, `docs/images/banner.jpg`, `AGENTS.md`,
`docs/compatibility.md`, `docs/running.md`, `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`,
`crates/vkxp/src/main.rs` and `args.rs` (the warning and the usage text), the tests of that
warning and of the usage text.
