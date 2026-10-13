## Why

The Witch's House (RPG Maker VX, in the corpus) ends when the player saves: its save script
makes the directory `Save` with `Dir.mkdir`, writes a thumbnail there as `Save/1.cdat` with
Ruby's `File`, and reads it back with `Bitmap.new("Save/1.cdat")`, which vkxp answers with
"Unable to find file Save/1.cdat" although the file exists. The file layer caches directory
listings and drops them only after `save_data` and some emulated `Win32API` calls; a file
or directory made by plain Ruby is not seen until then. Found on 2026-10-12 while walking
to the game's save point for milestone V3.

## What Changes

- A file or directory that appears, disappears or is renamed in a game directory or a
  run-time package while the game runs is found, or no longer found, by the next lookup,
  whoever made the change.
- Lookups of unchanged directories stay as cheap as today.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-files`: lookups see the directory as it is.

## Impact

- `crates/vkxp-vfs` (`files.rs`, `dir.rs`).
- A test of the binding with a generated game that writes a picture and loads it.
