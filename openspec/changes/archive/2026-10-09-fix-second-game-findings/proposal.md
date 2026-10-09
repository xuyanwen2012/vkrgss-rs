# Proposal

## Why

A second, much larger game (432 scripts, its own RGSS104E.dll, no RTP) was run in vkxp and in
the original. It reaches its title screen and its first map in both with the same picture
content, after three obstacles were worked around by hand:

- Its scripts define `class Scene_Save < Scene_File_Ex` when `Scene_Save < Scene_File` already
  exists. Ruby 1.8.1 silently replaces the class; the embedded 1.8.7 raises `TypeError`
  ("superclass mismatch") and the game does not start.
- It calls `Win32API.new("kernel32", "GetPrivateProfileString", ...)` without the `A` suffix,
  which Windows resolves to the ANSI function and vkxp reports as not emulated; and it calls
  `CopyFileA`, `SetPriorityClass` and `GetProcessTimes`.
- The oracle harness cannot read its script archive, whose script identifiers are large
  integers stored as Bignums.

## What Changes

- The vendored interpreter defines a new class, replacing the constant, when a class is
  reopened with an explicit superclass other than its own, as measured on the original.
- `Win32API` falls back to the `A` name when the plain name is not in the table, and emulates
  `CopyFileA` (inside the game directory), `SetPriorityClass` and `GetProcessTimes`.
- The harness's archive reader and writer accept and preserve Bignum identifiers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: class redefinition with another superclass.
- `rgss-runtime`: more emulated functions and the name fallback.
- `oracle-harness`: archives with large identifiers.

## Impact

- `third_party/ruby-1.8.7/eval.c` with its patch record and `vkxp-ruby-sys` tests;
  `crates/vkxp-binding/src/win32api.rs`; `xtask/src/oracle`.
