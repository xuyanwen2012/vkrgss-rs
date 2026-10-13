## Why

Milestone A2 of VX Ace is walking on a map. The class `Tilemap` of RGSS3 was measured on the
original runtime on 2026-10-13 (`ace-v1-tilemap-REPORT.md` with its model under
`ref/tools/oracle/measurements/`, outside the repository): 1481 frames, 318 million pixels,
none wrong, on both DLL versions (3.0.0.1 and 3.0.1.1), among them held-out random scenes and
real maps. It is the tilemap of RGSS2 with five rules changed and one more layer of data. The
interpreter of RGSS3 (Ruby 1.9.2) and its binding are being built by other changes, so no
script of RGSS3 runs yet. This change builds everything of the tilemap that does not need the
interpreter, so that the binding later only hands the data over.

## What Changes

- The tilemap plane op names a rule set, RGSS2 or RGSS3. With RGSS3:
  - the map has four layers: tile numbers in layers 0 to 2, a shadow mask in layer 3 whose
    bits 0 to 3 darken the four quarters of a cell;
  - a table is a tile of A2 whose entry of the flags table has the bit 0x80, in any layer;
  - the edge under a table comes from layer 1 of the cell above and is drawn after layer 1
    and the shadow, before layer 2;
  - the star bit 0x10 counts in layer 2 only; a star tile of layer 0 or 1 is drawn on the
    lower plane;
  - walls give no shadow by themselves.
- The CPU reference and the Vulkan compositor evaluate the rule set and stay byte-identical.
- The scene has a tilemap of RGSS3: two planes with that rule set, and none while its
  `visible` is false.
- **BREAKING** for encoded streams: the protocol version goes up by one, so an input
  recording written by an earlier build is refused, as after every such step. Save states
  are not affected.
- Where the original reads outside its arrays (numbers 1024 to 1535, negative numbers, maps
  with fewer than four layers), vkxp reads nothing outside and draws nothing; these are
  recorded as decisions with the cases in which the original draws something else.
- XP and VX are drawn exactly as before: no expected value of an existing test changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: the rule set of the tilemap plane op; the version.
- `frame-composition`: how a plane op of the rule set of RGSS3 is evaluated.
- `vulkan-compositor`: the Vulkan backend agrees with the reference on such ops.
- `core-scene`: the tilemap of RGSS3 in the scene.
- `supported-runtimes`: the decisions on what the original leaves undefined.

## Impact

- `crates/vkxp-proto` (`frame.rs`, `codec.rs`, `lib.rs`, a module for what the numbering of
  RGSS3 adds), `crates/vkxp-core` (`element.rs`, `scene.rs`, `tilemap/rgss2.rs` and its
  tests, `lib.rs`), `crates/vkxp-compose-ref` (`tilemap_rgss2.rs`, `compose.rs`, fixtures,
  tests and a file of digests), `crates/vkxp-vk` (`records.rs`, `shaders/compose.comp`,
  `shaders/records.glsl`, `shared.rs` and `build.rs` if a constant is shared, tests).
- `README.md` and `docs/architecture.md` where they name the protocol version;
  `docs/decisions.md`, section "VX Ace".
- Not touched: `crates/vkxp-binding` and everything of an interpreter (the class `Tilemap`
  of RGSS3 comes with the binding over Ruby 1.9.2); the `Rules` of the scene; the tilemap of
  RGSS1; `docs/compatibility.md`, which says nothing of VX Ace until a game of it runs.
- Nothing of the original is committed: the sheets of the tests are made by formula, and from
  the measurement only rules and numbers are taken.
