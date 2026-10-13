## Context

- `GameFiles` (`crates/vkxp-vfs/src/files.rs`) keeps the listing of every directory it has
  looked into and never reads it again until `GameFiles::refresh` is called. The binding
  calls `refresh` after `save_data` and after the emulated file functions of `Win32API`.
- Scripts also change the game directory with Ruby itself (`File.open(..., "wb")`,
  `Dir.mkdir`, `File.delete`, `File.rename`), which nothing reports to the file layer.
- The case found: scripts `122` of The Witch's House (a save screen with a thumbnail
  written by a PNG writer in Ruby).

## Goals / Non-Goals

**Goals:** a lookup answers from the directory as it is on the disk at that moment.

**Non-Goals:** the encrypted archive (it does not change); watching directories; changing
which extensions are tried or the order of the roots.

## Decisions

- A cached listing remembers the modification time of its directory as read just before
  the listing was taken. Every use of a cached listing first asks the directory's
  modification time (one `metadata` call) and reads the listing anew when it differs or
  cannot be read. A directory's modification time changes when an entry is added, removed
  or renamed in it, which is all a listing holds.
- A directory that does not exist is not cached at all, so one that is made later is seen.
- Two changes within the resolution of the file system's time stamps could hide the
  second. `refresh` stays, and the binding keeps calling it where it does, so the known
  writers are safe from that; the comment at the cache says so.
- No hooks on Ruby's `File` and `Dir` are added.
- The test `listings_are_cached_until_refresh` of `files.rs` asserted the behaviour this
  change removes: that a file written after a lookup is not found until `refresh`. It is the
  one existing test that could not stay as it was; the tests of the three scenarios at the
  level of `GameFiles`, and one of a rename, take its place.
- The tests wait 20 ms between the listing and the change of the directory, so that they
  do not depend on the resolution of the file system's time stamps.
