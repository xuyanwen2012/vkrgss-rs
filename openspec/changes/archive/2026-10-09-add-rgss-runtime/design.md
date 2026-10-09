# Design

## Context

See `proposal.md` for motivation. What this change starts from:

- `vkxp-binding` has the `Guest` state on the interpreter thread, the object wrappers, argument
  and error helpers, registration of methods with the original arities, the classes `Color`,
  `Tone`, `Rect`, `Table`, `Font`, `Bitmap`, and a test harness that evaluates Ruby in the
  embedded interpreter.
- `vkxp-core` has the `Scene` (`extract(&mut Bitmaps)` returns a `FramePacket` whose bitmap
  deltas cover exactly the bitmaps its ops refer to; `frame_count`, `set_frame_count`,
  `frame_rate`, `set_frame_rate`) and `InputState` (`receive`, `update`, `is_pressed`,
  `is_triggered`, `is_repeated`, `dir4`, `dir8`). `Bitmaps::deltas(&[BitmapId])` gives the
  deltas for bitmaps named by the caller.
- `vkxp_proto::link::link(wake)` returns a `GuestEnd` (`send`, blocking `recv`, `try_recv`) and
  a `HostEnd` (`try_recv`, `send`); `wake` is called whenever the guest has sent something. The
  host answers every `Frame` with one `Tick` and may send `Input`, `Reset`, `Quit` and
  `ModalDismissed` at any time. After `Freeze` it keeps showing the held frame; after
  `Transition { duration, .. }` it cross-fades over the `duration` frames that follow.
- `vkxp-vfs` has `GameIni::read`, `resolve_rtps` and `GameFiles` (`read(FileKind, path)` for
  images, audio and data, with a not-found error carrying the path as given).
- `vkxp-binding` is a guest crate: it must not depend on `vkxp-host`, so its tests drive the
  host end of a link themselves.
- The original runtime evaluates, in this order: its two built-in script blocks (the `RPG`
  data classes; `RPG::Cache`, `RPG::Sprite`, `RPG::Weather`), then
  `$RGSS_SCRIPTS = load_data(scripts_fname)`, inflating each entry's third element into a fourth
  and evaluating it as `Section%03d` with the entry's zero-based index. Its error report looks
  up the script name through that index.
- The built-in script blocks are Enterbrain's source and are not in the repository; a tool of
  the executable change extracts them from the user's DLL into one Ruby file.
- The test game's `Main` script rescues `Errno::ENOENT` and parses its message, calls
  `Graphics.transition` with and without a picture, and sets `Graphics.frame_rate`.

## Goals / Non-Goals

**Goals:**

- `run_game(ruby, config, link)`: one call, made on the interpreter thread, that plays a game
  from start to shutdown and returns success or a described failure.
- The three modules complete at the original arities.
- Everything testable against a scripted host end, with no window.

**Non-Goals:**

- Scene classes; they are added in parallel by another change and only need `Guest` to hold
  the scene.
- A panel drawn for `print` and `p`. The host is told and answers; drawing comes later.
- Real `Win32API` emulation and audio decoding.

## Decisions

### The guest owns scene, input state and link

`Guest` already holds the scene and the input state. It gains the `GuestEnd`, the currently
playing music and sound descriptions (for the duplicate check and for reset), and a flag for a
pending reset or quit.
All host messages are consumed in one place, `Guest::pump(wait_for_tick)`, called by
`Graphics.update`, by the frames inside `Graphics.transition`, by `Input.update` (without
waiting) and while waiting for a modal dismissal. It stores the latest snapshot, records reset
and quit, and returns when the awaited message has arrived.

Raising `Reset` or `SystemExit` happens after `pump` has returned and the guest borrow is
released, following the interpreter crate's raise rule.

### Graphics

`update`: `Scene::extract(&mut bitmaps)`, send `Frame`, `pump(true)`. `freeze` sends `Freeze`.
`transition(duration, filename, vague)`: when a filename is given, load it as a bitmap like
`Bitmap.new(path)`. No op refers to that bitmap, so its deltas are asked from the arena
(`Bitmaps::deltas`) before the next extraction and put in front of that packet's bitmap list,
which keeps the order a compositor's bitmap store accepts. Send
`Transition { duration, mask, vague }`; then `duration` times extract, send and pump; finally
dispose the mask bitmap, whose destroy delta travels with a later frame. With a mask and a
duration of 0 the mask is not announced and the message carries no mask. `frame_reset` sends `FrameReset`. Frame rate and frame count are the
scene's.

`Graphics.visible`, present in the original's method list but undocumented, is implemented as
a stored flag that defaults to `true` and has no effect yet.

### Input and Audio

`Input.update` pumps without waiting, then `InputState::update`. The constants are defined from
`vkxp_proto::Button`.

`Audio.*_play` resolves and reads the file (`GameFiles::read(Audio, name)`), clamps volume and
pitch and sends `AudioCommand::Play`. For music and background sound, a call that repeats the
description of what that channel already plays sends nothing, as the original keeps playing in
that case. `stop` and `fade` clear the remembered description.

### Running a game

`run_game(ruby, config, link)`. The binding's `Config` (today the game's files, the default
font names and the text backend) gains what running needs: the `GameIni` the caller read, the
path of the built-in scripts file, the debug and battle-test flags and the process-function
switch. Reading `Game.ini` and resolving RTP names stays with the caller, which has to report
those errors before a window exists.

1. `Guest::install`, then `SetTitle` with the title of the configuration.
2. Prelude, written in Ruby inside this crate: `$KCODE = 'U'`, `$:.clear`, `$DEBUG`, `$BTEST`,
   `Reset` and `Hangup` (subclasses of `Exception`), the disabled process functions, the
   restricted `require` and `load`, `Win32API`, `save_data`, `p` and `print`.
3. Evaluate the built-in scripts file under the name `(builtin)`; a missing file is the error of
   `specs/rgss-runtime`.
4. Read and unmarshal the archive, inflate each entry, set `$RGSS_SCRIPTS`.
5. Evaluate `Section000`, `Section001`, ... with `eval_protected`.
   - `Reset` escaping a script: stop audio, dispose all scene elements and bitmaps, clear input
     state, go to step 5 again. The interpreter and the constants scripts defined stay, as in
     the original.
   - `SystemExit`: normal end.
   - Anything else: build the report (script name through the section index in the backtrace
     entry, line number, class, message), write it to the error stream, send it as `Modal`, wait
     for the dismissal, end with failure.
6. Send audio stops and `Shutdown`.

`load_data` is a host function (case-insensitive lookup through the game's files, then
`Marshal.load`); `save_data` is Ruby (`File.open(name, "wb") { |f| Marshal.dump(obj, f) }`),
because writing goes to the game directory through Ruby's own file API like every save the
scripts make. The executable makes the game directory the working directory.

### Tests

The harness of the binding gains a scripted host, built on `vkxp_proto::link` alone: a thread
holding the `HostEnd` that answers
every `Frame` with `Tick`, answers `Modal` with `ModalDismissed`, records everything it
receives, and can be told to send snapshots, `Reset` or `Quit` after a given frame. Tests build
a temporary game directory with a `Game.ini` and a script archive assembled in the test
(`Marshal` and `Zlib` from the interpreter itself), run `run_game`, and assert on the recorded
messages and on values the scripts left in a result file. A stand-in built-in scripts file
defines a minimal `RPG` module.

## Risks / Trade-offs

- [A script blocks without calling `Graphics.update`] → The host stays responsive by design;
  nothing here depends on it. The original's hang watchdog stays off, as decided.
- [Sending whole music files through the link] → Simple and process-split friendly; a file is
  read once per change of music. Streaming is a later refinement.
- [Reset leaves script-defined global state behind] → Same as the original, which re-evaluates
  scripts in the same interpreter.
- [The scene classes land in parallel and touch `Guest`] → This change adds the scene field and
  the disposal-on-reset entry point; the other adds classes. The overlap is the registration
  list, merged by hand.

## Follow-ups

- `vkxp-core` has no way to enumerate or clear the elements of a `Scene` or the bitmaps of
  `Bitmaps`. Reset therefore disposes through the Ruby objects: `Graphics._reset` walks
  `ObjectSpace` for the instances of `Sprite`, `Viewport`, `Window`, `Plane`, `Tilemap` and
  `Bitmap` and calls each class's own `dispose`. An element or bitmap that no Ruby object
  owns is not found that way; none exists today. Should core get `Scene::clear` and a
  disposal of all bitmaps, and reset use them?
- `GameFiles` caches directory listings and has no way to learn that a script wrote a file
  through Ruby's `File`. `load_data` therefore drops the cache before every lookup, which
  costs a listing per call; `Bitmap.new` and `Audio` do not, so a picture or sound a script
  creates while the game runs is not seen. Should the cache be invalidated from the write
  side, or by directory modification time?
- The pump waits for the tick of every frame also after a quit request, on the protocol's
  word that the host answers every `Frame` with one `Tick`. A host that stops ticking once it
  has sent `Quit` would have to drop its end of the link, which the guest takes as a quit.
- The process-function switch covers the calls the specification lists, and `Process.fork`.
  `open("|command")` and `IO.read("|command")` are not covered; the real barrier is the
  sandbox planned for the guest process.
- The stand-in for the built-in scripts is all the tests have. Whether the extracted file
  evaluates cleanly (it defines `RPG::Sprite < ::Sprite`, so it needs the scene classes) can
  only be checked once `add-rgss-scene-classes` is merged.
- `Graphics.transition` clamps `vague` to the range of the protocol field (0 to 65535); the
  range the original accepts is not measured.
