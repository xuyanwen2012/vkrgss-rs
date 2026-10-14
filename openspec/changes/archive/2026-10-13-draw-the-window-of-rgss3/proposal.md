## Why

Milestone A1 of VX Ace is the title screen and the menu, and both are made of windows. The
class `Window` of RGSS3 was measured on the original runtime on 2026-10-13
(`ace-v1-window-REPORT.md` with its model and cases under `ref/tools/oracle/measurements/`,
outside the repository): the model reproduces 1185 captures, 268 million pixels, with no
wrong pixel, three held-out random sets among them, on both DLL versions (3.0.0.1 and
3.0.1.1). It is the window of RGSS2 with the differences that report lists. The
interpreter of RGSS3 (Ruby 1.9.2) and its binding are being built by other changes, so no
script of RGSS3 runs yet. This change builds everything of the window that does not need the
interpreter, so that the binding later only hands the calls over.

## What Changes

- The scene has a window of RGSS3, a third rule set beside those of RGSS1 and RGSS2, chosen
  when the window is created:
  - defaults z 100, back opacity 192, padding 8, bottom padding 8, arrows allowed, a
    neutral tone;
  - the background in the normal blend at every openness, with the window's own tone;
  - the cursor with a border of 4, at `pad + |cursor_rect - origin|`, placed and cut at
    certain calls and kept until the next;
  - contents at the padding, clipped by the paddings and by the window, drawn last;
  - scroll arrows whose conditions use the paddings, decided at their own list of calls,
    drawn below pause sign and contents; `arrows_visible`;
  - `move`; a window without area that still shows one pixel; disposed skin and contents
    drawn as none.
- The calls a script makes on such a window, the new ones included, go through
  `Window::call`, so that what the original keeps between calls is state of `vkxp-core`
  that is tested now. The binding will only translate.
- Skins smaller than the layout: the rule the measurement of RGSS2 left open was found. It
  is implemented for RGSS3 and, as far as the captures of RGSS2 decide it (the background),
  for RGSS2. **Two expected pictures of RGSS2 change**, to the digests of the pictures the
  original runtime of VX drew; they were named "not measured" until now.
- Where the original ends with a page fault, vkxp goes on; recorded as known differences.
- No protocol change and no change to either compositor: every rule is said with the ops
  that exist. The scenes of the measurement are composed by the CPU reference and by the
  Vulkan backend, which must agree byte for byte and with the digests of the model.
- XP is drawn exactly as before, and VX too but for the two pictures above.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `core-scene`: the window of RGSS3; the background of a window of RGSS2 from a skin
  smaller than the layout.
- `supported-runtimes`: the recorded decisions and known differences.

## Impact

- `crates/vkxp-core`: `window.rs`, `window/rgss2.rs` and a module for what RGSS3 differs
  in, `element.rs` (the window), `scene.rs` (creation), `lib.rs` (exports and
  documentation), `test_support/window_cases.rs`, tests, and under `tests/data/` two new
  files of scenes of RGSS3 and two lines and the head of `window-rgss2-cases.txt`.
- `crates/vkxp-vk`: `src/tests/windows.rs` only.
- `docs/decisions.md` (section "VX Ace"), `docs/compatibility.md` (section "The window of
  VX" only).
- Not touched: `crates/vkxp-proto` and the protocol version; `crates/vkxp-compose-ref/src`;
  the sources and shaders of `crates/vkxp-vk`; `crates/vkxp-binding` and everything of an
  interpreter (the class `Window` of RGSS3 comes with the binding over Ruby 1.9.2); the
  `Rules` of the scene; the window of RGSS1; text.
- Nothing of the original is committed: skins and contents of the tests are made by
  formula, and from the measurement only rules, numbers and digests of frames of generated
  bitmaps are taken.
