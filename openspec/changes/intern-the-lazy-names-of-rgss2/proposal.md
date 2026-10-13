## Why

A save that The Witch's House (RPG Maker VX) writes in vkxp has the size of the one the
original writes at the same frame and differs in 43776 bytes: instance variables are dumped
in another order, because 187 names of the game's scripts have other symbol numbers. The
numbers agree until the game's first `Bitmap.new`. From there the original interns names of
its own at the first use of some functions, and vkxp interns other names or none at those
moments (measured on 2026-10-12, `vx-v3-save-REPORT.md` under
`ref/tools/oracle/measurements/`, outside the repository). With a throwaway patch that
interns the same names at the same moments the save is byte for byte the original's.
Milestone V3 asks for exactly that.

## What Changes

- In RGSS2 the names `owner` and `observer`, the six names of `Win32API` and `bmp0` to
  `bmp8` are interned when the original interns them; the tilemap's hidden names of vkxp's
  own are gone.
- The interpreter interns `recv` where Ruby 1.8.1 does and names the members of a `Struct`
  `__member__` as the start-up tables have it.
- A test holds the symbol number after each of the measured steps of a small game of RGSS2
  against the original's.
- Nothing a script can see changes except symbol numbers and what follows from them (the
  order of instance variables and of hash entries in a dump).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `rgss-runtime`: the symbol numbers of a game of RGSS2 after its first script.
- `embedded-ruby`: two names the vendored interpreter interns.

## Impact

- `crates/vkxp-binding` (the value and element classes, `tilemap.rs`, `prelude.rb`,
  `win32api.rs`, tests), `third_party/ruby-1.8.7` (`struct.c`, `error.c`, the patch log),
  `crates/vkxp-ruby-sys` (tests).
