# Design

## Context

Row 10 of the table "Follow-ups: the changes that follow" of the design of
`run-rgss3-games-in-a-guest-of-their-own` (archived 2026-10-13): "`Win32API` as RGSS3 has
it, `DL` as far as the corpus needs it", measured source API report 5.8 and 5.9, core API
none. That change left `Win32API` and `DL` over Ruby 1.9.2 out ("Non-Goals").

Read for this change, seen from a clone under `wt/`:

- `crates/vkxp-binding/src/{win32api,runtime,rb,wrap}.rs`, `prelude_shared.rb`, the tests
  `crates/vkxp-binding/tests/win32api.rs` and `crates/vkxp-binding-rgss3/tests/
  {win32api,arity_rgss3}.rs`, `ruby192.allow`.
- The requirements "Win32API placeholder", "Emulated Win32API functions", "Files reached
  through Win32API stay in the game directory", "Win32API name fallback and file
  functions", "Keyboard and mouse through Win32API", "Pointer through Win32API", "Key
  injection functions are accepted", "Pointer visibility through Win32API" of
  `openspec/specs/rgss-runtime`; `docs/decisions.md`, "Corpus" (a DLL a game ships, a
  script that reads the memory of the runtime: each a decision of its own) and "VX Ace"
  (word size; `DL` gets a decision when a game needs it).
- The measurements, outside the repository: `../../ref/tools/oracle/measurements/
  ace-v1-api-REPORT.md` ("the API report", 2.3, 3.3, 4.1, 5.8, 5.9, 9), its samples
  `ace-v1-api-interp-samples.*` (section Win32API); `ace-v0-REPORT.md` ("the V0 report",
  7 and 8); `../../ref/RGSS301_api.md` (`Win32API`: `Call/-1`, `call/-1`, `initialize/4`;
  the module `DL` and its classes).

What vkxp has today. `prelude_shared.rb`, evaluated for every runtime, defines `Win32API`
in Ruby: `initialize(dll, function, import, export)` keeps the names in instance variables
and ignores the declarations; `call(*args)` hands them to the host function
`vkxp_win32api`, which looks the pair up in `TABLE` of `win32api.rs` (library without
letter case and `.dll`, function or function + `A`), converts `nil` to 0, a String to a
buffer it writes back with `String#replace` and anything else with `rb::long`, and returns
the entry's `i64` as an Integer; a function outside the table returns 0 and writes one line
per pair. Over Ruby 1.9.2 all of this runs: the shared tests of `win32api.rs` run over both
interpreters, three of them listed in `ruby192.allow` because their Ruby snippets read and
write bytes with `String#[]`, which Ruby 1.9 does differently. `DL` is not defined: the
interpreter crate does not link the extension.

What the original runtime of RGSS3 does (API report 5.8), against today:

| Rule | Original | vkxp today in RGSS3 |
|---|---|---|
| constructor | exactly 4 arguments (`wrong number of arguments(3 for 4)`); library and function Strings (`can't convert Symbol into String`); import and export may be `nil` | 4 arguments with Ruby's wording for a Ruby method; anything is turned into a String by `to_s` |
| object | `call`, `Call`; no instance variables visible; `dup` and subclasses work; `Marshal.dump` raises `TypeError: no marshal_dump is defined for class Data` (2.3) | two instance variables `@vkxp_dll`, `@vkxp_function` visible; can be dumped |
| lookup | `LoadLibrary(dll)`, `GetProcAddress(name)`, then `name + "A"`; `RuntimeError: LoadLibrary: nosuch\n`, `RuntimeError: GetProcAddress: Name or NameA\n`; the game's own DLL can be opened (`System/RGSS301.dll`, `RGSS301`) | never raises |
| import | each character a parameter: `p`/`P` pointer, `l`/`L`, `i`/`I`, `n`/`N` 32-bit integer, `v`/`V` none, any other character ignored (`"pointer"` is p, i, n); an Array of Strings, the first character of each | ignored |
| count | `RuntimeError: wrong number of parameters: expected 1, got 0` | not checked |
| integer parameter | Float truncated; Bignum up to 2**32 - 1 unsigned and wrapped; above ``RangeError: bignum too big to convert into `unsigned long'``; -2**31 and below `bignum out of range of unsigned long`; `nil` `TypeError: no implicit conversion from nil to integer`; String, `true` `can't convert String into Integer` | `nil` is 0; a String is a buffer; anything else `rb::long` (64 bits) |
| pointer parameter | a String, written in place, length kept; a frozen String `RuntimeError: can't modify frozen string`; `nil` and 0 NULL; another Integer the address; Float, Symbol, Array `TypeError: can't convert Float into String` | a String a buffer; `nil` 0; Integer an address; Float converted |
| export | `p`: a new ASCII-8BIT String up to the NUL, `ArgumentError: NULL pointer given` for 0; `v`: 0; anything else, `""`, `nil`, `"x"`: a signed 32-bit Integer | the entry's value as it is |
| buffer label | a filled buffer keeps its label (3.3) | `String#replace` labels it ASCII-8BIT over Ruby 1.9 |
| conventions | stdcall and cdecl | no DLL is called |

Calls measured on the original (5.8), and today: `GetPrivateProfileStringA("Game", "Title",
"", buf, 255, "./Game.ini")` returns 16 and the title, emulated; with the bare name
`"Game.ini"` it returns 0 (Windows looks in the Windows directory), where vkxp reads the
game's file. `FindWindowA("RGSS Player", title)`: the window class is `RGSS Player`;
emulated (any class finds the game window). `GetClientRect` 544 by 416: emulated (the
logical size). `GetWindowRect` 550 by 448 and `GetSystemMetrics(0/1)` 1024 by 768 under
Wine: vkxp gives the logical size for both. `ShowCursor(0)` gives -1 from the start value
0: emulated, the count follows the documentation of Windows and is now measured too.
`GetAsyncKeyState`, `GetKeyState`, `GetKeyboardState`, `MultiByteToWideChar(65001, ...)`,
`WideCharToMultiByte` to 65001 and 932: emulated. `RtlMoveMemory("pll")` from an address,
and the address trick of bitmap scripts (the pixels found through `object_id * 2 + 16`,
BGRA, bottom row first): an integer source copies nothing in vkxp.
`GetWindowThreadProcessId`, `GetModuleHandle` (0x00400000 for `nil`, 0x10000000 for the
DLL), `strlen`, `abs`, `sprintf` of `msvcrt`: not emulated.

The calls of the corpus (the scripts of the three games were read, the two archives
decrypted into `target/` with `ref/tools/oracle/measurements/ace-v0-rgss3a.py`; nothing is
copied into the repository):

| Game | Library | Functions, declaration | When | Today | After this change |
|---|---|---|---|---|---|
| Crysalis | none | | | | |
| In Search of Immortality | `user32` | `GetAsyncKeyState` `"i"`, `GetKeyState` `"i"` and `["i"]` | `Input.update` (debug script) and every `update_basic` (Tab) | emulated | emulated |
| | `wininet` | `InternetOpenA` (called once while the scripts load), `InternetConnectA`, `InternetOpenUrl`, `HttpQueryInfo`, `InternetReadFile`, `InternetCloseHandle`, Array declarations | an online account, only with a profile | not emulated, 0 and one line each | the library is found (list); not emulated, 0 and one line each |
| | `Kernel32` | `GetLastError` `[]` | as above | not emulated | not emulated |
| Capella's Promise | `user32` | `FindWindow` `"pp"` with `("RGSS Player", 0)`, `MoveWindow` `"liiiil"`, `GetSystemMetrics` `"i"` | F5, the window zoom | `FindWindowA` and `GetSystemMetrics` emulated, `MoveWindow` not | the same |
| | `tktk_bitmap` (`tktk_bitmap.dll` beside `Game.exe`) | `PngSaveA`, `Blur`, `ChangeTone`, `ClipMask`, `InvertColor`, `Mosaic`, `GetAddress`, `GetPixelData`, `SetPixelData`, `BlendBlt`, `ChangeSize`, declarations with spaces (`"n i i i i"`) | all made while the scripts load; `InvertColor` and `BlendBlt` called by the light script on maps whose darkness switch is on, with `bitmap.object_id`; `GetPixelData` and `SetPixelData` by `Bitmap#_dump` and `_load` | not emulated, 0 | the library is found (a file of the game); not emulated, 0 |

Neither game uses `DL`, `RtlMoveMemory` or the address trick.

## Goals / Non-Goals

**Goals:**

- `Win32API` of RGSS3 as measured, with each departure decided and recorded.
- The calls of the corpus run as they do today: no new error at load or on a map.
- `DL` defined as the original's module, refusing what vkxp cannot do.
- The three entries of `ruby192.allow` for `win32api` go.
- XP and VX unchanged: every existing test passes over both interpreters with its expected
  values unedited.

**Non-Goals:**

- Running a DLL a game ships, or reading or writing the memory of the process
  (`docs/decisions.md`, "Corpus"): `tktk_bitmap.dll` gets a change and a measurement of
  its own (Follow-ups).
- New emulated functions beyond the bare profile name: none of the corpus needs one.
- The six names `Win32API.new` interns lazily (API report 4.1): `intern-the-names-of-rgss3`.
- Any change of the protocol, the host or the interpreter crates.

## Decisions

1. **The class of RGSS3 is Rust.** `win32api::define(ruby, runtime)` defines, for RGSS3,
   `Win32API < Object` with `initialize` (arity 4, private), `call` and `Call` (arity -1)
   as host functions, so a wrong count of the constructor reads `wrong number of
   arguments(3 for 4)` as measured and the arities are those of the method list. The
   object's state (library, function, the parsed import, the export) is a Rust value in an
   object of the class `Data`, kept in an instance variable whose name has no `@`, so
   `instance_variables` is `[]`, `dup` and `clone` share the state, a subclass inherits
   the methods, and `Marshal.dump` of the object raises `TypeError: no marshal_dump is
   defined for class Data`, as measured. `Win32API` is added to the bound classes of
   `crates/vkxp-binding-rgss3/tests/arity_rgss3.rs`.
   - The class of `prelude_shared.rb` moves to a new `prelude_win32api.rb`, evaluated for
     RGSS1 and RGSS2 where `prelude_shared.rb` is today; the lines it leaves in
     `prelude_shared.rb` become comment lines, so that no line number of that file moves
     (as the guest change did with `prelude.rb`).

2. **Arguments of the constructor.** The library and the function are converted as the
   interpreter converts a String argument (`can't convert Symbol into String`; an object
   with `to_str` accepted). The import is a String, an Array of Strings (the first
   character of each; an empty String or an empty Array declares nothing), or `nil`
   (nothing); the export a String or `nil`. Other types of the declarations were not
   measured: they raise `TypeError` with the interpreter's wording for a String.

3. **Which libraries exist.** vkxp loads no library. A library name is *found* when, with
   letter case ignored and with or without `.dll`:
   - it names a file of the game directory (with its directory part, as `System/RGSS301.dll`),
     as `GameFiles::resolve(FileKind::Data, ..)` finds it, `tktk_bitmap` among them;
   - it is the runtime library of `Game.ini` (`Library=`), by its path or by its file name
     without directory (`RGSS301`), which the original has loaded;
   - it is one of these system libraries of Windows: `kernel32`, `kernelbase`, `user32`,
     `gdi32`, `advapi32`, `shell32`, `shlwapi`, `ole32`, `oleaut32`, `comctl32`,
     `comdlg32`, `winmm`, `wininet`, `winhttp`, `urlmon`, `ws2_32`, `wsock32`, `msvcrt`,
     `ntdll`, `version`, `imm32`, `psapi`, `dwmapi`, `uxtheme`, `msimg32`, `gdiplus`,
     `dsound`, `ddraw`, `dinput`, `dinput8`, `xinput1_3`, `opengl32`, `d3d9`, `setupapi`,
     `crypt32`, `iphlpapi`, `secur32`, `netapi32`, `mpr`, `rpcrt4`, `hid`, `powrprof`.
   Any other name raises `RuntimeError` with the message `LoadLibrary: <name as given>` and
   a line feed, at `Win32API.new`, as for a missing DLL on the original; scripts that probe
   for a library (a Steam library, say) by rescuing that error take their fallback. A
   system library outside the list raises too where the original would load it: known
   difference, and the list grows when a game shows a need.
   - A function of a found library never raises `GetProcAddress`: vkxp does not know the
     functions of a library it does not load. Known difference. The function is looked up
     in `TABLE` as today (with `A` appended second); outside it, the call returns the value
     of decision 6 for a function that is not emulated and writes one line per library and
     function pair to the error stream, as for XP and VX.

4. **The import and the count.** Each character of the import in `pPlLiInN` is a parameter
   (`p`/`P` a pointer, the others an integer), `v`/`V` and every other character declare
   nothing. `call` with another number of arguments raises `RuntimeError` with
   `wrong number of parameters: expected <declared>, got <given>`, before any conversion.

5. **Conversions.**
   - *Integer parameter*: converted as the interpreter converts to an unsigned long
     (`rb_num2ulong`: a Float truncated, `nil` raising `TypeError: no implicit conversion
     from nil to integer`, a String `TypeError: can't convert String into Integer`, `true`
     a `TypeError` in the interpreter's words, which the report did not quote), then taken
     by its low 32 bits. The RangeErrors of the measured wording
     (``bignum too big to convert into `unsigned long'``, `bignum out of range of unsigned
     long`) so come at the 64-bit bounds of the guest, not at 2**32 and -2**31 as on the
     original. This is the word-size rule of `docs/decisions.md` ("VX Ace"): a game of the
     corpus depends on it. Capella's Promise passes `bitmap.object_id` to `tktk_bitmap`,
     and an object id of the 64-bit guest is above 2**32: the measured RangeError would
     end the game on its first dark map. Known difference.
   - *Pointer parameter*: a String is a buffer of its bytes; a frozen String raises
     `RuntimeError: can't modify frozen string`; `nil` is NULL; an Integer is converted as
     above and is an address; anything else raises `TypeError` as the interpreter converts
     a String (`can't convert Float into String`).
   - The entries of `TABLE` receive `Arg::Int` and `Arg::Buffer` as today.

6. **Values.** The export's first character decides: `v`/`V` gives 0; `p`/`P` gives, for
   0, `ArgumentError: NULL pointer given`, and for any other value an empty ASCII-8BIT
   String, because no value of an emulated function is an address the guest has (the
   original copies the memory there, and faults where there is none); anything else,
   `""` and `nil` included, gives the value as a signed 32-bit Integer. A function that is
   not emulated gives 0 by the same rule, except that for `p` it gives an empty ASCII-8BIT
   String instead of raising, so that a game goes on as with the other exports.

7. **Buffers keep their label** (3.3). A buffer an entry changed is written back into the
   very String with its bytes, without changing its length or its encoding label (not with
   `String#replace`, which labels it ASCII-8BIT over Ruby 1.9). RGSS1 and RGSS2 keep
   `replace`.

8. **The profile functions with a bare file name.** In RGSS3, a file name for
   `GetPrivateProfileStringA`, `GetPrivateProfileIntA` and `WritePrivateProfileStringA`
   without `/` or `\` names a file of the Windows directory, which vkxp does not have:
   reading gives the default (`GetPrivateProfileStringA` with the default `""` returns 0,
   as measured with `"Game.ini"`), writing writes nothing and returns 0. The write was not
   measured; it follows the rule that files reached through `Win32API` stay in the game
   directory.

9. **The address trick is refused** (5.8, last point). An Integer source of
   `RtlMoveMemory` copies nothing, as today; vkxp has no memory at the original's
   addresses and does not give scripts the memory of the guest. No game of the corpus
   uses it. Known difference, recorded with the measured layout so a later decision can
   find it.

10. **`DL`** (5.9; the method list). `prelude_dl.rb`, evaluated for RGSS3 only, defines
    the module `DL` with the constants of the method list that are numbers: `TYPE_VOID` 0,
    `TYPE_VOIDP` 1, `TYPE_CHAR` 2, `TYPE_SHORT` 3, `TYPE_INT` 4, `TYPE_LONG` 5,
    `TYPE_LONG_LONG` 6, `TYPE_FLOAT` 7, `TYPE_DOUBLE` 8, `SIZEOF_*` and `ALIGN_*` (`CHAR`
    1, `SHORT` 2, `INT` 4, `LONG` 4, `LONG_LONG` 8, `FLOAT` 4, `DOUBLE` 8, `VOIDP` 4),
    `RTLD_GLOBAL`, `RTLD_LAZY`, `RTLD_NOW` -1, `DLSTACK_SIZE` 20, `MAX_CALLBACK` 5; the
    classes `DL::DLError < StandardError`, `DL::DLTypeError < DL::DLError`, `DL::Handle`,
    `DL::CFunc`, `DL::CPtr`. Every function of the module (`dlopen`, `dlwrap`, `dlunwrap`,
    `malloc`, `realloc`, `free`, as module functions with the arities of the list) and
    every constructor and class method of the three classes (`Handle.new`, `Handle[]`,
    `Handle.sym`, `CFunc.new`, `CPtr.new`, `CPtr[]`, `CPtr.malloc`, `CPtr.to_ptr`) raises
    `DL::DLError` with a message that names the call and says that vkxp loads no library
    and reads no memory of the process. `DL::DLError` is what a script rescues for a
    library that cannot be opened. Refused, not emulated: `DL` gives a script the memory
    of the runtime and calls of any address, which `docs/decisions.md` leaves to a
    decision of its own, and no game of the corpus needs it. Left out: `NULL`,
    `RUBY_FREE`, `Handle::DEFAULT`, `Handle::NEXT` (objects the refusal cannot make), the
    four callback constants (a `LoadError` on the original), `BUILD_RUBY_PLATFORM`, the
    instance methods of the three classes (no instance exists).

11. **The shared tests over Ruby 1.9.2.** The snippets of
    `the_toggle_of_a_key_flips_at_each_latched_press`,
    `set_keyboard_state_replaces_what_is_reported_until_the_next_update` and
    `the_keyboard_state_has_0x81_for_a_key_held_for_the_first_time` of
    `crates/vkxp-binding/tests/win32api.rs` read and write the bytes of their buffers with
    `unpack("C*")` and `pack("C*")`, which both interpreters have, in place of `String#[]`
    and `String#[]=` (Ruby 1.8.7 has no `getbyte`); their expected values are not edited.
    Their markers and their entry of `ruby192.allow` go.

12. **Tests.** `crates/vkxp-binding-rgss3/tests/win32api_rgss3.rs`: every scenario of the
    requirements of this change that a scripted host can play, with a `Game.ini`, a
    `tktk_bitmap.dll` stand-in (an empty file named so) and the files of the profile tests
    written into the game directory by the test. `crates/vkxp/tests/rgss3.rs`: a game whose
    script makes `Win32API` objects of `wininet`, `user32` with Array and spaced
    declarations, and of a library file of the game, at load, and runs to its end with
    status 0 through both executables.

## Implementation notes

- A flow reads `../../ref/` and `../../games/` and copies nothing from them, no script text
  of a game and no name of a function beyond those this design lists. No Wine and no
  original runtime is run. Scratch output goes under `target/` or `local/`.
- Before anything changes, keep under `local/` the lists of `cargo test --locked -- --list`
  of `vkxp-binding` and `vkxp-binding-rgss3`. At the end every name is there again.
- The conversion functions of the interpreter crates (`rb_num2ulong`, `rb_string_value`,
  the check of a frozen string) give the measured wordings; the binding calls them through
  `rb.rs` rather than wording its own.
- `vkxp_win32api` and the class of `prelude_win32api.rb` keep serving RGSS1 and RGSS2
  unchanged.
- Every command that reaches Vulkan has `VK_LOADER_DRIVERS_SELECT='*lvp*'`.

## Which milestone needs which item

- **A1 to A3, In Search of Immortality**: its `GetKeyState` with an Array declaration is
  called in every frame of every scene, and `InternetOpenA` while the scripts load. Both
  work today. After this change they need `["i"]` and `["p", "l", ...]` read as
  declarations (decision 2), `wininet` and `Kernel32` found (decision 3) and the counts
  right; the end-to-end test of decision 12 holds that. Nothing else of this change.
- **A2, Capella's Promise**: the twelve `Win32API.new` of `tktk_bitmap` run while the
  scripts load and need the library found as a file of the game (decision 3), and the
  calls with `bitmap.object_id` on dark maps need decision 5. The pixels of a dark map
  need `InvertColor` and `BlendBlt` of `tktk_bitmap` emulated, which this change does not
  do (Follow-ups): A2 compared on a dark map of Capella's Promise depends on that change.
- **A3**: no item. `Bitmap#_dump` of Capella's Promise calls `GetPixelData` only when a
  bitmap is dumped, which its saves do not do (they hold no bitmap, as read).
- **Crysalis**: no item.
- `DL`: no game, no milestone.

## Follow-ups

Left out, with the measured source (API report unless named):

- `tktk_bitmap.dll` of Capella's Promise (V0 report 7): `InvertColor` and `BlendBlt` for
  the light script on dark maps, `GetPixelData` and `SetPixelData` for `Bitmap#_dump` and
  `_load`, the others unused by the game. A change that measures what the library does
  (under Wine, with the probe) and emulates the functions the corpus calls in Rust over
  the guest's bitmaps, found by `object_id`. A decision of its own ("Corpus").
- `GetProcAddress: Name or NameA` (5.8): vkxp never raises it (decision 3).
- System libraries outside the list of decision 3: they raise where the original loads them.
- The RangeErrors at 2**32 and -2**31 (5.8): at the 64-bit bounds in vkxp (decision 5).
- The export `p` with a non-zero value: the memory at that address on the original; an
  empty String in vkxp (decision 6).
- The address trick and `RtlMoveMemory` from an address (5.8): refused (decision 9).
- `GetWindowRect` 550 by 448 and `GetSystemMetrics(0/1)` 1024 by 768, seen through Wine: the
  logical size in vkxp. `GetModuleHandle` (0x00400000, 0x10000000),
  `GetWindowThreadProcessId`, `msvcrt` functions: not emulated.
- `FindWindowA` with a class other than `RGSS Player`: finds the game window in vkxp; not
  measured on the original.
- The window style 0x14CA0000 and extended style 0x00000100 (5.8): no function that reads
  them is emulated.
- Declarations of other types than String, Array and `nil` (decision 2): not measured.
- `DL` (5.9): `dlwrap` (`object_id * 2`), `CFunc#call`, `CPtr`, `Handle#sym`, the stdcall
  convention, `DL::DLError: unknown symbol`, `No such file or directory`, `unsupported call
  type`: refused (decision 10). `NULL`, `RUBY_FREE`, `Handle::DEFAULT`, `NEXT`,
  `BUILD_RUBY_*` and the callback constants are not defined.
- The six names `Win32API.new` interns (4.1): `intern-the-names-of-rgss3`.
