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
- Gate before declaring done: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace`, `openspec validate --all`.

## Reference material (outside the repository)

- `../ref/sys/` original RGSS DLLs, `../ref/sys/RGSS104E.unpacked.dll` for disassembly.
- `../ref/app/` RTP assets. `../ref/RGSS104E_api.md` API list with arities (Chinese).
- `../ref/tools/unpack/` unpacker and analysis scripts.
- `../games/` local test games.
