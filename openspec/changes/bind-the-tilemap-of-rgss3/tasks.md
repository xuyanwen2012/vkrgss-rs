# Tasks

## 1. The class of RGSS3

- [x] 1.1 Before anything changes, keep the lists of tests under `local/` as the implementation notes of `design.md` say. In `crates/vkxp-binding/src/tilemap.rs`: the constructor that calls `Scene::create_tilemap_rgss3`, `flags` and `flags=`, and the third shape of `define` (decisions 1 to 3). New test file `crates/vkxp-binding-rgss3/tests/tilemap_rgss3.rs` (decision 7) with the method surface, the values and the errors. Verify: the ten scenarios of "Tilemap class of RGSS3" and the six of "Tables of a tilemap of RGSS3"; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes with every name of the kept lists, no expected value of an existing test changed, and `git diff main -- crates/vkxp-binding-rgss3/tests/ruby192.allow` is empty.

## 2. What it draws, and a disposed tilemap

- [x] 2.1 Tests in `tilemap_rgss3.rs` that read pixels of frames drawn by the reference compositor: hidden, the star tiles, the shadow, the maps of fewer layers, changes through the table objects. A fault they show is fixed in `tilemap.rs`; one in `vkxp-core` is recorded under Follow-ups of `design.md` and its task stays unticked. Verify: the eight scenarios of "A tilemap of RGSS3 is drawn by the rules of RGSS3"; `cargo test -p vkxp-binding -p vkxp-binding-rgss3 --locked` passes.
- [x] 2.2 The disposed tilemap and its array (decision 4). Verify: the four scenarios of "Disposed tilemap of RGSS3"; the same test command passes twice in a row, and `cargo test -p vkxp-binding-rgss3 --locked --test tilemap_rgss3` passes with `VKXP_GC_STRESS=1`.

## 3. Documents

- [ ] 3.1 `docs/compatibility.md`, the section on VX Ace: the class `Tilemap` and the known differences, as the implementation notes of `design.md` say; `CHANGELOG.md`; the documentation of `tilemap.rs` and, where it names what a tilemap keeps, of `lib.rs`. Verify: the text names nothing that the code does not do, and names the write to a slot of a disposed tilemap as a difference in which the original ends and vkxp goes on; `cargo test -p xtask --locked` passes.

## 4. Integration check

- [ ] 4.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate bind-the-tilemap-of-rgss3 --strict` passes; every name of the kept lists is listed again; `git diff main -- crates/vkxp-core crates/vkxp-proto crates/vkxp-host crates/vkxp-vk crates/vkxp-audio crates/vkxp-vfs third_party` is empty.
