## Context

What exists (read for this change):

- `Runtime` (`crates/vkxp-vfs/src/runtime.rs`) has `Rgss3`. `Runtime::detect` already gives
  it for a `Library` whose file name is `RGSS3`, two digits, an optional letter and `.dll`
  (in a folder or not) and for a `Scripts` that ends in `.rvdata2`; the main specification
  has the scenario. `GameIni::rtp_names` already gives the one value `RTP` for RGSS3.
  `Runtime::archive_name` is `None` for RGSS3, so nothing of an archive is read for it.
- `archive.rs` of `vkxp-vfs` reads RGSSAD version 1 only: it walks the entries, which are a
  chain, keeps for each the offset, the size and the key of its data, and decrypts an entry
  when it is read. `GameFiles::open` looks for the archive of the runtime under any letter
  case. `VfsError::DamagedArchive` names the path and a reason.
- The program (`crates/vkxp/src/main.rs`): `runtime_of` refuses RGSS3 as soon as the runtime
  is known ("this is a game of RPG Maker VX Ace (rgss3), which is planned and not supported
  yet", status 1), so `guest_inputs` never resolves a package or looks for a built-in
  scripts file for it. `builtin_scripts_name` already answers `rpg_builtin_rgss3.rb`, which
  nothing reaches. `run` (the program) and `run_guest` (a guest process) both start with
  `guest_inputs`; `guest_config` opens the game's files. The usage text names the default
  files of RGSS1 and RGSS2.
- The change `move-the-guest-role-into-a-library`, open beside this one, moves
  `guest_inputs`, `guest_config`, `start_interpreter` and their helpers into one module of
  the program. The functions keep their names; this document names functions, not files.
- The program has no reader of the script archive in Rust: the guest reads the file that
  `Scripts=` names with `Marshal` of its interpreter (`vkxp_load_scripts` in the prelude of
  the binding). The only Rust reader is the oracle's (`xtask/src/oracle/archive.rs`).
- `cargo xtask extract-rpg <DLL> <OUTPUT>` (`xtask/src/extract.rs`) reads any file and writes
  every NUL-terminated string of text (printable ASCII, tab, line breaks) that starts with
  `module RPG;`, in file order, each followed by a line break. For RGSS2 the input it is
  given today is not the DLL file, which is packed, but the image of the DLL as it lies in
  the memory of the running original, written to a file by the user (`docs/running.md`,
  "The built-in scripts of RPG Maker VX").
- `vkxp-vfs` has no rule about fonts. The program's text backend takes the directories of
  `--font-dir`, then `<game>/Fonts`, then the system's fonts, for every runtime.

Measured on the original runtime of VX Ace on 2026-10-13 (outside the repository; from a
clone under `wt/<change>/` the report is `../../ref/tools/oracle/measurements/ace-v0-REPORT.md`,
sections "Materials", 1, 2, 7 and 8, with `ace-v0-rgss3a.py` and `ace-v0-extract.py` beside
it; they are read, and only numbers, names and descriptions of formats are taken from them):

- `Game.ini` of a game of VX Ace: `Library=System\RGSS300.dll` or `System\RGSS301.dll`,
  `Scripts=Data\Scripts.rvdata2`, one key `RTP=` (value `RPGVXAce`, or empty: one game of
  the corpus names no package and starts without one). The name of the DLL does not tell
  its version: one game ships the file of version 3.0.1.1 under both names. The run-time
  package holds no DLL; every game ships its own in `System/`.
- A game that names a package that is not installed does not start on the original
  ("RPGVXAce RTP is required to run this game.").
- `Game.rgss3a` is RGSSAD version 3, measured on two archives (345 and 1474 entries):
  - bytes 0 to 7: `RGSSAD`, 0, 3;
  - bytes 8 to 11: a seed, little endian; the table key is seed * 9 + 3 in 32 bits (seeds
    0x2A38 and 0x350A, keys 0x17BFB and 0x1DD5D);
  - then the table, one record per entry: four 32-bit words, each XORed with the table key
    (offset of the data from the start of the file, size, file key, length of the name),
    then the name, byte i XORed with byte i mod 4 of the table key, little endian. Names
    are UTF-8 with backslashes. The table key is the same for every record;
  - the table ends with a record whose first word decrypts to 0. The editor writes twelve
    more bytes there, three words that decrypt to nothing meaningful (0x17CD3, 0x12A2A,
    0x16B22 in one archive);
  - the data of the entries follow the table in table order, each XORed in 4-byte
    little-endian words with a key that starts at the entry's file key and becomes
    key * 7 + 3 after every word: the step of version 1. The file keys of the first three
    entries are 41, 18467 and 6334 in both archives.
  Decrypting both archives and packing them again with the same keys gave the input back
  byte for byte; an archive packed from extracted files with another seed and other file
  keys started on the original.
- With `Game.rgss3a` present the original does not use a loose `Data/Scripts.rvdata2`.
- `Data/Scripts.rvdata2` has the structure of `Scripts.rxdata` and `Scripts.rvdata`:
  `Marshal` 4.8, an array of `[Fixnum, String, String]` with the deflated source third; the
  strings the editor writes carry no encoding.
- The built-in scripts of RGSS3 are three NUL-terminated strings in the image of the DLL in
  memory, the same in both DLL versions, in this order: one that starts with
  `module RPG;end;` (the data classes), one that starts with `def rgss_main`, one that
  starts with `def rgss_stop`. In both images each of `module RPG;`, `def rgss_main` and
  `def rgss_stop` occurs exactly once. The strings are 20623, 144 and 45 bytes long (the
  header lines `ace-v0-extract.py` wrote; the report's 20661, 180 and 80 count the comment
  line that the script puts before each). No DLL file holds them as text. Between them lie
  other strings that the runtime evaluates around the game's scripts; they are not
  built-in scripts.
- Fonts: the original loads fonts from `<RTP>/Fonts` and from `<game>/Fonts` and finds them
  by family name; `Font.default_name` is `"VL Gothic"`, which the run-time package ships.

## Goals / Non-Goals

**Goals:**

- Everything the program does for a game before a guest starts works for a game of RGSS3
  and is tested on generated games, so that the changes that add the guest of RGSS3 only
  replace the refusal.
- `Game.rgss3a` is read with the layout measured on the original.
- A user can make `rpg_builtin_rgss3.rb` from an image of their own DLL with the command
  that exists.
- XP and VX behave as before in every case.

**Non-Goals:**

- Ruby 1.9.2, the executable `vkxp-guest-rgss3`, any RGSS3 class, running a script: a game
  of RGSS3 does not run after this change, and the documents keep saying that VX Ace is
  planned.
- Reading `Scripts.rvdata2` beyond its bytes. It is `Marshal` 4.8 of the same structure as
  the script archives of XP and VX, and the program has never decoded a script archive in
  Rust: the guest does, with the `Marshal` of its interpreter, and for RGSS3 that is the
  guest of Ruby 1.9.2. This change reads the file to see that it is there and does not look
  into it.
- The oracle on a game of VX Ace (`cargo xtask oracle`, also with `--original-only`). See
  "The oracle is a change of its own".
- Writing a `Game.rgss3a` outside test code (`cargo xtask pack-rgssad` stays as it is).
- Loading fonts from `<RTP>/Fonts` and the default font of RGSS3: `vkxp-vfs` has no font
  rule to extend, and a text backend is only built for a guest. See "Follow-ups".
- A tool that writes the memory image of a DLL for the user.
- Refusing `--guest thread` for RGSS3 and the default form of its guest (`plan-vx-ace`,
  decision 3): they come with the guest executable.
- Any change to detection: the rules of `Library=` and `Scripts=` stay as they are.

## Decisions

### Detection stays; the DLL's name gives the runtime and nothing else

`Library=System\RGSS300.dll` and `Library=System\RGSS301.dll` both give RGSS3 by the rule
that exists. Nothing is derived from the two digits after the first: the file's name does
not tell its version. Tests are added for the values of the corpus; no code of detection
changes.

### The archive of RGSS3 is read by a second table reader over the same entries

- `Runtime::archive_name` answers `Game.rgss3a` for RGSS3. The format follows the runtime:
  version 1 for RGSS1 and RGSS2, version 3 for RGSS3. The version byte of a file does not
  choose the reader: a `Game.rgss3a` with the header of version 1, and a `Game.rgssad` with
  the header of version 3 (the test `wrong_header` that exists), are damaged archives.
  Whether the original of RGSS3 would read a version 1 archive under that name was not
  measured.
- `archive.rs` gets a second function that reads the table of version 3 into the index
  that exists (lower-case name with `/` to offset, size and data key). Finding an entry
  and reading it are shared: the data of both versions is decrypted by the same word-wise
  XOR with the step key * 7 + 3, starting at the key the index holds.
- Reading the table of version 3:
  1. The first eight bytes are `RGSSAD`, 0, 3 and four bytes of seed follow; otherwise the
     archive is damaged. Table key = seed * 9 + 3, wrapping.
  2. Records are read until one whose first word, XORed with the table key, is 0. Nothing
     after that word is read as table: the editor's twelve bytes are neither needed nor
     checked, and an archive that has the data right behind that word is read the same.
  3. A file that ends inside a record or before the ending word is damaged. A name longer
     than the bytes left is damaged, before anything is allocated for it. An entry whose
     offset plus size is larger than the length of the file is damaged, as version 1
     refuses an entry that declares more bytes than are left.
  4. The data is found by its offset; that the data lie in table order is not assumed.
  5. As in version 1: a name that is not UTF-8 is left out, and of two entries whose names
     are the same once normalised the first is kept.
- Precedence is the rule that exists for every runtime: an entry before a loose file, a
  loose file before the run-time packages. (The original ignores a loose script archive
  when the archive is present; vkxp finds a loose file only when the archive has no entry
  of that name, for XP and VX as well. Not changed here.)
- `VfsError::DamagedArchive` keeps its form; the reasons of version 3 name version 3.

### The run-time package

No code changes: the name comes from `RTP`, and `--rtp RPGVXAce=DIR` or `RPGVXAce = "DIR"`
under `[rtp]` of a configuration file is a name like any other. An empty `RTP` needs no
package. A named package that is not in the table is the error that exists, which matches
the original, where such a game does not start. Tests and the document are added.

### The built-in scripts file and the extraction tool

- The default file of RGSS3 is `$XDG_DATA_HOME/vkxp/rpg_builtin_rgss3.rb`
  (`~/.local/share/vkxp/` without the variable), beside the other two; `--builtin-scripts`
  and `builtin_scripts` of a configuration file name it for the run's runtime, as today.
  The message for a missing file names the file and the section of `docs/running.md` that
  says how it is made ("The built-in scripts of RPG Maker VX Ace"), as the message of RGSS2
  names its section. The program checks that the file exists; it does not read it.
- `cargo xtask extract-rpg <DLL> <OUTPUT>` keeps its two arguments and accepts what it
  accepts today: any file. For RGSS3, as for RGSS2, the file that works is the memory image
  of the DLL written by the user; the DLL file itself is packed and gives the error for
  "nothing found".
- The tool gets two more markers: a block is a NUL-terminated string of text that starts
  with `module RPG;`, `def rgss_main` or `def rgss_stop`. Blocks are written in the order
  they appear, each followed by a line break, as today. No option names the runtime: the
  DLLs of RGSS1 and the images of RGSS2 hold neither of the two new texts (counted on
  2026-10-13 on the DLLs 1.00J, 1.02E, 1.02J and the unpacked 1.04E, four `module RPG;`
  each, and on the images of the DLLs 2.00 and 2.02, one each), so their output is what it
  was, and an image of RGSS3 gives its three blocks.
- The message for "nothing found" also says that a DLL of VX or VX Ace gives its blocks
  only from a memory image and names `docs/running.md`.

### What a run of RGSS3 does

`runtime_of` no longer refuses. The refusal moves into one function that `run` and
`run_guest` both call right after `guest_inputs`, before the recording of `--record` is
created, before a window, a device, an interpreter or a guest process is started, and
before the working directory changes. For a runtime other than RGSS3 it does nothing. For
RGSS3 the order of a run is therefore:

1. `Game.ini`, the runtime (`--runtime` or detection), the info line that names it.
2. The run-time package of `RTP`, by the table of `--rtp` and the configuration files.
3. The built-in scripts file of RGSS3 exists.
4. The game's files are opened (`GameFiles::open`): `Game.rgss3a` when the directory holds
   it, with the info line that names the archive.
5. The file that `Scripts=` names is read from them as a data file (from the archive, or
   loose); the bytes are dropped, and an info line names the file and its size.
6. The run ends with the message of today, which names RPG Maker VX Ace and `rgss3` and
   says "planned and not supported yet", and with the status of today (1).

A failure in 1 to 5 ends the run with that step's own error and status 1, and step 6 is not
reached. This is the behaviour the guest's changes need: they replace step 6 by starting
the guest executable, and steps 1 to 5 stay. The price is that a user without the built-in
scripts file is told to make it before being told that the game will not run yet; the
document says both in one place.

### The oracle is a change of its own

Not small, and not checkable in a flow:

- It needs a third probe. The probe of the oracle replaces `Input` by a model of the
  schedule and keeps its own frame number; that of RGSS3 has to do so with Symbols for the
  buttons and with the syntax and library of Ruby 1.9.2. The probes of XP and VX are tested
  by running them in vkxp; no guest of RGSS3 exists to run this one, and a flow cannot run
  Wine, so the probe would be merged without ever having run.
- It needs a second reader and a writer of RGSSAD version 3 in `xtask` (independent of the
  one in `vkxp-vfs`, as for version 1), which keeps the seed, the file keys and the twelve
  bytes of the end record when it replaces the script archive.
- On the original's side: the DLL goes to the path that `Library=` names (`System/`), not
  beside `Game.exe`; the registry key is `Software\Enterbrain\RGSS3\RTP`; the window may be
  resized at once (640 by 480 in one game of the corpus), which the search for the window
  and the grab, both by the frame size of the runtime, do not expect; and the games play
  Ogg audio through GStreamer, whose hardware plug-ins the measurement had to shut out
  beyond what the oracle's sandbox sets today.

It is to be drafted when the first RGSS3 classes run in a guest, with the facts above.

## Implementation notes

- `archive.rs`: `Archive::open` takes which version to read (a small enum, or the runtime).
  The reader of version 3 needs the length of the file as the reader of version 1 has it.
  The module's documentation describes both layouts. `Runtime::archive_name` may stop
  being an `Option`; its callers (`GameFiles::open`, `archive` in the program) follow.
- Tests of version 3 build their archives in test code from the layout above, with the
  seed, the file keys and the form of the end record (one word, or sixteen bytes) as
  parameters, and include: the 38 bytes of the scenario "Worked example" compared with the
  builder's output for seed 1, one entry `A`, file key 41, contents `abcde` (table key 12;
  the offset 33 is stored as 0x2D, the size 5 as 0x09, the file key as 0x25, the name
  length 1 as 0x0D, `A` as 0x4D, the ending word as 0x0C, and the fifth data byte is XORed
  with 0x22, the low byte of 41 * 7 + 3); the two measured seeds with their keys; the
  property tests of version 1 repeated for version 3. The builder used by
  `crates/vkxp/tests/end_to_end.rs` is written there a second time, as `rgssad` is.
- The end-to-end tests of RGSS3 use `Game::of(Runtime::Rgss3, ...)`, which exists and
  writes `Library=System\RGSS301.dll`, `Data/Scripts.rvdata2` and a stand-in built-in
  scripts file. The test `a_game_of_vx_ace_ends_before_any_script_runs` stays and keeps
  passing. Tests that need `RTP=RPGVXAce` or another `Library` rewrite `Game.ini`, as the
  test of the package of VX does. The unit test of `runtime_of` that expects the refusal
  there moves to the new function.
- The tests tell "the step failed" from "the run was refused" by the word "planned" on the
  error stream. The messages of steps 2 to 5 must not contain it.
- `extract.rs`: the search for the marker becomes a search for the first of three markers;
  the rest of `blocks` stays. A generated image for the tests holds no text of the
  original: short stand-ins that start with the markers.
- `docs/running.md`: the table "What follows the runtime" gets RGSS3 for the rows this
  change settles (run-time package key, archive, default built-in scripts file) and says
  for the others that they come with the guest; `--rtp RPGVXAce=DIR`; a section "The
  built-in scripts of RPG Maker VX Ace" (three blocks, from a memory image as for VX, the
  same command, the default file); the rows of `--rtp` and `--builtin-scripts` in the table
  of options; the sentence that a game of VX Ace "ends at once" becomes what the run does
  now. `args.rs`: the usage text names the third default file. The first sentences of the
  documents on what is supported do not change.

## Risks / Trade-offs

- The layout was measured on two archives, both written by the editor. An archive written
  by another tool may differ in what was not measured (see "Follow-ups"); the reader
  assumes nothing beyond the measured layout except the refusals of point 3 of "Reading
  the table of version 3".
- If `move-the-guest-role-into-a-library` merges first, the functions named here are in
  another file; if this change merges first, that change moves the new function with the
  others. Both touch `run` and `run_guest`, so the second to merge resolves a conflict by
  hand in a few lines.
- `extract-rpg` cannot tell an incomplete image of RGSS3 (only the first block) from an
  image of RGSS2. It says how many blocks it wrote; the document says there are three.

## Follow-ups

- Not measured on the original, so not decided here: whether RGSS3 reads an RGSSAD version
  1 archive under the name `Game.rgss3a`; what it does with a table that has no ending
  record, with an entry that lies outside the file, with two entries of the same name and
  with a name that is not UTF-8; what the twelve bytes after the ending word are. That the
  file keys are successive values of the C library's `rand` is inferred in the report and
  used nowhere.
- Fonts of RGSS3: `<RTP>/Fonts` as a font directory of the text backend, and the default
  font "VL Gothic" with its outline, in the change that binds `Font` of RGSS3.
- The oracle on a game of VX Ace, as a change of its own (see the decision above).
- A way for a user to write the memory image of the DLL, for VX and VX Ace alike.
- The loose script archive beside an archive: the original ignores it (measured for VX and
  for VX Ace); vkxp would read it when the archive has no such entry.
- Replacing step 6 of a run of RGSS3 by the start of `vkxp-guest-rgss3`, with the built-in
  scripts file then evaluated by the guest (`rgss_main` and `rgss_stop` are defined in it).
- `symbol_table`, the key map and the frame size answer for RGSS3 with the values of RGSS2
  today; nothing reaches them. They are settled with the guest.
