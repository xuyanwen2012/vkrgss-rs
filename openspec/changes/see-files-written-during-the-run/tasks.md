# Tasks

## 1. File layer

- [x] 1.1 Cached listings are checked against the directory's modification time; missing directories are not cached. Verify: the scenarios of `specs/game-files/spec.md` at the level of `GameFiles`; `cargo test -p vkxp-vfs --locked` passes.

## 2. Through a game

- [x] 2.1 A generated game that makes a directory with `Dir.mkdir`, writes a picture into it with `File.open`, and loads it with `Bitmap.new` under a name with an unknown extension. Verify: the scenario "A picture a script wrote"; `cargo test -p vkxp-binding --locked` passes.

## 3. Integration check

- [ ] 3.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate see-files-written-during-the-run --strict` passes.
