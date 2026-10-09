# Design

## Context

- `vkxp-binding` defines `Win32API` in its Ruby prelude: the constructor stores library,
  function, argument and return specifications; `call` returns 0 and reports each unknown pair
  once.
- In the original, `Win32API.new(dll, func, import, export)` takes `import` as a string or an
  array of `p` (pointer to a Ruby string's bytes), `l`, `i`, `n` (integers), `v` (none), and
  `call` passes Ruby strings as mutable buffers.
- Scripts written for Chinese and Japanese Windows expect ANSI functions to speak the game's
  legacy code page; vkxp knows that encoding from reading `Game.ini`.
- The guest's time comes from frame ticks, to keep runs deterministic.

## Decisions

- `call` becomes a host function: it looks the pair (library name lower-cased without `.dll`,
  function name) up in a table of Rust closures over the guest state. Integer arguments arrive
  as `i64`, `p` arguments as access to the Ruby string's bytes, which a shim may overwrite in
  place up to the string's current length (never growing it).
- Profile functions parse the file with the `Game.ini` reader of `vkxp-vfs` semantics
  (sections, keys without regard to case), resolve the path against the game directory and
  refuse anything that leaves it. Values are exchanged in the file's bytes.
- Text conversion supports code pages 65001, 936, 932, 950, 0 and 1 (the latter two meaning
  the game's code page) through the encoding crate the workspace already uses.
- Tick functions derive milliseconds from the frame count and frame rate, not from the clock.
- Window functions return the fixed handle `0x0001_0001`; rectangles are the logical size.
  `ShowCursor` keeps a counter and has no visible effect yet (PLACEHOLDER).
- Functions outside the table keep the report-once behaviour.

## Risks / Trade-offs

- [A script passes a buffer shorter than the data] → Truncate as Windows does and return the
  truncated length.

## Follow-ups

- The tick count is accumulated per frame shown (1000 over the frame rate each) instead of
  being computed from `Graphics.frame_count`: scripts set that counter back (a new game, a
  loaded save), and a tick count must not go backwards. It still comes from frames and the
  frame rate only.
- The game's code page (code pages 0 and 1) is detected again from the bytes of `Game.ini`,
  because `GameIni` and `RunConfig` do not carry the encoding the file was read with. An
  encoding override given to the host is therefore not seen. Should `vkxp-vfs` expose the
  encoding it chose?
- A `Game.ini` that is valid UTF-8, which includes plain ASCII, makes the game's code page
  UTF-8. The original would use the system's ANSI code page; is a configuration value wanted?
- Profile functions with a null section or key (listing the sections or the keys) and
  `WritePrivateProfileStringA` with a null key or value (deleting) are not emulated: reading
  gives the default and writing returns 0.
- Profile files are read from the game directory only, not from the encrypted archive.
- `RtlMoveMemory` copies between two strings. Scripts that pass a raw address obtained
  elsewhere get no copy, because the guest has no such memory.
- `GetSystemMetrics` answers only the indices 0 and 1; window frame and caption metrics, which
  window-resizing scripts ask for, return 0.
- A file name with bytes outside UTF-8 (a legacy-encoded name) is not found by the profile
  functions.
