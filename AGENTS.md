# vkxp agent guide

vkxp is a Rust + Vulkan runtime for RPG Maker XP (RGSS1) games. Read `docs/decisions.md` before
changing anything; it records decisions that are settled.

## Rules

- Everything in this repository is English: code, comments, commits, OpenSpec artifacts.
- Work only on the OpenSpec change you were given. Follow its `tasks.md` in order and tick a
  task only when its stated verification passes.
- Respect the guest/host boundary: only plain data from `vkxp-proto` crosses it. Guest crates
  never depend on `vkxp-vk`, `vkxp-audio` or `vkxp-host`.
- `vkxp-core` does no file I/O, has no Ruby or Vulkan dependency, and forbids `unsafe`.
- `unsafe` is allowed only in `vkxp-vk`, `vkxp-ruby-sys`, `vkxp-binding` and platform code.
- Never commit anything from `../ref` or `../games`, any original DLL, RTP asset, or Enterbrain
  Ruby source. Tests use procedurally generated images only.
- Gate before declaring done: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` (format
  check, clippy with warnings denied, workspace tests, documentation with warnings denied,
  `openspec validate --all`). The Vulkan tests refuse to run unless the loader is restricted to
  chosen drivers.
- Chat with the owner in Chinese; everything written to the repository stays English.

## Scope and proportion

- The requirements and scenarios of the change are the acceptance criteria, for the author and
  for the reviewer alike. Meet them; do not read them more strictly than they are written.
- Do not add machinery that the change's `design.md` does not name: a second implementation
  next to a library, a validator in front of one, a fallback path. If a requirement seems to
  need it, or a review finding asks for it, record the question under a "Follow-ups" heading
  at the end of the change's `design.md` instead of building it, and go on.
- Old games must run. Where the original runtime, or the decoders and libraries of its time,
  accept an input, accept it too.

## Reference material (outside the repository)

The workspace directory that holds the main checkout (`vkxp/`) also holds `ref/` and `games/`.
From the main checkout they are `../ref` and `../games`; from a clone or worktree under
`wt/<name>/` they are `../../ref` and `../../games`. Nothing in them is ever committed.

- `ref/sys/` original RGSS DLLs, `ref/sys/RGSS104E.unpacked.dll` for disassembly.
- `ref/app/` RTP assets. `ref/RGSS104E_api.md` API list with arities (Chinese).
- `ref/tools/unpack/` unpacker and analysis scripts.
- `games/` local test games.

## Working inside an agent flow

- A flow runs in a full clone under `wt/<change>/`, on a branch named after the change. Commit
  there as usual, one commit per task group. Do not push, archive the change or sync specs.
- The sandbox may make `~/.cargo` read-only. If cargo cannot write its cache, set
  `CARGO_HOME="$PWD/target/cargo-home"`.
- Files that must not be committed (downloads, extracted data) go under `local/`, which is
  ignored.
- Building needs the development files of ALSA and libudev (`alsa-lib-devel` and
  `systemd-devel` on Fedora), which are installed on the development machine.
- GPU rule for the development machine: the Intel Arc B580 is reserved for unrelated
  measurement work and must never be used or opened. Naming a device is not enough, because
  the Vulkan loader loads every installed driver to enumerate devices. Every command that
  reaches Vulkan (tests, the gate, examples, the game itself) therefore restricts the loader:
  `VK_LOADER_DRIVERS_SELECT='*lvp*'` for llvmpipe, which is the default for tests, or
  `VK_LOADER_DRIVERS_SELECT='*radeon*' VKXP_TEST_DEVICE=RADV` for a hardware run on the AMD
  integrated GPU.
