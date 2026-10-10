# Tasks

## 1. Framing and stream transport (`vkxp-proto`)

- [x] 1.1 Add the framing module (write and read one message on `Write` / `Read`). Verify: the scenarios of "Stream framing" in `specs/guest-host-protocol/spec.md` as tests; `cargo test -p vkxp-proto --locked` passes.
- [x] 1.2 Give `GuestEnd` the Unix stream transport. Verify: the scenarios of "Guest end over a stream socket" as tests on a socket pair; `cargo test -p vkxp-proto --locked` passes.

## 2. Guest process (`vkxp`)

- [ ] 2.1 Add the option, the configuration key and the environment variable. Verify: scenarios "Unknown value" and "Environment" (the latter may be checked together with 2.3); `cargo test -p vkxp --locked` passes with `VK_LOADER_DRIVERS_SELECT='*lvp*'`.
- [ ] 2.2 Add the guest role (hidden option, configuration settled as in the parent, `run_game` on the socket transport, exit status, parent-death signal) and the parent's side (private directory, listening socket, start of the child, relay, wait and end). Verify: `cargo test -p vkxp --locked` passes with `VK_LOADER_DRIVERS_SELECT='*lvp*'`.
- [ ] 2.3 Add end-to-end tests in `crates/vkxp/tests/` for the scenarios "Same frames", "Script error", "Guest process dies" and "Frame limit". Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkxp --locked` passes.
- [ ] 2.4 Frame statistics in the process form: the guest's rows are written by the guest process. Verify: the existing statistics test of `end_to_end.rs`, run in both forms, finds the same row names.

## 3. Documentation and measurement

- [ ] 3.1 Describe the option in `README.md` and replace the sentence of `docs/decisions.md` that says both sides are threads of one process by what holds now (thread by default, process on request, save states still to come). Verify: `cargo xtask gate` checks pass for documentation; the texts name `--guest`.
- [ ] 3.2 Measure a headless run of a generated game of at least 300 frames with `--frame-stats` in both forms on llvmpipe and record the two summaries under a heading "Checked" in `design.md`. Verify: the heading exists with both sets of numbers.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate run-the-guest-in-a-process --strict` passes.
