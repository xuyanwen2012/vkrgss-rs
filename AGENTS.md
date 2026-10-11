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
- Never commit any original DLL, RTP asset, or Enterbrain Ruby source. Tests use procedurally
  generated images only.
- Reference material of the original runtime (its DLLs, the RTP, games) lives outside the
  repository and is never committed.
- Gate before declaring done: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` (format
  check, clippy with warnings denied, workspace tests, the Vulkan tests again without the
  validation layer, documentation with warnings denied, `openspec validate --all`).
- The Vulkan tests refuse to run unless the loader is restricted to chosen drivers with
  `VK_LOADER_DRIVERS_SELECT`, because the loader loads every installed driver; see
  [docs/building.md](docs/building.md).

## Scope and proportion

- The requirements and scenarios of the change are the acceptance criteria, for the author and
  for the reviewer alike. Meet them; do not read them more strictly than they are written.
- Do not add machinery that the change's `design.md` does not name: a second implementation
  next to a library, a validator in front of one, a fallback path. If a requirement seems to
  need it, or a review finding asks for it, record the question under a "Follow-ups" heading
  at the end of the change's `design.md` instead of building it, and go on.
- Old games must run. Where the original runtime, or the decoders and libraries of its time,
  accept an input, accept it too.
