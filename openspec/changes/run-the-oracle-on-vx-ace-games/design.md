## Context

The facts below were measured on the original of VX Ace on 2026-10-13 and are written up
outside the repository, readable from a clone as `../../ref/tools/oracle/measurements/`:
`ace-v0-REPORT.md` ("the V0 report", points 1 to 7 and "Differences from VX", point 11),
`ace-v1-sprite-REPORT.md` ("the sprite report", section 6, "Input") and
`ace-v1-api-REPORT.md` (1.4, F12). The scripts used there (`ace-v0-probe.rb`,
`ace-v0-rgss3a.py`, `ace-v0-env.sh`, `ace-v0-oracle.sh`) are the models of this change; only
rules, numbers and the description of the format are taken from them. In short:

- Start (V0 point 1): the `D3D8.DLL` stand-in, the Wine environment and the locale option
  are as for VX. Run-time package: key `HKLM\Software\Enterbrain\RGSS3\RTP`, 32-bit view,
  value named as `RTP=` of `Game.ini` names it (`RPGVXAce`); a game with `RTP=` empty needs
  none. The run-time package holds no DLL: every game ships its DLL in `System/`, named by
  `Library=` (`Library=System\RGSS300.dll`), and the name says nothing of its version.
- Window (V0 point 1, point 6): 544 by 416 at the start. A game may call
  `Graphics.resize_screen` at once (one game of the corpus sets 640 by 480), and the window
  then has the new size. `resize_screen` clamps each side to 1..640 and 1..480 and shows no
  frame.
- Scripts (V0 point 2): `Data/Scripts.rvdata2` has the structure of `Scripts.rxdata`
  (Marshal 4.8, an array of `[Fixnum, String, String]`; the editor's strings carry no
  encoding). With `Game.rgss3a` present the loose file is not used.
- `Game.rgss3a` (V0 point 2) is RGSSAD version 3:
  - bytes 0..7 `RGSSAD\0\3`; bytes 8..11 a seed, little endian; table key = seed * 9 + 3
    (32 bits);
  - then a table, one record per entry, every 32-bit little-endian word XORed with the table
    key: offset of the data from the start of the file, size, file key, length of the name;
    then the name, byte i XORed with byte i % 4 of the table key (little endian). Names are
    UTF-8 with backslashes. The key does not change from record to record;
  - the table ends with a record whose first word decrypts to 0; the editor writes 16 bytes
    there;
  - the data of the entries follow, in table order, each XORed in 4-byte little-endian words
    with a key chain that starts at its file key, next = key * 7 + 3 (the step of version 1);
    the bytes after the last whole word are XORed with the bytes of the key that would come
    next.
  Decrypting and packing again with the same seed, keys and order gave both archives of the
  corpus back byte for byte; an archive with the scripts entry replaced ran.
- Frames (V0 point 6): the rules of RGSS2, unchanged: `update` 1; `wait(n)`, `fadeout(n)`,
  `fadein(n)` n; `freeze` 0; `transition(n)` n after a freeze, 0 without one, default 10;
  `snap_to_bitmap`, `frame_reset`, `resize_screen` 0. The game assigns
  `Graphics.frame_count`, so a probe keeps its own count. `wait` and the fades run as loops
  of `update` show the same pictures (85729 pixels, 0 wrong). A late `update` does not draw;
  `frame_reset` before the update of a capture frame and after each hold makes it draw
  (V0 point 3). Frames inside a `transition` cannot be held.
- Input (sprite report 6): the constants are the Symbols of their names. `press?`,
  `trigger?` and `repeat?` are true for the Symbol of a held button and for its Integer of
  RGSS2 (2 4 6 8, 11..18, 21 22 23, 25..29), and false, without an error, for anything else.
  `trigger?`, `repeat?` (1st, 24th, 30th, 36th update), `dir4` and `dir8` follow the model of
  RGSS2 with 0 mismatches in 12084 updates. Real keys are seen only through calls of
  `Graphics` that show frames, which is why the V0 probe set the keys of frame f + 1 before
  the call that shows frame f.
- Repeatability (V0 point 5): `srand` in the probe makes runs equal; `hash` values are seeded
  per process; Ruby threads run (V0 point 8, unlike XP and VX under this Wine) but at times
  that are not repeatable.
- Fonts (V0 point 1): `Font.default_name` is `"VL Gothic"`, loaded from `<RTP>/Fonts`
  (which holds `VL-Gothic-Regular.ttf` and `VL-PGothic-Regular.ttf`, open fonts) and from
  `<game>/Fonts`.
- Rule incident (V0, "Rule incidents" 1): the first start of a VX Ace game under the VX
  recipe opened a GPU through VA-API, because the game plays Ogg through quartz,
  `winegstreamer` and GStreamer, whose hardware plugins probe the devices. The oracle has
  since run every Wine process in `bwrap` without `/dev/dri` and with VA-API variables that
  name no driver (change `run-the-original-without-device-nodes`). `ace-v0-env.sh` did one
  more thing: GStreamer got a plugin directory without its hardware plugins and a registry of
  its own.

Today (`xtask/src/oracle`): `runtime.rs` knows RGSS1 and RGSS2; `game.rs` puts the probe into
the loose script archive or into an archive of version 1 (`xtask/src/rgssad.rs`);
`original.rs` copies the DLL beside `Game.exe` under its own name, grabs the top-level window
of the runtime's fixed size, and builds every Wine command in `Run::wine`; `vkxp.rs` builds
`--package vkxp`; `probe.rb` and `probe_rgss2.rb` are the probes, each one text for both
sides.

## Goals / Non-Goals

**Goals:** `cargo xtask oracle` runs a game of VX Ace on the original's side with the
exactness of A0, and on the vkxp side through the guest executable of RGSS3, with the report
of VX. The original's side of every runtime runs without GStreamer's hardware plugins.

**Non-Goals:** comparing a real VX Ace game or making its frames equal (that is the work of
the class bindings of RGSS3); recording evidence of a milestone; real key presses, F12 or
the reset; `Graphics.play_movie`; capture through `snap_to_bitmap`; audio; anything in
`crates/`; a command that packs a game into `Game.rgss3a`.

## Decisions

1. **Runtime.** `Runtime::Rgss3`, from the extension `.rvdata2` of the file `Scripts=`
   names, or from `--runtime rgss3`. Its frame size at the start is 544 by 416, its registry
   key `Enterbrain\RGSS3\RTP` (the registry script names the 32-bit view as today), its
   archive `Game.rgss3a`, its probe `probe_rgss3.rb`. The message for an unknown extension
   names the three values of `--runtime`.

2. **Archive of version 3: a module of its own.** `xtask/src/rgss3a.rs`, beside
   `rgssad.rs`, reads and writes the format above. It is a module of its own because an
   entry of version 3 has a key that must be kept and the archive a seed and an end record,
   which version 1 has not; it reuses the word cipher and `names_match` of `rgssad.rs`
   (made visible to the crate, unchanged). The archive is read into: the seed; the entries
   in table order, each with its name bytes, file key and decrypted data; and the end
   record, the bytes from the record whose first word decrypts to 0 up to the first byte of
   data (up to the end of the file when there is no entry). The writer writes the header,
   the seed, the table with offsets that lay the data out in table order right after the end
   record, the end record as kept, and the data. So an archive laid out as the editor lays it
   out is written back byte for byte, and only the entry whose data changed differs. A new
   archive (the tests') gets an end record of four words that decrypt to 0. The reader
   refuses a header other than `RGSSAD\0\3`, a table that runs past the end of the file or
   has no end record, and an entry whose data lie outside the file. The independent reader
   in `vkxp-vfs` and this one check each other through the tests that run a packed game in
   vkxp, as for version 1.

3. **The probe of RGSS3 is a third text**, `probe_rgss3.rb`, so that the probes of RGSS1 and
   RGSS2 stay as they are, byte for byte. It is the probe of RGSS2 with three differences:
   - *Buttons.* Its model keeps the held buttons by Symbol. An argument of `press?`,
     `trigger?` or `repeat?` that is one of the 20 Symbols, or the Integer of RGSS2 of one
     of them, means that button; anything else gives false. The model is that of RGSS2
     otherwise (trigger, the one repeating button chosen in ascending order of the numbers of
     RGSS2, 1st, 24th, 30th, 36th update, `dir4` by the axis flag, `dir8`), as the sprite
     report found it. The probe does not read `Input`'s constants: in vkxp they are the
     Integers of RGSS2 until `bind-input-of-rgss3` is merged, and the mapping answers both.
   - *Size.* On the original's side the request to hold a frame holds the size of the
     screen, `Graphics.width` by `Graphics.height`, at the time of the hold (decision 5).
   - *Syntax.* It is written for Ruby 1.9.2, which both sides run; its text is ASCII only.
   Kept from RGSS2: the seed, the frame numbers, the loops of `wait`, `fadeout` and `fadein`
   and the `frame_reset` around holds on the original's side, the late mark of a frame
   inside a transition, `exit` after the last capture, one text for both sides that differs
   only in the value that names the side.

4. **"Set the keys one `Graphics` call early" does not apply.** That rule of the V0 report
   is about real key presses, which the original sees only through calls of `Graphics` that
   show frames. The harness's probes send no key: on both sides they replace `Input` by the
   model, which answers from the schedule entry for the probe's own frame number when the
   game calls `Input.update`. The meaning of a schedule stays that of VX: the buttons of the
   entry `F:` are what the scripts see while they work on frame F. `SceneManager.scene`,
   which replaces `$scene` in RGSS3, does not concern the harness's probes either: they read
   no scene (only the V0 probe logged it).

5. **The size of a captured frame comes from the request.** The probe of RGSS3 writes the
   request of a held frame as `<frame>`, then ` late` when it is late, then ` <w>x<h>`. The
   harness reads the size from the word of the form `<digits>x<digits>`; a request without
   one (the probes of RGSS1 and RGSS2, unchanged) gives the runtime's frame size. The window
   grabbed is the topmost viewable top-level window of that size, as today; the choice among
   the windows is a function of their sizes and states, tested without a display. A size the
   probe reports and no window has is the error of today, naming the size. The vkxp side
   dumps the picture at the size vkxp has; pictures of different sizes are already reported
   as not comparable.

6. **The DLL of a game of RGSS3 goes where `Library=` says.** For RGSS3 the harness reads
   `Library=` of the copy's `Game.ini` and puts the DLL at that path below the copy: parts
   separated by `\` or `/`, each matched without regard to case against what exists and
   created when missing; a path with a drive, a leading separator, `.` or `..` is refused.
   With `--dll` that file replaces whatever the game has there; without `--dll` the file the
   game ships at that path is used, and a missing file is an error that names the path and
   `--dll`. Both are checked before any Wine process starts. Nothing is copied beside
   `Game.exe`. A missing `Library=` is an error naming it (what the original does without
   the line was not measured). For RGSS1 and RGSS2 nothing changes: `--dll` is required and
   copied beside `Game.exe` under its own name.

7. **GStreamer without its hardware plugins, for every runtime.** Before the original starts,
   the harness makes `<out>/gstreamer/plugins-<n>` for each existing directory of
   `/usr/lib64/gstreamer-1.0`, `/usr/lib/gstreamer-1.0`,
   `/usr/lib/x86_64-linux-gnu/gstreamer-1.0` and `/usr/lib/i386-linux-gnu/gstreamer-1.0`
   (made afresh on every run), with a symbolic link to every file `*.so` of it except
   `libgstva.so`, `libgstvaapi*.so`, `libgstqsv.so`, `libgstmsdk.so`, `libgstvulkan.so`,
   `libgstkms.so`, `libgstopengl.so`, `libgstqmlgl.so`, `libgstnvcodec.so`,
   `libgstvideo4linux2.so` and `libgstv4l2codecs.so` (the list of `ace-v0-env.sh`). Every
   Wine command gets `GST_PLUGIN_SYSTEM_PATH_1_0` naming those directories (an empty
   directory below `<out>/gstreamer` when none exists), `GST_REGISTRY_1_0` naming
   `<out>/gstreamer/registry.bin`, and none of `GST_PLUGIN_PATH`, `GST_PLUGIN_PATH_1_0` and
   `GST_PLUGIN_SYSTEM_PATH`. It is a third line behind the sandbox and the VA-API variables;
   the sandbox stays the requirement. The registry of its own keeps the user's registry from
   being rewritten with the filtered view. Applying it to XP and VX changes no picture (their
   games use MIDI, and audio is not compared).

8. **The vkxp side builds the guest.** For a game of RGSS3, `vkxp::build` builds
   `--package vkxp --package vkxp-guest-rgss3` in one cargo call, so that `vkxp` finds the
   guest beside itself as `run-rgss3-games-in-a-guest-of-their-own` decides, and fails with
   a message naming `cargo build -p vkxp-guest-rgss3` when the file is not there afterwards.
   Other runtimes build `vkxp` alone, as today.

9. **Fonts: no new machinery.** VL Gothic, the default font of RGSS3, is an open font that
   the run-time package ships in `Fonts`. `--font <RTP>/Fonts/VL-Gothic-Regular.ttf` gives
   both sides the very file the game names: the prefix gets it installed and registered, and
   vkxp gets it in its font directory with `--font-own-metrics`. The list of replaced names
   does not gain the VL names, because whether a registry substitute applies to a font the
   runtime loads from `<RTP>/Fonts` itself was not measured. Glyphs stay outside the
   pixel-exact standard, as for XP and VX.

10. **Threads, locale.** `--no-threads` is not needed for the original of RGSS3 to run, and
    it stays valid with the third probe: script threads run there at times that are not
    repeatable, and the option makes such a game repeatable at the price it has today. The
    default locale stays `zh_CN.UTF-8`; the document names `en_US.UTF-8`, which A0 used for
    the three games of the corpus.

11. **Wine is only started by the harness.** As in the change of VX: no step of this change
    runs `wine`, `wineboot`, `wineserver` or `regedit` outside `Run::wine`, in tests or by
    hand. The flow runs no Wine at all.

## Implementation notes

- `game.rs`: `prepare` chooses the archive by runtime; for `Game.rgss3a` it reads with
  `rgss3a`, replaces the data of the entry `Scripts=` names (by `names_match`) with the
  probed script archive, and writes back. The probed script archive is made by
  `archive::with_first` as today. Its strings are plain, as the editor writes them;
  `load_data` of RGSS3 labels such strings UTF-8 (V0 point 8).
- `mod.rs`: the runtime is decided before the prerequisites of the original are checked,
  since the DLL check depends on it (decision 6). The usage text names `rgss3` and says that
  `--dll` may be left out for a game of RGSS3.
- Tests of `rgss3a.rs`, on archives the tests generate: round trip; byte-for-byte rewrite of
  an archive with 16 bytes of end record of which the last 12 are arbitrary; the first
  twelve bytes and the first decrypted record of a written archive checked against the
  format above, computed in the test from the seed (not with the reader); the refusals.
  Seeds and keys may be any values; the ones seen in the corpus (seeds 0x2a38 and 0x350a,
  first keys 41, 18467, 6334) are fine as inputs.
- Generated games of RGSS3 in `mod.rs` tests: `Game.ini` with
  `Library=System\RGSS300.dll`, `Scripts=Data\Scripts.rvdata2` and `RTP=`, the scripts
  written by `archive::write`, and a stand-in built-in scripts file that is the tests' own
  text (a `module RPG` and an `rgss_main` that yields), never the original's. The tests that
  run such a game in vkxp exist only where the tests of `crates/vkxp/tests/rgss3.rs` run
  (the same `cfg`), since the guest of RGSS3 is built only there.
- The scenarios on the probe of RGSS3 are tested as those of RGSS2: the probe of the vkxp
  side in a generated game of RGSS3 that writes what the model answered into a file; the
  probe of the original's side in vkxp, with the test answering the requests in place of
  the harness and recording their text.
- The scenarios on DLL placement, the registry script, the GStreamer directories and the
  command environment are tested on prepared copies and built commands, without Wine; the
  plugin directories from a temporary directory that stands in for the system's (the
  function takes the candidate directories as an argument).
- The ignored test of the original side stays as it is.
- Made in the flow outside the files the change names: `xtask/src/main.rs` declares the new
  module (`mod rgss3a;`), one line without which `rgss3a.rs` is not compiled.
- The tests that run a game of RGSS3 in vkxp give it the test's own directory as
  `XDG_RUNTIME_DIR`, where vkxp makes the socket of the guest process, as the tests of `vkxp`
  do; those that go through the harness run its vkxp side (the copy with the probe and the
  command of `vkxp::Run`) for that reason, rather than the whole of `run`. The harness itself
  leaves the variable as it is.
- `docs/development.md`, section "Comparing with the original runtime": a paragraph on VX
  Ace (runtime, key, archive, DLL and `--dll`, screen size, fonts with VL Gothic, locale,
  threads), the GStreamer directories next to the sandbox, the limit on window size
  rewritten (fixed for XP and VX, from the probe for VX Ace), and the usage table.
- After the merge, not a task of the flow: the owner's agent runs `cargo xtask oracle
  ../../games/ace-crysalis --original-only --rtp RPGVXAce=../../ref/ace/rtp --locale
  en_US.UTF-8` twice with the schedule of `ace-v0-crysalis-input.txt` (its comment lines
  removed; the schedule syntax has no comments) and frames of that run, then the two
  encrypted games (`ace-in-search-of-immortality`, 640 by 480; `ace-capellas-promise`),
  checks that repeated runs are byte-identical, compares with `ace-v0-crysalis-sha256.txt`
  where seed and method match (`run-a`: seed 20261009, native waits; the harness runs waits
  as loops, which differ only inside them), and looks at `/proc/<pid>/maps` and `wine.log`
  for GStreamer hardware plugins and VA-API drivers. The result is written under a
  "Measured" heading of this design before the change is archived.

## Follow-ups

- Real key presses through XTEST (worked in `ace-v1-api-REPORT.md`, 1.4) for F12 and the
  reset, if a milestone needs them; then the rule of keys set one call early applies.
- `Graphics.play_movie`: frames and capture not measured.
- Capture through `snap_to_bitmap` as a second check of the grab (V0 point 3: equal except
  at brightness 0).
- A check of the mapped libraries of the Wine processes after a run, as `ace-v0-env.sh`'s
  `ace_maps` did.
- Comment lines in schedule files.
- Whether a registry substitute applies to a font the runtime of RGSS3 loads itself.
- Mask transitions of RGSS3 were recorded, not fitted (as VX).
