# Proposal

## Why

Everything about a run is a command-line option today, the key map is fixed and there is no
gamepad support. Milestone M5 asks for a configuration file and gamepads.

## What Changes

- A configuration file in TOML: a user file (`$XDG_CONFIG_HOME/vkxp/config.toml`) and an
  optional per-game file (`vkxp.toml` in the game directory), the latter overriding the former
  and command-line options overriding both. It holds what the options hold today (RTP
  directories, built-in scripts file, device, fonts, SoundFont, audio on or off, seed, preload
  scripts) and the key and gamepad maps.
- A configurable key map: each RGSS button gets a list of keys; unlisted buttons keep their
  defaults.
- Gamepads through `gilrs`: the directional pad and left stick are the directions, the face
  and shoulder buttons map to RGSS buttons by default and by configuration; pads may be
  connected and removed while the game runs.
- `--config FILE` to name the user file, `--no-config` to ignore both files.

Out of scope: a settings screen, rumble, per-pad maps, analog input for scripts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `game-launch`: configuration files.
- `host-session`: configurable key map and gamepads.

## Impact

- Crates `vkxp-host` (key map, gamepad input) and `vkxp` (configuration loading), README. New
  dependencies `toml`, `gilrs`.
