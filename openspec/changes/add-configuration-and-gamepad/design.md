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

## Follow-ups

- The files hold the settings the proposal lists, plus `encoding`, `debug` and
  `allow_process_functions`. The options that shape one run (`--record`, `--replay`,
  `--headless`, `--frames`, `--input`, `--dump`) have no key in the files. Should they?
- The development machine has `libudev.so.1` but not the development files of libudev
  (`libudev.pc`), which `gilrs` needs to build through `libudev-sys`. The change was built
  with a stand-in like the one for ALSA, kept under the ignored `local/udev-standin` and added
  to `PKG_CONFIG_PATH`. Either install `systemd-devel` or move the stand-in next to the ALSA
  one and name it in the agent guide.
- The windowed runner reads the pads whenever its event loop wakes, which a running game makes
  happen at least once per frame. A pad does not wake the loop by itself; whether it should
  (a thread that waits for pad events) is open.
- `gilrs` is built with its default features, because `gilrs-core` refuses to compile without
  a Windows backend feature on every target.
- A seed in a file is a TOML integer, so seeds above 2^63 - 1 can only be given with `--seed`.
- No pad was at hand: the mapping is tested over plain pad states, the `gilrs` layer was not
  exercised with a device.
