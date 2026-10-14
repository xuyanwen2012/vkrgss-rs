# Design

## Context

`Input` is defined once for every runtime today (`crates/vkxp-binding/src/input.rs`): twenty
constants with the numbers of RGSS1 (the same in RGSS2), and `press?`, `trigger?` and
`repeat?` that convert their argument with `rb::long` and answer for the button of that
number. `Input.update` takes the snapshots that have arrived without waiting
(`Guest::pump(Await::Nothing)`), latches the newest (`InputState::update`) and tells the
emulated `Win32API` (`input_updated`). The guest takes snapshots while it waits for the tick
of a frame (`graphics::frame`).

`vkxp-core` has the input of RGSS3 since the archived `draw-sprites-and-snapshots-as-rgss3`:
`InputState::with_rules(Rules::Rgss3)` has the repeat timing of RGSS2, and
`InputState::frame_shown` marks a shown frame, after which the snapshots received so far
are visible to the next update (requirements "Input of RGSS3" and "Snapshots are seen at
shown frames in RGSS3" of `core-input`). Its decision 5 says the binding of RGSS3 calls it
once per frame it shows and from nowhere else. `bind-the-scene-classes-of-rgss3` makes the
scene of a game of RGSS3 with `Rules::Rgss3` and keeps its input state at `Rules::Rgss2`
through `runtime::input_rules`, until this change.

This change is row 5 of the table "the changes that follow" of
`run-rgss3-games-in-a-guest-of-their-own`: Symbols and the Integers of RGSS2, `false` for
anything else, the constants as Symbols, a key seen at shown frames.

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding/src/{input,graphics,runtime,guest}.rs`, `crates/vkxp-core/src/input.rs`,
  `testing/host.rs` (`play`, `Script::at_start`, `Script::after_frame`), the tests of `Input`
  in `crates/vkxp-binding/tests/modules.rs` (the model), `crates/vkxp/src/config.rs`
  (`keys_of`: a game of RGSS3 has the keys of VX).
- The design of the archived `draw-sprites-and-snapshots-as-rgss3` (decision 5, the
  Follow-ups) and `docs/decisions.md`, "VX Ace" (the rule of shown frames and its known
  differences).
- The measurements, outside the repository: `../../ref/tools/oracle/measurements/
  ace-v1-sprite-REPORT.md` section 6 ("the sprite report"), `ace-v1-sprite-input-keys.txt`,
  `ace-v1-sprite-pump-log.txt`, `ace-v0-REPORT.md` section 4 ("the V0 report") and
  `../../ref/RGSS301_api.md` ("the method list", section `Input`).

What was measured (sprite report 6, V0 report 4), against the binding today:

| | Today | RGSS3, measured |
|---|---|---|
| constants | 2, 4, 6, 8, 11 to 18, 21 to 23, 25 to 29 | `:DOWN` ... `:F9`, the Symbols of their names |
| `press?(:DOWN)` | `TypeError` | the button |
| `press?(2)` | the button | the button |
| `press?(2.9)`, `press?(nil)`, `press?("DOWN")` | the button 2, `TypeError`, `TypeError` | `false`, never an error |
| `press?(258)`, `press?(:down)` | `false`, `TypeError` | `false` |
| `Input.update` alone | sees a new snapshot | sees no new key |
| repeat, `dir4`, `dir8` | RGSS2 | RGSS2, 12084 updates without a mismatch |
| keys | the map of VX (host) | the table of RGSS2 line for line |
| method list | `dir4 dir8 press? repeat? trigger? update` | the same arities |

## Goals / Non-Goals

**Goals:**

- `Input` of a game of RGSS3 answers every argument the standard scripts and the measured
  probes give, with the measured values, and sees keys at the frames the original sees
  them in a frame loop.
- The method list and the constants of RGSS3 match exactly.
- RGSS1 and RGSS2 do not change.

**Non-Goals:**

- The count of key messages a frame of the original takes, a key pressed and released
  between two frames, the message taken during `Kernel#sleep`: vkxp has no message queue,
  and the decision on it is recorded (`docs/decisions.md`, "VX Ace"). Known differences.
- `GetKeyState`, `GetAsyncKeyState` and `GetKeyboardState` of the emulated `Win32API` in a
  game of RGSS3 (V0 report 4: `GetKeyState` changes when `Input.press?` does): with
  `emulate-win32api-and-dl-over-ruby-1-9-2`.
- Gamepads, the F1 dialog, F12 and Alt+Enter of the original: not measured (sprite report,
  "Not measured / open", point 5). The F5 resize of one game of the corpus (V0 report,
  "Open problems", point 3) is the game's own and not examined.
- What RGSS2 does with a Symbol, `nil` or a Float (sprite report 6 names it: `nil` raised
  `TypeError`, 13.9 was truncated, a Symbol gave `false`): Follow-ups.
- Any change of `vkxp-core`, of the protocol, of the host or of its key maps.

## Decisions

1. **Selected by the runtime of the game.** `input::define` takes the `Runtime`, as the
   other modules do, and `Guest::install` hands it over. For `Runtime::Rgss3` it registers
   the functions of RGSS3 for `press?`, `trigger?` and `repeat?` (arity 1) and the
   constants of RGSS3; `update`, `dir4` and `dir8` are the functions of today in every
   runtime. RGSS1 and RGSS2 keep their functions and constants.

2. **Constants.** In RGSS3 the Ruby source that defines the constants says `DOWN = :DOWN`
   for each of the twenty names of `Button::ALL`, in the order of today. The twenty names
   are interned at start-up, as on the original, where they are constants of the module.

3. **The argument of a query** (sprite report 6). `button_arg_rgss3(value) ->
   Option<Button>`, without calling any method of the value and without raising:
   - a Symbol whose name is one of the twenty names, exactly as written (`:DOWN`; not
     `:down` or `:Down`): that button;
   - a Fixnum equal to the number of RGSS2 of a button: that button;
   - anything else (another Symbol, a String, a Float also when integral, a Bignum, `nil`,
     `true`, `false`, an Array, any other object or module): `None`, which every query
     answers with `false`.
   Ruby counts the arguments (arity 1): `Input.press?` and `Input.press?(:A, :B)` raise the
   interpreter's `ArgumentError`. The Symbol is compared by its ID with the IDs of the
   twenty names, taken once when the module is defined; a Symbol that names no button is
   never interned by the comparison. Not measured and decided here: an object that defines
   `to_int` or `to_sym` gives `false` (the measured "an Object" did not define them).

4. **The input state of RGSS3.** `runtime::input_rules`, which `bind-the-scene-classes-of-
   rgss3` added, goes; `guest.rs` makes the input state with `runtime::rules` again, which
   gives `Rules::Rgss3` for RGSS3. `connect` and `reset` take the rules of the state they
   replace, as today.

5. **Marking shown frames.** `graphics::frame`, through which every frame the guest shows
   passes (`Graphics.update`, each frame of `wait`, `fadeout`, `fadein` and of a
   `transition`), calls `guest.input.frame_shown()` after the tick of the frame has been
   awaited, for every runtime: with the rules of RGSS1 and RGSS2 the call changes nothing
   (`core-input`). Nothing else marks a frame: not `Input.update`, not a call of
   `Graphics` that shows no frame (`frame_reset`, `brightness=`, `snap_to_bitmap`,
   `freeze`, a transition of no frame, `wait(0)`, `fadeout(0)`, `resize_screen`), not a
   modal message. That is the measured rule of the sprite report 6 ("When a key is seen")
   as `docs/decisions.md` records it for vkxp: a snapshot that arrived before a shown
   frame is seen by the next `Input.update`. Because a scheduled or replayed run delivers
   the snapshot of frame n before the tick of frame n, it gives the same answers at the
   same updates as under the rules of RGSS2.

6. **`Input.update`** is unchanged: it still takes what has arrived without waiting, and
   the state of RGSS3 keeps it invisible until a frame is shown, which is the measured
   "`Input.update` takes no message". `win32api.input_updated` is called as today.

7. **Keys.** A game of RGSS3 gets the key map of VX from the program (`config.rs`,
   `keys_of`), which is the measured table of RGSS3 line for line, F5 to F9 included. No
   change; `docs/compatibility.md` says so.

8. **Method surface.** `Input` is added to `BOUND` of `arity_rgss3.rs`, nothing else of
   that file being edited. `input_rgss3.rs` compares `Input.singleton_methods(false)` and
   `Input.constants` with the method list itself, because that test compares instance
   methods of classes.

9. **Tests.** `crates/vkxp-binding-rgss3/tests/input_rgss3.rs`, a file of the second package
   alone that starts the harness with `testing::start_as(Runtime::Rgss3)`, plays snippets
   against a scripted host (`testing::host::play`) and sends snapshots with
   `Script::at_start` and `Script::after_frame` (which delivers before the tick of that
   frame, as a scheduled run does). The scenario "XP and VX" is one new test in
   `crates/vkxp-binding/tests/graphics_rgss2.rs`, a binary of RGSS2 that runs over both
   interpreters; the existing tests of `Input` in `tests/modules.rs` hold RGSS1 and are
   not edited. No entry of `ruby192.allow` is expected to start passing (the entry `test
   modules button_constants` is a test of RGSS1 over Ruby 1.9.2 and stays); if the
   mechanism reports one, the entry and its marker are removed.

10. **Documents.** `docs/compatibility.md`, under "RPG Maker VX Ace", a subsection "Input
    of VX Ace": the constants and arguments, the timing, the keys, and the known
    differences of decision 5 of the archived change (the count of key messages, a key
    pressed and released between two frames, `Kernel#sleep`). `docs/decisions.md` is not
    touched.

11. **Delta specifications.** "Method surface of the modules" and "Input" of
    `rgss-modules` are MODIFIED: they state the numbers and the latching of RGSS1 and
    RGSS2, and point to the requirements of RGSS3, which are ADDED.

## Implementation notes

- A flow reads `../../ref/` and copies nothing from it but names, arities, rules and
  numbers. No Wine and no original runtime is run.
- Before anything changes, keep `cargo test --locked -- --list` of `vkxp-binding` and of
  `vkxp-binding-rgss3` under `local/`: every name must be there at the end.
- `ffi::symbol_p` and `ffi::sym2id` exist in both interpreter crates; the IDs of the twenty
  names can be kept in the module's state beside the classes (`guest::classes()`), or
  compared with `rb_intern` results taken in `define`.
- A snippet of `input_rgss3.rs` starts in a new connection, so its input state is new:
  nothing held, the `dir4` flag 0. The tests of a binary share one interpreter and run one
  snippet at a time.
- In the scenarios "shown frame" means a `Graphics.update` (or a frame of `wait` or of a
  `transition`); "update" means `Input.update`.

## Follow-ups

- **VX: the arguments of `Input`**: on the original of RGSS2 `nil` raised `TypeError`, 13.9
  was truncated and true, a Symbol gave `false` (sprite report 6). vkxp converts with
  `rb::long`, so a Symbol raises `TypeError` in a game of VX. Not measured one by one for
  RGSS2 here (`vx-v1-api-*` has the RGSS2 side).
- The count of key messages a frame takes; a key pressed and released between two frames;
  the message taken during `Kernel#sleep` (sprite report 6, "When a key is seen"): known
  differences, recorded.
- `GetKeyState` and the other key functions of the emulated `Win32API` in RGSS3 (V0 report
  4): `emulate-win32api-and-dl-over-ruby-1-9-2`.
- Gamepads, F1, F12, Alt+Enter on the original (sprite report, "Not measured / open",
  point 5); the F5 resize of Capella's Promise (V0 report, "Open problems", point 3).
- Whether a frame shown while a modal message is open takes key messages on the original:
  not measured; vkxp marks no frame then.
- The irregular sequences of the V0 report, point 4, are explained by the sprite report
  (one message per frame without slack); they are the known difference above.
