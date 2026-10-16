# See changes made within one tick of the file system's clock

## Why

The first test run on aarch64 hardware (Jetson Orin Nano, Ubuntu 24.04, kernel 6.8,
2026-10-16) failed two tests of `vkrgss-vfs` every time:
`names_with_dots_and_separators_skip_the_archive_of_rgss3` and
`a_loose_data_file_of_rgss3_is_found_from_the_current_directory`. Both open a game and then
write a file into a directory that was already listed.

The listing cache of `vkrgss-vfs` (`dir.rs`) keeps a listing while the directory's
modification time equals the one read with it. On kernels before 6.13, which have no
multigrain timestamps, that time is taken from a coarse clock: on the Jetson
(`CONFIG_HZ=250`) it advances in steps of 4 ms. A file added within the same step as the
listing leaves the time unchanged, and the cache keeps the old listing. The development
machine (kernel 7.2) and GitHub's runners pass by luck of timing or finer timestamps.

A game can meet this on any such system, Ubuntu 24.04 LTS included: a script that writes a
file and reads it back at once (a save and its thumbnail, as The Witch's House does) can be
told the file does not exist. The requirement "Lookups see changes made during the run" is
not met there.

## What Changes

- A listing whose directory was modified too close to the moment it was read is not trusted
  as current: it is read again at the next lookup, until a listing is read after the
  directory's time has settled. This is the "racy" rule git uses for its index.
- A test that does not depend on timing: it gives the cache a directory whose modification
  time it sets to the moment of the listing, adds a file without changing that time, and
  expects the file to be found.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-files`: lookups see changes made within one tick of the file system's clock.

## Impact

`crates/vkrgss-vfs` (`dir.rs`, `files.rs`, tests), `CHANGELOG.md`. A lookup in a directory
changed within the last moment reads it again; no other behaviour changes.
