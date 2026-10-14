# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- RPG Maker VX Ace (RGSS3): `Font` and the text and creation of `Bitmap` of RGSS3: the
  defaults of RGSS3, `outline` and `out_color`, the objects a font is given kept as given,
  `Font.exist?` exact, `font=` that copies every attribute, text drawn by the rules of RGSS3,
  no limit of 4194304 pixels on `Bitmap.new`, and the argument errors of the original. The
  guest reads the version of the DLL the game ships, by which `RGSS_VERSION` is `"3.0.0"` or
  `"3.0.1"` and the glyph rule is that of 3.0.0.1 or 3.0.1.1, and a game takes fonts from the
  `Fonts` directories of its run-time packages.
- RPG Maker VX Ace (RGSS3): the class `Tilemap` with `flags` in the place of `passages`,
  drawn by the rules of RGSS3. A write to a slot of `bitmaps` of a disposed tilemap is kept
  where the original runtime ends.
- Experimental support for RPG Maker VX Ace (RGSS3): the scripts of a game of VX Ace run
  over Ruby 1.9.2 in the guest executable `vkxp-guest-rgss3`, which `vkxp` starts from
  beside itself, always as a guest process. The start of a game is that of the original
  runtime: the built-in scripts file with `rgss_main` and `rgss_stop`, `$RGSS_SCRIPTS`,
  scripts evaluated as `{0000}` and compiled as UTF-8, `RGSSReset` with `Graphics.__reset__`
  and `Audio.__reset__`, the endings, the error report of RGSS3, `msgbox`, `msgbox_p`,
  `load_data`, `save_data` and `RGSS_VERSION`. The classes are those of RGSS2 still, so a
  game made with the standard scripts ends with an error before its title screen; a run
  says once that the support is experimental.
- Save states and the sandbox for a guest of VX Ace; a state is not saved while a script
  thread is alive.
- The release archive holds `vkxp-guest-rgss3` beside `vkxp` and the licence texts of
  Ruby 1.9.2.
- `cargo xtask stress` has the step `guest-1.9.2`.
- RPG Maker VX Ace (RGSS3): the module `Audio` of RGSS3. `bgm_play` and `bgs_play` take
  `pos`, `bgm_pos`, `bgs_pos` and `setup_midi` exist, and volume and pitch follow the limits
  of RGSS3. The same music played again goes on and only changes its volume. The position
  follows the rule measured on the original, computed by the game's side from the frames it
  has shown and the header of the file, so it is the same in every run with the same input;
  a music effect holds it. A sound is looked for as `ogg`, `mid`, `mp3`, `wav`, `wma`.
- Audio: a sound can start part of the way into its file, the music returns to its volume
  over one second after a music effect of VX Ace, the volume of a playing sound can change
  without a restart, and the mixer plays every pitch from 1 up.

### Changed

- The protocol is at version 15: the play command carries where its sound starts and the
  time over which the music returns after a music effect, and a volume command exists. An
  input recording or a stream of version 14 is refused.

- A game of VX Ace no longer ends with the message that VX Ace is planned. `--guest thread`
  with such a game ends the run with a message that names `--guest process`.

### Fixed

- RPG Maker VX Ace (RGSS3): text is measured and placed from glyphs hinted as Wine's
  FreeType hints them, so that its widths are those of the original runtime.

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
