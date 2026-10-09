# Design

## Context

- `vkxp`'s `main` parses options by hand into one options value and passes parts of it to the
  guest's run configuration and to the host runner.
- `vkxp-host` has a fixed default key map from window key codes to RGSS buttons and builds an
  input snapshot per frame; recorded and replayed input bypass live input.
- The design record places configuration and gamepads in milestone M5.

## Decisions

- Configuration: a `serde` structure with `deny_unknown_fields`, read with `toml`. Layering is
  done on optional fields: user file, then game file, then command line. Paths in a file are
  relative to that file. Section `[rtp]` maps names to directories, `[keys]` and `[gamepad]`
  map button names (`DOWN`, `C`, `F5`, ...) to lists of key or pad button names.
- Key names are those of the windowing library's logical key codes, documented in the README
  with the default map as an example.
- Gamepad: `gilrs` in a small module of `vkxp-host` that turns its events into a set of held
  RGSS buttons, merged into the snapshot with the keyboard's. The mapping logic is a pure
  function over an abstract pad state so that every scenario is tested without a device; the
  `gilrs` layer is thin. If `gilrs` fails to initialise, one report is made and the run goes on.
  Default table: south C, east B, west A, north X, left and right shoulder L and R, start C,
  select B; dead zone 0.5.
- Headless runs read no gamepad.

## Risks / Trade-offs

- [`gilrs` needs `libudev` development files on Linux] → Check what the build needs; if it is
  not on the development machine, say so in the final report and in the README prerequisites.
