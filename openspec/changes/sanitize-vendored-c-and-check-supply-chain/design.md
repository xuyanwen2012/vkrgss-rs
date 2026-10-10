# Design

## Context

- `crates/vkxp-ruby-sys/build.rs` builds the interpreter and Oniguruma with `cc` and exports
  the compiler, its flags and the directories (`VKXP_RUBY_CC`, `VKXP_RUBY_CFLAGS`, ...).
  `tests/upstream.rs` rebuilds Ruby's own test executable from them and runs the upstream
  suites (625 tests).
- Patches to the vendored tree are recorded (see the "Vendored source matches upstream"
  requirement and the patch list it checks).
- The garbage collector is conservative: it scans the machine stack and registers word by
  word. Under AddressSanitizer such a scan reads red zones and is reported unless the
  scanning functions are excluded from instrumentation, and the "fake stack" of
  use-after-return detection hides stack values from the scan.
- `crates/vkxp-vfs/src/archive.rs` parses the archive; `proptest` is a workspace dependency
  used by other crates.
- `cargo deny` is not installed on the development machine and needs the network for the
  advisory database.

## Decisions

### Sanitizer run

- A test in `crates/vkxp-ruby-sys/tests/` marked `#[ignore]`, run with
  `cargo test -p vkxp-ruby-sys --test sanitized -- --ignored`. It compiles the interpreter,
  Oniguruma and the test executable itself, in a directory of its own under the target
  directory, with the exported flags plus `-fsanitize=address,undefined
  -fno-omit-frame-pointer -g`, and runs the upstream suites with
  `ASAN_OPTIONS=detect_stack_use_after_return=0:detect_leaks=0` and
  `UBSAN_OPTIONS=print_stacktrace=1:halt_on_error=0`. Leak detection is off: the interpreter
  frees nothing at exit by design.
- The stack and register scan of the collector is excluded from AddressSanitizer with the
  compiler's attribute, as a recorded patch to the vendored tree. Nothing else is excluded
  without a written reason.
- The run passes when the suites pass and the sanitizers report nothing outside a short
  allow-list kept next to the test, each entry with the reason it is harmless here (for
  example a deliberate unaligned read in a hash function).
- Findings: a real invalid access or an overflow a game could reach is fixed in the vendored
  tree as a recorded patch, with a test where one is practical, when the fix is a few lines.
  Larger ones are listed under "Follow-ups" with the sanitizer's report, and allow-listed so
  that the run is usable.
- If the sanitized build cannot be made to start at all (the collector and the sanitizer
  runtime disagree beyond the scan), fall back to UndefinedBehaviorSanitizer alone and say so
  under "Checked".
- The result of the run, with the list of findings, goes under a "Checked" heading at the end
  of this file.

### Archive property tests

- In `crates/vkxp-vfs`, with `proptest` as a dev-dependency: (1) arbitrary bytes after a
  valid header; (2) a valid archive built by the existing test helper with a few bytes
  flipped, or cut at an arbitrary length. The parser returns `Ok` or `Err`; reading every
  listed entry returns `Ok` or `Err`; no panic. An entry whose declared length exceeds the
  bytes that remain is an error before anything of that size is allocated.
- If the parser fails a property, fix the parser.

### Audit

- `deny.toml` at the root: advisories (deny vulnerabilities, warn unmaintained), licences
  (an allow-list made from what the dependency graph has today, which must stay compatible
  with MIT distribution), sources (crates.io only), bans left at defaults.
- `cargo xtask audit` runs `cargo deny --locked check`. When `cargo-deny` is not installed it
  ends with a message that says how to install it. It is not a gate step.
- If the tool or the network is not available in the run that implements this, the command
  and the configuration are still written, the unit test covers how the command is built and
  the missing-tool message, and "Checked" says that the check itself was not run.

## Risks / Trade-offs

- [The sanitizer run is fragile across compiler versions] → It is opt-in and outside the gate.
- [An allow-list hides a real problem] → Each entry carries its reason; the list is reviewed
  with the change.

## Follow-ups

- Findings that are not fixed here.
- Running the audit on a schedule once the repository has a remote.
