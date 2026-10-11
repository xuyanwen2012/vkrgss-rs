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

To be filled in.

## Follow-ups

None yet.
