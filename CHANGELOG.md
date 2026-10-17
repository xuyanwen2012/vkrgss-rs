# Changelog

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Security

- A confined guest process also filters its system calls with seccomp, on Linux and
  Android, after its Landlock rules: starting programs, tracing other processes, sockets of
  every family but `AF_UNIX`, mounts, namespaces, kernel modules and keys, `bpf`,
  `perf_event_open` and `io_uring` get `EPERM`. On Android, whose kernels have no Landlock,
  the filter is the confinement besides the app's private directory. `--sandbox off` and
  `--allow-process-functions` leave it out, as they leave out Landlock.

- Both embedded interpreters are built with `_FORTIFY_SOURCE=3`, `-fstack-protector-strong`,
  `-fstack-clash-protection` and, on x86_64, `-fcf-protection=full`, asked for by their
  build scripts with gcc and clang alike instead of left to the defaults of Ubuntu's gcc.
- Ruby 1.9.2 of RPG Maker VX Ace games carries the fixes of CVE-2014-4975 (`pack('m')`),
  CVE-2018-8778 (`unpack` counts), CVE-2017-0898 (`sprintf` of infinity and NaN with the
  largest width), CVE-2014-3916 (`String` capacity) and CVE-2026-27820
  (`Zlib::GzipReader#ungetc`). Its regular expression engine carries the fixes of
  CVE-2017-9224, CVE-2017-9226, CVE-2017-9227, CVE-2017-9229 and CVE-2019-19246, of the
  reads past the end of a pattern that upstream Oniguruma fixed, and of nested repeats such
  as `(?:(?:(?:a{0,100000}){0,100000}){0,100000})bc`, which ended the process when compiled.
  `docs/vendored-security.md` reviews the security fixes published after that release and
  says what was done about each.
- The fuzz targets and the sanitized run of Ruby 1.9.2 found defects that vkrgss now fixes:
  a dumped Time with a month above 12, which a save file can hold, read past a table; a
  regexp literal with text after an interpolation, as `/#$y #{"s"}#{"t"}/`, ended the
  process; `class ::c` could end the process at the next collection; a backslash before a
  character cut off by the end of a pattern was read past the pattern; groups that refer to
  each other, as `()(\4)()(\2)`, recursed without end when compiled; and `"%a" % 0.0` wrote
  one byte past its buffer.
- The regular expression engines of both interpreters check the least length of a match
  in backward searches (`String#rindex`), as upstream 33e5b47c does for CVE-2024-27282:
  nested repeats whose least length is too large to count formed a position past the
  subject.
- The fuzz targets of the parsers found more defects that vkrgss now fixes, each after a
  syntax error in a script: in Ruby 1.8.7 a number such as `1_.` was read past the end of
  the lexer's buffer, and a script with `__END__` after an open `module` could copy its
  last line from freed memory; in Ruby 1.9.2 `(A::B += p 1)..1`, and `=~` after a word list
  that is no valid UTF-8, ended the process.

### Added

- aarch64 Linux is supported (milestone P1): on the owner's Jetson Orin Nano the compared
  frames and saves of the XP, VX and VX Ace games equal those of x86_64 byte for byte, and the
  CI job `gate-aarch64` must pass.

- The workflow `nightly.yml` runs the dependency audit, the collector stress, the sanitized
  suites of Ruby 1.8.7 and five minutes of each fuzz target once a day and by hand.
- Ruby 1.9.2 has a sanitized run of its suites
  (`cargo test -p vkrgss-ruby192-sys --test sanitized -- --ignored`) and the fuzz targets
  `marshal192`, `parse192` and `regex192` of `cargo xtask fuzz`; `nightly.yml` runs both.
- `cargo xtask gate --job <name>` runs one part of the merge gate: `lint`, `ruby`,
  `ruby-stress`, `vulkan`, `vulkan-unvalidated` or `rest`. Together the parts run exactly
  what `cargo xtask gate` runs, which is unchanged.
- When a job of `nightly.yml` fails, the workflow opens the issue "Nightly checks failed",
  or comments on it while it is open.
- A failed fuzz job of `nightly.yml` uploads the inputs its target put aside, for 30 days.
- Both interpreters and the guest executables build for Android (`aarch64-linux-android`) at
  API level 34 or later, the minimum the owner decided on 2026-10-17; a lower level stops the
  build with a message naming 34. `cargo xtask android-test <crate>` runs the tests of an
  interpreter crate on a phone over adb; they pass on the Pixel 7a and the Galaxy S24+. Not
  supported before the milestone P3 is accepted. There Ruby 1.8.7 reads the end of a file
  and of a pipe through `__freadahead`, and `String#crypt` raises `NotImplementedError`.

### Changed

- `nightly.yml` installs `cargo-deny` 0.20.2, which reads the current advisory database;
  0.18.2 could no longer parse it.
- `cargo xtask fuzz` sets aside an input that exhausts the machine stack by nesting, as it
  does an input that runs into the time limit: a stack overflow that AddressSanitizer
  reports in a function that recurses on the nesting of the target's input is counted, and
  its input kept as `stack-<hash>`; any other stack overflow is still a finding.
- Ruby 1.8.7 is built on aarch64 with configuration headers of its own instead of those of
  x86_64, and names the platform `aarch64-linux` there. Ruby 1.9.2, and with it the games
  of RPG Maker VX Ace, is built on aarch64 too. The headers are those that the releases'
  `configure` wrote on aarch64; aarch64 is not supported before the milestone P1 is
  accepted.
- The tests of `vkrgss-vk` run the validation layer with its synchronization validation on,
  through `VK_EXT_layer_settings` where the layer offers it, so a missing or wrong barrier
  fails a test; a test records a hazard on purpose to show that it does. A recorded
  validation message starts with its id. The tests report no hazard.
- The Vulkan backend submits with `vkQueueSubmit2` and waits for its submissions on a
  timeline semaphore instead of a fence that is reset after each wait. Bitmap uploads record
  the barriers of all their images in one dependency before the copies and one after, and an
  image that an upload replaces whole starts from `UNDEFINED`. No composed pixel changes.
- The guest process of a game of RPG Maker XP or VX is the new executable
  `vkrgss-guest-rgss12`, which `vkrgss` starts from beside itself as it starts
  `vkrgss-guest-rgss3` for VX Ace; `vkrgss` no longer starts itself as a guest. The new
  executable links Ruby 1.8.7 and nothing of the host, so the process that a save state
  copies holds no window framework. A run with the guest in a process, the default in a
  window, ends with a message that names the file and the directory when it is missing;
  `--guest thread` needs it not. The release archive holds the three executables, each with
  libcrypt linked statically, and `cargo build --release -p vkrgss -p vkrgss-guest-rgss12 -p
  vkrgss-guest-rgss3` builds them.
- The workflow `ci.yml` runs each part of the gate as a job of its own, in parallel, and
  caches the objects of Rust and C with sccache, so that the interpreters are compiled
  again only when their sources change. The job `gate-aarch64` still runs the whole gate.
- `Math` and the power of floats (`**`) of both interpreters are correctly rounded: they
  come from CORE-MATH, vendored in `third_party/core-math`, instead of the C library, so a
  float that a script computes, and a save that holds it, is the same on every machine,
  version of glibc and operating system. A result differs from the one before only in the
  last bit, where the C library was not correctly rounded; special arguments and errors are
  as before. Both interpreters are compiled with `-ffp-contract=off`, so that no compiler
  fuses a multiplication and an addition on one architecture and not on another.
- The workflows name actions by commit hash, install the OpenSpec CLI at version 1.14.2, and
  build with `CC=gcc-13`, the compiler of the release, named explicitly. A push to `main`
  no longer cancels the run of the push before it.
- The test harness of the binding has no default runtime: `testing::start_as` returns the
  handle through which alone a test reaches the interpreter, so every test declares its
  runtime and the order of the tests no longer decides it. Each test process generates its
  files in a directory of its own.

### Fixed

- `Math` of Ruby 1.8.7 raises the same errors on every platform and C library, those of
  glibc 2.43: the functions of CORE-MATH set `errno` themselves, so that, for one,
  `Math.atanh` of a subnormal float raises `Errno::ERANGE` on Android and with glibc 2.42 or
  older as well. Where CORE-MATH overflows its correctly rounded infinity is returned, not
  the value of the C library, so `Math.hypot` at the edge of overflow is the same
  everywhere.
- Ruby 1.9.2 of RPG Maker VX Ace games keeps values that only a pointer into them holds
  while it allocates, which the stressed suite on aarch64 showed it did not: the sum of a
  Bignum of two digits and a small integer could read the digits of a collected object, so
  that `(2**63 - 1) + 1` gave 1, every `Time` after 1970 was "out of system range" for
  `File.utime` and local times at changes of daylight saving time came out wrong; and
  `Marshal.dump` of a `Time` with a fraction raised "can't dump anonymous ID 0". An audit of
  `bignum.c`, `marshal.c`, `time.c`, `string.c`, `pack.c` and `sprintf.c` added the same
  guard to Bignum division, multiplication and bitwise operations, `String#sub!`,
  `#each_codepoint`, `#tr`, `#delete`, `#squeeze`, `#count`, signed Bignum conversions of
  `format` and the BER-compressed integers of `Array#pack('w')`.
- Present timing reads the monotonic clock from the system, and any other clock of the
  swapchain only when the device can calibrate it; a swapchain without such a clock is paced
  as without present timing. A time of 0 makes the presenter choose the clock again.
- The validation messenger no longer asks for performance messages, which it never received
  at error severity only.
- `docs/decisions.md` says how fonts are found (`fontdb` and its directory scan) and that
  Linux is the only platform supported now.
- A file written into a directory that was looked into within the same tick of the file
  system's clock is found: on kernels before 6.13, whose directory times advance in steps of
  a few milliseconds, a script that wrote a file and read it back at once could be told it
  did not exist. A directory changed in the last two seconds is listed again at each lookup.

## [0.3.1] - 2026-10-14

### Security

- Ruby 1.9.2 of RPG Maker VX Ace games carries the fix of CVE-2013-4164: a very long digit
  string, such as a Float dumped in a save file, overflowed the heap when it was converted.
  The tree of Ruby 1.8.7 has had the fix since 0.1.0.

### Added

- RPG Maker VX Ace (RGSS3): the endings and loading of the original runtime. `$0` is
  `false` and a script's backtrace ends with ``ruby:in `eval'``; a game without a title is
  `Untitled`; an archive that cannot be loaded shows `Failed to load script.`; `at_exit`
  never runs its block; `exit!` and `Process.exit!` end the game at once. `load_data` asks
  the archive first, also by the name without its extension, then a loose file from the
  current directory, which it reads as a `File`, labels frozen strings too, raises
  `Errno::EACCES` for a directory and checks the name as 3.0.1 does, as `save_data` does.
  vkrgss's own rules: `require` and `load` compile a file as UTF-8, `require_relative` and
  `autoload` follow the rule of `require`, a long report is shown whole, and every uncaught
  error ends with status 1.

### Changed

- The project is renamed vkrgss; versions 0.1.0 to 0.3.0 were released as vkxp. The
  executables are `vkrgss` and `vkrgss-guest-rgss3`, with no executable or link named
  `vkxp`; the package is `vkrgss-<version>-x86_64-unknown-linux-gnu.tar.gz`; the crates are
  `vkrgss` and `vkrgss-*`; the repository is `https://github.com/xuyanwen2012/vkrgss-rs`.
  The user file is `$XDG_CONFIG_HOME/vkrgss/config.toml`, the game's file `vkrgss.toml`, the
  default built-in scripts files are under `$XDG_DATA_HOME/vkrgss/`, and the environment
  variables are named `VKRGSS_*`. Diagnostics start with `vkrgss: `, `--version` prints
  `vkrgss <version>`, a new recording starts with `vkrgss-input 2`, and in a game of XP or
  VX `$0` is `"vkrgss"`. The modules that a filter of `VKRGSS_LOG` names follow the crates:
  `vkxp_host=trace` is now `vkrgss_host=trace`.
- For this version only, a file at its place of 0.3.0 (`vkxp/config.toml`, `vkxp.toml`,
  `vkxp/rpg_builtin*.rb`) is read when the file is not at its new place, and `VKXP_LOG`,
  `VKXP_GUEST`, `VKXP_SANDBOX` and `VKXP_PRESENT_TIMING` are read when the new variable is
  not set, each with a warning that names both; the following release reads neither.
  Recordings whose first line is `vkxp-input 1` or `vkxp-input 2` replay as before, without a
  warning and without an end. docs/running.md, section "Coming from vkxp 0.3.0", says what to
  move.

## [0.3.0] - 2026-10-14

### Added

- Support for RPG Maker VX Ace (RGSS3): the scripts of a game of VX Ace run over Ruby 1.9.2
  in the guest executable `vkxp-guest-rgss3`, which `vkxp` starts from beside itself, always
  as a guest process. The start of a game is that of the original runtime: the built-in
  scripts file with `rgss_main` and `rgss_stop`, `$RGSS_SCRIPTS`, scripts evaluated as
  `{0000}` and compiled as UTF-8, `RGSSReset` with `Graphics.__reset__` and
  `Audio.__reset__`, the endings, the error report of RGSS3, `msgbox`, `msgbox_p`,
  `load_data`, `save_data` and `RGSS_VERSION`. Two games of VX Ace equal the original
  runtime on every compared frame of their title, menus, maps and a battle but for three
  antialiased pixels of one glyph, and a save written at the same frame is byte for byte
  that of the original.
- RPG Maker VX Ace (RGSS3): the class `Tilemap` with `flags` in the place of `passages`,
  drawn by the rules of RGSS3. A write to a slot of `bitmaps` of a disposed tilemap is kept
  where the original runtime ends.
- RPG Maker VX Ace (RGSS3): `Font` and the text and creation of `Bitmap` of RGSS3: the
  defaults of RGSS3, `outline` and `out_color`, the objects a font is given kept as given,
  `Font.exist?` exact, `font=` that copies every attribute, text drawn by the rules of RGSS3,
  no limit of 4194304 pixels on `Bitmap.new`, and the argument errors of the original. The
  guest reads the version of the DLL the game ships, by which `RGSS_VERSION` is `"3.0.0"` or
  `"3.0.1"` and the glyph rule is that of 3.0.0.1 or 3.0.1.1, and a game takes fonts from the
  `Fonts` directories of its run-time packages.
- RPG Maker VX Ace (RGSS3): the module `Audio` of RGSS3. `bgm_play` and `bgs_play` take
  `pos`, `bgm_pos`, `bgs_pos` and `setup_midi` exist, and volume and pitch follow the limits
  of RGSS3. The same music played again goes on and only changes its volume. The position
  follows the rule measured on the original, computed by the game's side from the frames it
  has shown and the header of the file, so it is the same in every run with the same input;
  a music effect holds it. A sound is looked for as `ogg`, `mid`, `mp3`, `wav`, `wma`.
- RPG Maker VX Ace (RGSS3): `Win32API` of RGSS3. The constructor takes four arguments and
  finds a library that is a file of the game, the runtime library of `Game.ini` or a system
  library of Windows of a fixed list, and raises `LoadLibrary` for any other; the import
  declares the parameters, whose count, conversions and errors are those of the original;
  the export decides the value; a filled buffer keeps its encoding label; the profile
  functions read nothing under a bare file name. No library is loaded: functions are
  emulated as for XP and VX or give 0. `DL` exists with its constants and classes and
  refuses every way into a library or into memory with `DL::DLError`.
- Save states and the sandbox for a guest of VX Ace; a state is not saved while a script
  thread is alive.
- The release archive holds `vkxp-guest-rgss3` beside `vkxp` and the licence texts of
  Ruby 1.9.2.
- `cargo xtask stress` has the step `guest-1.9.2`.
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
