## Why

The title screen and the menu of a game of RPG Maker VX Ace are windows, and the standard
scripts make them with `Window.new(x, y, width, height)`, `padding=`, `padding_bottom=`,
`tone`, `open?` and `move` (fact 1 of the design of
`run-rgss3-games-in-a-guest-of-their-own`). In a game of RGSS3 the class `Window` is that of
RGSS2 today: its constructor takes a viewport and none of these methods exists. The window
itself, with everything the original keeps between calls, is in `vkxp-core` since
`draw-the-window-of-rgss3` (`Scene::create_window_rgss3`, `WindowCall`), tested against 1185
captures of the original. What is missing is the class. Its behaviour as scripts see it was
measured on 2026-10-13 (`ace-v1-window-REPORT.md`, section "The class" and rules 7, 9, 10,
14 and 15, with the facts log `ace-v1-window-facts-samples.txt`, outside the repository).

## What Changes

- In a game of RGSS3, and only there, `Window` is the window of RGSS3 of the scene:
  - `Window.new(x = 0, y = 0, width = 0, height = 0)`: up to four Integers and no viewport;
    a fifth argument raises `ArgumentError` and a `Viewport` `TypeError`. The defaults are
    the scene's (z 100, back opacity 192, paddings of 8) and the window has a contents
    bitmap of 1 by 1.
  - New: `padding` (its writer sets both paddings), `padding_bottom`, `arrows_visible`,
    `move(x, y, width, height)`, `open?`, `close?`, and `tone`, a `Tone` object of the
    window whose changes take effect.
  - Every assignment, `move` and `update` is handed to the scene as the call it is, also
    of an equal value, and so is every change of `cursor_rect` through its `Rect` and of
    `tone` through its `Tone`. What a call recomputes is decided in `vkxp-core`.
  - `viewport` and `viewport=` as in RGSS2. `stretch` of RGSS1 is not defined.
- A disposed skin or contents bitmap raises nothing and is drawn as none. The calls at
  which the original ends with a page fault are accepted, as `docs/decisions.md` decides.
- The test of the method list of RGSS3 checks `Window`.
- XP and VX are unchanged: every existing test passes over both interpreters with its
  expected values unedited.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-scene-classes`: the class `Window` of a game of RGSS3.
- `supported-runtimes`: what `docs/compatibility.md` says of that class.

## Impact

- `crates/vkxp-binding/src`: `window.rs` (the constructor, the new methods, the definition
  by runtime), `wrap.rs` (the window's tone as an owner of a `Tone` object), `lib.rs`
  (documentation only).
- `crates/vkxp-binding-rgss3/tests/`: `window_rgss3.rs` (new) and one line of
  `arity_rgss3.rs` (the list of bound classes).
- `docs/compatibility.md`: a subsection of "RPG Maker VX Ace".
- Depends on `bind-the-values-of-rgss3`, which must be merged first: this change calls
  `rb::check_count_rgss3`, edits `arity_rgss3.rs`, adds to the same section of
  `docs/compatibility.md`, and its tests use `Rect.new` without arguments, `Rect#set(rect)`
  and `Tone#set(tone)`.
- Not touched: `crates/vkxp-core` and every other crate; `color.rs`, `rect.rs`, `rb.rs`,
  `element.rs`, `font.rs`, `bitmap.rs`, `runtime.rs` and the prelude files of the binding;
  `tests/data/rgss3_api.txt`; the existing test files of both binding packages but the one
  line named above; `ruby192.allow`; the protocol; `docs/decisions.md`, which holds the
  decisions on the window of RGSS3 already.
- Nothing of the original is committed: skins and contents of the tests are made by
  `fill_rect`.
