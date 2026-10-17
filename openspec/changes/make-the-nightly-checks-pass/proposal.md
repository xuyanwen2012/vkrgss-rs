# Make the nightly checks pass

## Why

The first scheduled run of `nightly.yml` (run 38872699059, 2026-10-16, at `d505bd48`) failed
in four of its ten jobs:

- `audit`: `cargo-deny` 0.18.2 cannot parse the current advisory database ("failed to load
  advisory database: parse error ... crates/libcrux-chacha..."). The newest release is
  0.20.2.
- `fuzz (parse192)`: UndefinedBehaviorSanitizer, `parse.y:2065:8: runtime error: member
  access within null pointer of type 'struct RNode'` in Ruby 1.9.2's parser.
- `fuzz (parse)`: after several processes ended on libFuzzer's memory limit (expected and
  counted), UndefinedBehaviorSanitizer reported `eval.c:496:38` of Ruby 1.8.7
  (`search_method`, `RCLASS(klass)->m_tbl` of a class that is not one).
- `fuzz (regex)`: AddressSanitizer reported a stack overflow in `get_min_match_length`
  (`regcomp.c:2030`) of Oniguruma 5.9.6. A stack exhausted by nesting is not a finding of
  these targets (`docs/development.md`, `docs/vendored-security.md`), but the run counts every
  end other than a clean one or a limit as a finding, and an input of 512 bytes is deep
  enough on the runner.

The inputs of the findings were not kept: the workflow does not upload the artifacts of a
failed fuzz job (a follow-up of `harden-the-c-build-and-pin-ci-inputs`).

## What Changes

- `cargo-deny` pinned at 0.20.2 in `nightly.yml`, and `deny.toml` adapted if the new version
  needs it.
- The fuzz job uploads `target/tmp/fuzz/<target>/artifacts/` when it fails.
- A stack overflow that the sanitizer reports in a function known to recurse on nesting is
  counted like a limit (put aside, not a finding), as the documents already say; any other
  stack overflow stays a finding.
- The two findings of the parsers are reproduced locally by running the targets
  `parse192` and `parse` long enough, fixed as recorded patches with the inputs under
  `fuzz/fixed/` and a test that replays them; if one cannot be reproduced in two hours of
  running, it is listed under "Follow-ups" with what is known.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: stack exhaustion by nesting in a fuzz run is set aside, not a finding.
- `workspace-structure`: the fuzz job keeps the inputs of its findings.

## Impact

`.github/workflows/nightly.yml`, `deny.toml`, `xtask` (tests of the workflow), the fuzz
tests of `vkrgss-ruby-sys` and `vkrgss-ruby192-sys`, `third_party/ruby-1.8.7` and
`third_party/ruby-1.9.2` (recorded patches only), `docs/development.md`,
`docs/vendored-security.md`, `CHANGELOG.md`.
