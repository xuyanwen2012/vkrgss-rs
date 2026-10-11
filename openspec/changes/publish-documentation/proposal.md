# Documentation for publication

## Why

The repository is about to be published on GitHub. Its README is an 857-line reference
manual, which is the wrong first page: a visitor cannot see in a minute what vkxp is, how far
it is, and how to run a game. There is no changelog, no contribution guide and no notice file
for the bundled third-party source. `AGENTS.md` and `CLAUDE.md` carry rules that belong to
the owner's machine (a reserved GPU, the language of the chat, paths outside the repository),
and generated agent skill files are tracked.

## What Changes

- `README.md` becomes a landing page; the manual moves to topic files under `docs/` with its
  text kept.
- New: `docs/compatibility.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `THIRD-PARTY.md`, a
  screenshot of the built-in demo.
- `AGENTS.md` keeps the rules of the repository only; `CLAUDE.md` points to it; `.claude/`
  and `.agents/` are no longer tracked.
- A test keeps the relative links of the documents valid.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `workspace-structure`: the public documents and their link check.

## Impact

Documents, `.gitignore`, one test file in `xtask/tests/`. No code of the runtime changes.
