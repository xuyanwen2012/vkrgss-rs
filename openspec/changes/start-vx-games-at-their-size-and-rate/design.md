## Context

- `RunConfig` (`crates/vkxp-binding/src/runtime.rs`) carries the runtime of the run since
  `detect-the-runtime-and-open-vx-games`.
- `vkxp_ruby_sys::Options::symbols` selects the start-up symbol table
  (`SymbolTable::Rgss1`, the default, or `SymbolTable::Rgss2`). `crates/vkxp/src/main.rs`
  starts the interpreter in two places, both with `Options::default()`.
- `Scene::new()` (`crates/vkxp-core/src/scene.rs`) is 640 by 480 (`Scene::DEFAULT_SIZE`)
  at 40 frames per second. Every frame packet carries the logical size and the frame rate,
  and the host, the compositors and the headless dump already follow the packet.
  `WindowedOptions::logical_size` is the size the window starts with.
- Measured on the original runtime of VX (2026-10-12, both DLL versions of the corpus):
  - 1996 symbols exist before the first game script; a name that is new in the first script
    gets 16089 and the next 16097; `:initialize`, `:to_s` and `:update` are 2961, 3161 and
    6481.
  - The window is 544 by 416. `Graphics.width` and `Graphics.height` give 544 and 416.
  - `Graphics.frame_rate` is 60 at the start and clamps to 10 to 120 as in XP.

## Goals / Non-Goals

**Goals:** a game of RGSS2 starts with the symbol numbers, the frame size and the frame rate
of the original; XP is unchanged.

**Non-Goals:** `Graphics.resize_screen`, `wait`, `fadeout`, `fadein`, `brightness`,
`snap_to_bitmap` and the transition rules of RGSS2 (a later change, after
`show-brightness-and-rgss2-transition-steps`); the repeat timing of `Input`; anything of
`Window`, `Bitmap`, `Sprite` or `Tilemap` that RGSS2 adds.

## Decisions

- The program maps the runtime to the symbol table (RGSS1 to `Rgss1`, RGSS2 to `Rgss2`) in
  one function and uses it at both starts of the interpreter. RGSS3 never reaches them.
- The guest creates its scene by the runtime of `RunConfig`: 640 by 480 at 40 for RGSS1,
  544 by 416 at 60 for RGSS2. The two sizes and rates are named constants beside the
  runtime handling of the binding; `vkxp-core` learns nothing about runtimes and gets, if
  it lacks one, a constructor that takes the size and the rate.
- The size the window starts with is the frame size of the run's runtime.
- `Graphics.width` and `Graphics.height` are defined only in RGSS2 and return the logical
  size of the scene. In RGSS1 they stay undefined, as in the original of XP.
- What the binding defines by runtime is decided where `Graphics` is defined, from the
  runtime of the run; later changes add the other functions of RGSS2 the same way.
- The symbol numbers a script sees also depend on the names the prelude of vkxp makes
  before the first script. The existing test of XP (`tests/run_game_symbols.rs`) shows how
  that is kept equal to the original; the test of RGSS2 is its twin.

## Risks / Trade-offs

- A prelude that makes a name the original does not make shifts every later number. The
  twin test catches it; the fix is in the prelude, not in the table.
