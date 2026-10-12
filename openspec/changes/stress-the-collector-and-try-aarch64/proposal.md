# Stress the collector and try aarch64

## Why

The embedded Ruby 1.8.7 has a conservative collector: a value is kept alive only while a
copy of it lies on the machine stack, in a register, or in an object that is reachable. C
code and the Rust binding that hold a value only through a pointer into it (the buffer of a
string, the slots of an array) lose it when a collection comes at the wrong moment. Two such
defects were found in `marshal.c` on 2026-10-11, and only because another machine's
compiler laid the stack out differently. They were then looked for by reading `marshal.c`,
`pack.c` and `zlib.c`. The rest of the tree and the 42 `unsafe` places of `vkxp-binding`
have not been looked at this way, and reading is the wrong tool: a run in which the
collector runs at every allocation makes each such defect fail at once, and under
AddressSanitizer it fails with a report that names the place.

The interpreter has the switch already (`GC.stress`), but nothing runs with it.

Separately, `docs/decisions.md` lists "aarch64 is untested" as a risk. GitHub has aarch64
runners; whether the tree builds and passes there can be found out for the price of a job.

## What Changes

- An environment variable makes every interpreter that `vkxp-ruby-sys` starts collect at
  every allocation, or at every n-th.
- `cargo xtask stress` runs the upstream suites in the sanitized build and the tests of the
  binding with it. It is not a gate step.
- What the first run finds is fixed where the fix is small, and listed otherwise.
- A second CI job runs the gate on aarch64 and is allowed to fail; the documents say that
  aarch64 is not supported and where its state can be seen.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: tests run under collector stress.
- `workspace-structure`: the aarch64 job.

## Impact

`crates/vkxp-ruby-sys` (start-up, tests), `third_party/ruby-1.8.7` (recorded patches only),
`crates/vkxp-binding` (fixes of what the run finds, nothing else), `xtask` (the command),
`.github/workflows/ci.yml`, `docs/development.md`, `docs/compatibility.md`,
`docs/building.md`. No behaviour of a game changes except where a defect is fixed.
