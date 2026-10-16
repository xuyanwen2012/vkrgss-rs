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

## Measured parts

Measured on 2026-10-16 on the owner's machine (12 threads, llvmpipe) with the build warm.
`cargo test --workspace --locked --no-fail-fast` took 138 s to build and 334 s to run; the
slowest binaries were the unit tests of `vkrgss-vk` (102.6 s, 336 s on CI),
`upstream_stress` of `vkrgss-ruby192-sys` (94.8 s, 412 s on CI), `end_to_end` of `vkrgss`
(36.1 s), the unit tests of `xtask` (20.7 s) and `upstream` of `vkrgss-ruby192-sys`
(13.4 s). CI is about four times slower than this machine for the tests.

`VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate --job <name>`, each part after the other:

| Part | Local, warm | Expected on CI |
|---|---|---|
| `lint` | 36 s (`clippy` 32.1 s) | about 3 min with the build |
| `ruby` | 46 s | about 3 min of tests and the two interpreters |
| `ruby-stress` | 91 s | about 7 min of tests (412 s) and Ruby 1.9.2 |
| `vulkan` | 151 s | about 6 min of tests (336 s) |
| `vulkan-unvalidated` | 133 s | about 6 min of tests (355 s) |
| `rest` | 73 s | about 5 min of tests and the build of the workspace |

A cold build of `rest` with 4 jobs (`CARGO_BUILD_JOBS=4`, empty target directory, the
registry crates included, which `Swatinem/rust-cache` keeps on CI) took 284 s, so the
longest part stays near ten minutes even when sccache misses.

The starting point was changed in one place: the validated Vulkan tests and
`test-unvalidated` took 336 s and 355 s on CI, together over eleven minutes before the
build, so `test-unvalidated` is the part `vulkan-unvalidated` of its own. The validated
tests of `vkrgss-host` are in `rest`, as the starting point has them.

Cargo cannot leave one test target out of a package, so the part `ruby` names the library,
the documentation tests and every other test target of `vkrgss-ruby192-sys` (three cargo
commands). A test target added to that package fails the test of the union until it is
named there; `docs/development.md` says so.

The scenario "Job is present and may fail" of the requirement "aarch64 job" speaks of "the
steps of the gate job"; with the gate in parts, `gate-aarch64` has the steps of the part
jobs, which differ only in the part, and runs the whole gate as its last step, which the
test of the workflows checks. It got sccache like the parts, which needed no other change.
The test that every job sets `CC` now asks it of the jobs that build (run cargo or install
packages), as the requirement "Pinned inputs of the workflows" says: the issue job of the
nightly workflow builds nothing.

## Out of scope

cargo-nextest, required status checks in the repository's settings (the owner sets them),
making `gate-aarch64` required (P1).

## Follow-ups

- The test targets of `vkrgss-ruby192-sys` but `upstream_stress` are listed by hand in
  `xtask/src/gate.rs`. Reading them from `cargo metadata` when the part runs would take a
  new one into `ruby` without a change, at the cost of a lookup the design does not name;
  the owner may decide whether it is wanted.
- The expected CI times above are estimates from local runs; the first runs of the new
  workflow show the real ones.
