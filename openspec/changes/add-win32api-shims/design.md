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
