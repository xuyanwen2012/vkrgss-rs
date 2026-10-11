# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0]

The first public release.

### Added

- A runtime for RPG Maker XP (RGSS1) games on Linux x86_64: the RGSS classes on an embedded
  Ruby 1.8.7-p374 with Oniguruma 5.9.6, built from vendored source.
- A Vulkan compositor and a CPU reference compositor that produce byte-identical frames.
- A window with integer or filtered scaling, borderless fullscreen and frame pacing.
- Audio: Ogg Vorbis, WAV, MP3 and, with a SoundFont, MIDI.
- Keyboard and gamepad input with key maps that can be changed.
- Encrypted archives (`Game.rgssad`) and run-time packages.
- Configuration files for the user and for a game.
- Fast-forward, input recording and replay, and save states.
- A headless mode that writes chosen frames as PNG files.
- `cargo xtask extract-rpg`, which copies the built-in scripts from an RGSS DLL of the user.
- `cargo xtask oracle`, which compares frames with the original runtime under Wine.
