# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- RPG Maker VX Ace (RGSS3): the class `Tilemap` with `flags` in the place of `passages`,
  drawn by the rules of RGSS3. A write to a slot of `bitmaps` of a disposed tilemap is kept
  where the original runtime ends.

## [0.2.0] - 2026-10-13

### Added

- Support for RPG Maker VX (RGSS2) games beside those of RPG Maker XP. The runtime of a game
  is taken from its `Game.ini`, or from `--runtime`; a game of VX runs at 544 by 416 and 60
  frames per second, with the classes and functions of RGSS2, its tilemap, window, sprite,
  bitmap operations, keys and input repeat, its run-time package (`--rtp RPGVX=DIR`) and
  its encrypted archive (`Game.rgss2a`). It needs the built-in scripts of RGSS2 in a file
  of their own.
- `cargo xtask oracle` compares games of RPG Maker VX with their original runtime, with the
  options `--original-only` and `--no-threads`.
- A sandbox for the guest process: it confines itself with Landlock before the first script
  runs, writes only below the game directory, executes nothing and opens no TCP connection.
  `--sandbox off` turns it off.
- Audio: MPEG audio layer II, WMA where `ffmpeg` is installed, and MIDI music that loops at
  its controller 111.
- A loaded save state recalls the music and the background sound with their positions.
- Keys that a script injects through `Win32API` reach the game, and a script can hide the
  mouse pointer.
- Fuzz targets for the interpreter (`cargo xtask fuzz`), a stress run of the garbage
  collector (`cargo xtask stress`), and a CI job that builds and tests on aarch64.

### Changed

- The protocol between guest and host is at version 12.
- A picture or other file that a script writes during the run is found by later lookups.

### Fixed

- Transitions with a coloured mask, the additive and the subtractive blend on translucent
  destinations, and bitmap transfers above opacity 255, mirrored or from a disposed source
  follow the original runtime.
- A 32-bit BMP whose alpha bytes are all zero is decoded as the original decodes it.
- Faults that fuzzing found in the parser, in regular expression sources and in Oniguruma.

## [0.1.0] - 2026-10-11

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
