# Design

## Decisions

### The sporadic failures are explained before they are fixed

For each of the two tests: reproduce it (the whole workspace suite running at the same time
is what showed it; `stress`-like load from a second `cargo test` is a way), find the cause,
write it under "Found" with the evidence, and remove the cause.

- A cause in the runtime (for the first test, for example, an object that the garbage
  collector frees or a scene element disposed although the script still holds it after a
  copy; for the second, statistics of a guest process that are lost when the host ends the
  run at its frame limit) is fixed in the runtime, with a test that fails without the fix
  if one can be written that does not depend on timing.
- A cause in the test alone is fixed in the test.
- Retrying a test, lengthening a timeout without a stated reason, marking a test ignored or
  weakening what it asserts is not a fix.

The verification is by repetition under load: each of the two tests passes 200 times in a
row while `cargo test --workspace --locked` runs in a loop beside it.

If a cause cannot be found, what was tried and observed goes under "Follow-ups", the task
stays unticked and the run ends as the brief says.

### Help option

`--help` and `-h` are recognised by `args::parse` like `--version`: `main` prints the usage
text to the standard output and returns status 0 before the log, the configuration or the
game directory are touched. An unknown option keeps printing the usage to the error stream
with status 2. The usage text lists `--help`; `docs/running.md` lists it with `--version`.

### Comments and the notice file

- `crates/vkxp-vk/src/device.rs`, `xtask/src/oracle/mod.rs`,
  `crates/vkxp-core/tests/data/sprite-rotation-samples.txt`: "see the README" becomes the
  file under `docs/` that holds the text now.
- `crates/vkxp-ruby-sys/tests/sanitized.rs`: the mention of "the development machine"
  becomes what the sentence is about (the distribution or the tool).
- `THIRD-PARTY.md`: a section "Fonts used by tests" naming
  `crates/vkxp-text/tests/fonts/LiberationSans-Regular.ttf` and
  `NotoSansSymbols2-Regular.ttf`, their licences as the `LICENSE.*.txt` files beside them
  state, and that they are not part of the executable.

## Found

### `a_guest_that_is_copied_sends_the_frames_it_sends_without`: a cause in the test harness

The cause is in `vkxp_binding::testing::host::run_game`, not in the runtime and not in the
copying of a guest.

- An element object (`Sprite` and the others) keeps the handle of its element after it is
  disposed, and when the collector frees the object the handle goes to a queue; the next
  entry into the guest disposes the element the handle names, if there is one
  (`element.rs`). A handle is a slot and a generation, and a scene never hands one out
  twice, so in a run of the program a queued handle of a disposed element names nothing.
- The harness plays every game of a test binary on one interpreter. Between two games it
  disposed what the earlier one left and then gave the guest a new `Scene`, whose handles
  start again at slot 0, generation 0. The objects of the earlier game are garbage from
  then on, but they are collected only when the interpreter next collects, which is in the
  middle of a later game. Their handles then name elements of that game, and these are
  disposed under the script: `RGSSError: disposed sprite` at the next use, line 4 of `Main`
  in the test named above.
- Whether a collection falls inside the 80 frames of that game depends on how much the
  interpreter has allocated before, which depends on the order in which the tests of the
  binary reach the interpreter thread, and so on the load of the machine.

Evidence: the test
`the_collected_objects_of_an_earlier_game_dispose_nothing_of_a_later_one` in
`testing/host.rs` plays a game that creates 200 sprites and then one that creates 200,
calls `GC.start` and counts the disposed ones. Before the fix it counted 200 of 200, every
time; it does not depend on timing.

Removed by keeping the scene: `Scene::clear` in `vkxp-core` empties a scene and puts back
what `Scene::new` sets while the handles go on, and `run_game` of the harness calls it in
place of making a new scene. The program itself never replaced its scene (a reset disposes
the elements one by one), so nothing in the runtime changes.

### `the_guest_splits_its_frame_time_when_statistics_are_requested`: a cause in the runtime

A guest process sends its outcome and the shutdown message, closes its connection, and only
then writes its frame time statistics to the error stream (`runtime::play`); a game that
could not be prepared writes why at that point too (`game_succeeded` in `main.rs`). The
relay in the parent (`process.rs`) returned as soon as the host had gone and that
connection had ended, and `GuestProcess::relay` then sent the child SIGKILL at once. The
child had from the closing of its socket to that signal to write; on a busy machine it was
ended first, and the rows were lost although the run ended with status 0.

Evidence: a failing run of the workspace suite under load, kept from before the fix, shows
the host's rows (`vkxp: frame times over 50 frame(s):` and its stages), status 0 and no
`guest ...` row. The test
`a_guest_process_that_has_closed_its_connection_is_left_to_end_by_itself` in `process.rs`
gives the relay a child that writes a file 0.3 seconds after its connection ended: without
the fix the file is never written. That test waits on a clock, but only its failing without
the fix depends on it; with the fix it passes however slow the machine is.

Removed in `GuestProcess::relay`: a child that was the running guest to the end and closed
its connection, or ended, by itself is waited for until it has gone, for `GUEST_WAIT` (5
seconds) at most, which is the time a guest is already given to end after the host has
gone. A guest that was given up on, or whose place a loaded state took, is ended at once as
before. A resumed copy that is the running guest at the end was never ended by the relay
and is not waited for; its rows reach the error stream after the parent has gone.

### Repetition under load

The load is `cargo test --workspace --locked` in a loop with a target directory of its own
(`CARGO_TARGET_DIR`). The game directories of a test binary lie next to the executable and
are made afresh when it starts, so two runs of one test executable at the same time, which
a loop in the same target directory would bring about, remove each other's files. That is
no part of the two failures above, which showed in single runs of the suite.

## Follow-ups

- Two runs of one test executable at the same time share its generated game directories
  (see "Repetition under load"). Not changed here.
- A resumed copy of the guest process that ends the game is not waited for by the parent,
  so its last lines on the error stream can follow the end of the parent. Nothing is lost.
