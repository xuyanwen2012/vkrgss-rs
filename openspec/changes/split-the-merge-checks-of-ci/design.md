# Design

## Context

`xtask/src/gate.rs` has six steps: `fmt`, `clippy`, `test` (`cargo test --workspace --locked
--no-fail-fast`), `test-unvalidated` (the Vulkan tests without the validation layer),
`doc`, `openspec`. CI timings of 2026-10-15 are in the proposal. The machine of the owner has
12 hardware threads; GitHub's `ubuntu-24.04` runner has 4.

## Decisions

- **Parts.** A part is a list of steps, and a test step may be limited to packages or test
  targets (`-p`, `--exclude`, `--test`). Measure the test binaries locally
  (`cargo test --workspace --locked --no-fail-fast` with `--test-threads` as CI has it is
  not needed; the time of each binary from the output is enough) and choose the parts so
  that the longest is expected under about ten minutes on a 4-thread runner, compile time
  included. A starting point to adjust after measuring:
  - `lint`: `fmt`, `clippy`, `doc`, `openspec`;
  - `ruby`: the tests of `vkrgss-ruby-sys` and `vkrgss-ruby192-sys` except
    `upstream_stress`;
  - `ruby-stress`: `vkrgss-ruby192-sys --test upstream_stress` (the quick stressed files
    the requirement "Collector stress" puts in the workspace tests; it stays in the merge
    checks, as the owner decided);
  - `vulkan`: the tests of `vkrgss-vk`, validated, and `test-unvalidated`;
  - `rest`: every other package.
  The union of the parts, as a set of (step, package, test target), equals the full gate;
  a test of `xtask` checks it, so that a new package or test target cannot fall between
  them. A package added later goes to `rest` by default.
- **Command.** `cargo xtask gate --job <name>`; an unknown name fails and lists the names.
  The report of durations stays.
- **CI.** One job per part in `ci.yml`, each named after its part, with the prerequisites of
  today's job, the pins of today's workflow, `CC: gcc-13`, and sccache for C and Rust:
  `mozilla-actions/sccache-action` pinned at `fc920bf0ec8de6ee65d409111f7ec508035751ba`
  (v0.0.11, resolved on 2026-10-16), sccache version `v0.18.0`, with `SCCACHE_GHA_ENABLED:
  "true"` and `RUSTC_WRAPPER: sccache`; the `cc` crate uses the wrapper for C when
  `RUSTC_WRAPPER` is sccache (check that the version in `Cargo.lock` does; otherwise set
  `CC: sccache gcc-13` only where it works). `Swatinem/rust-cache` stays for the registry
  and the build directory.
- **aarch64.** `gate-aarch64` keeps running the whole gate in one job, unchanged except for
  sccache if it fits without other changes.
- **Nightly issue.** A last job of `nightly.yml` with `needs:` on every job and
  `if: failure()`, `permissions: issues: write`, runs `gh issue` to open an issue titled
  "Nightly checks failed" with the link to the run, or to comment on the open issue of that
  title. Only that job has write permission.
- **Tests.** `xtask/tests/workflows.rs` checks: every part has a CI job that runs
  `cargo xtask gate --job <part>`, the pins rule still holds for the new action, the issue
  job exists with the narrow permission.

## Out of scope

cargo-nextest, required status checks in the repository's settings (the owner sets them),
making `gate-aarch64` required (P1).

## Follow-ups

None yet.
