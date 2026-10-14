## ADDED Requirements

### Requirement: Win32API of RGSS3
In a game of RGSS3 `Win32API` SHALL have the methods `initialize` (arity 4, private), `call`
and `Call` (arity -1) of the original runtime, SHALL take the library and the function as
Strings and the declarations as a String, an Array of Strings or `nil`, SHALL keep its state
out of `instance_variables`, and SHALL not be dumped. This SHALL hold in the place of the
requirement "Win32API placeholder" where the two differ.

#### Scenario: Method list, measured on the original
- **WHEN** the test of the method list of RGSS3 checks `Win32API`
- **THEN** its three methods have the arities of the list and it has no other public method of its own

#### Scenario: Arguments, measured on the original
- **WHEN** a script evaluates `Win32API.new("user32", "GetKeyState", "i")`, `Win32API.new(:user32, "GetKeyState", "i", "i")` and `Win32API.new("user32", "GetKeyState", nil, nil)`
- **THEN** the first raises `ArgumentError` with `wrong number of arguments(3 for 4)`, the second `TypeError` with `can't convert Symbol into String`, and the third returns a `Win32API`

#### Scenario: The object, measured on the original
- **WHEN** a script evaluates `api = Win32API.new("user32", "GetKeyState", "i", "i")`, then `[api.instance_variables, api.dup.call(0x41), Class.new(Win32API).new("user32", "GetKeyState", "i", "i").call(0x41)]` and `Marshal.dump(api)`
- **THEN** the first is `[[], 0, 0]` and the dump raises `TypeError` with `no marshal_dump is defined for class Data`

### Requirement: Libraries of Win32API in RGSS3
In a game of RGSS3 `Win32API.new` SHALL find a library that is a file of the game, the
runtime library of `Game.ini`, or a Windows system library of the list of this change's
design, without regard to letter case and with or without `.dll`, and SHALL raise
`RuntimeError` with `LoadLibrary: <name>` and a line feed for any other. A function of a
found library outside the emulation table SHALL be reported once per pair, as before.

#### Scenario: A missing library, measured on the original
- **WHEN** a script evaluates `Win32API.new("nosuch", "Function", "", "")`
- **THEN** it raises `RuntimeError` with the message `"LoadLibrary: nosuch\n"`

#### Scenario: The game's own libraries, measured on the original
- **WHEN** a game whose `Game.ini` names `System\RGSS301.dll` and whose directory holds that file and `tktk_bitmap.dll` evaluates `Win32API.new` with `"System/RGSS301.dll"`, `"RGSS301"` and `"tktk_bitmap"`, then `call` of each
- **THEN** no constructor raises, each call returns 0, and the error stream has one line for each library and function pair

#### Scenario: System libraries
- **WHEN** a script makes `Win32API` objects of `"wininet"`, `"Kernel32"`, `"USER32.DLL"` and `"msvcrt"`
- **THEN** none raises

### Requirement: Declarations and count of Win32API in RGSS3
In a game of RGSS3 each character `p`, `l`, `i` or `n` of the import, in either case, SHALL
declare one parameter, `v` and every other character none; an Array of Strings SHALL
declare the first character of each. A call with another number of arguments SHALL raise
`RuntimeError` with `wrong number of parameters: expected <declared>, got <given>`.

#### Scenario: Ignored characters, measured on the original
- **WHEN** a script evaluates `Win32API.new("user32", "GetKeyState", "pointer", "i").call(1, 2)`
- **THEN** it raises `RuntimeError` with `wrong number of parameters: expected 3, got 2`

#### Scenario: Count, measured on the original
- **WHEN** a script evaluates `Win32API.new("user32", "GetKeyState", "i", "i").call`
- **THEN** it raises `RuntimeError` with `wrong number of parameters: expected 1, got 0`

#### Scenario: Array, spaces and v
- **WHEN** a script calls `Win32API` objects declared `["l", "p", "p", "l", "l", "p"]` with six arguments, `"n i i i i"` with five, `[]` and `"v"` with none
- **THEN** none raises

### Requirement: Parameters of Win32API in RGSS3
In a game of RGSS3 an integer parameter SHALL be converted as the interpreter converts to an
unsigned long and taken by its low 32 bits; a pointer parameter SHALL take a String that is
not frozen as a buffer, `nil` and 0 as NULL, another Integer as an address, and SHALL raise
`TypeError` for anything else. The errors SHALL be worded as measured on the original.

#### Scenario: Integer parameters, measured on the original
- **WHEN** a script calls `GetKeyState` declared `"i"` with `nil`, `"a"`, `true` and `65.9`
- **THEN** the first raises `TypeError` with `no implicit conversion from nil to integer`, the second `TypeError` with `can't convert String into Integer`, the third `TypeError`, and the last returns what `GetKeyState` gives for key 65

#### Scenario: Pointer parameters, measured on the original
- **WHEN** a script calls `GetKeyboardState` declared `"p"` with a frozen String of 256 bytes, `1.5`, `:sym` and `[]`
- **THEN** the frozen String raises `RuntimeError` with `can't modify frozen string`, and `1.5`, `:sym` and `[]` raise `TypeError` with `can't convert Float into String`, `can't convert Symbol into String` and `can't convert Array into String`

#### Scenario: Object ids
- **WHEN** a script calls a function of `tktk_bitmap` declared `"n"` with `Bitmap.new(1, 1).object_id`
- **THEN** it returns 0 and raises nothing

#### Scenario: Unsigned, measured on the original
- **WHEN** a script calls `GetKeyState` declared `"i"` with `2**32 - 1` and with `2**64`
- **THEN** the first returns 0, and the second raises `RangeError` with ``bignum too big to convert into `unsigned long'``

### Requirement: Values of Win32API in RGSS3
In a game of RGSS3 the export `v` SHALL give 0, the export `p` SHALL raise `ArgumentError`
with `NULL pointer given` for 0 and give an empty ASCII-8BIT String otherwise, and any other
export SHALL give a signed 32-bit Integer. A buffer an emulated function fills SHALL keep
its length and its encoding label.

#### Scenario: Exports, measured on the original
- **WHEN** a script calls `GetSystemMetrics` with 5 declared with the exports `"v"`, `"p"`, `""`, `nil` and `"x"`
- **THEN** the first gives 0, the second raises `ArgumentError` with `NULL pointer given`, and the others give 0

#### Scenario: Signed, measured on the original
- **WHEN** `GetKeyState` declared `"i"` reports a held key
- **THEN** the value is negative, as a signed 32-bit Integer

#### Scenario: Not emulated with p
- **WHEN** a script calls a function of `user32` outside the emulation table declared with the export `"p"`
- **THEN** the value is an empty ASCII-8BIT String and one line names the pair

#### Scenario: A buffer keeps its label, measured on the original
- **WHEN** a script passes the UTF-8 String `"\0" * 16` to `GetClientRect` with the handle of `FindWindowA`
- **THEN** after the call the String is still labelled UTF-8 and has 16 bytes, which unpack with `"l4"` to `[0, 0, 544, 416]`

### Requirement: Measured calls of Win32API in RGSS3
In a game of RGSS3 the profile functions SHALL read nothing and write nothing for a file
name without `/` or `\`, the window class `RGSS Player` SHALL find the game window, and an
Integer source of `RtlMoveMemory` SHALL copy nothing. The other emulated functions SHALL
give the values measured on the original runtime where they were measured.

#### Scenario: Title, measured on the original
- **WHEN** a game whose `Game.ini` has a title of 16 bytes calls `GetPrivateProfileStringA` declared `"pppplp"` with `("Game", "Title", "", buf, 255, "./Game.ini")` and then with the file `"Game.ini"`
- **THEN** the first returns 16 with the title in the buffer, and the second returns 0

#### Scenario: Window, measured on the original
- **WHEN** a script calls `FindWindowA` with `"RGSS Player"` and the title, and `GetClientRect` with the handle and a 16-byte buffer
- **THEN** the handle is not 0 and the rectangle is 0, 0, 544, 416

#### Scenario: Cursor, measured on the original
- **WHEN** the first call of `ShowCursor` of a run is `ShowCursor(0)`
- **THEN** it returns -1

#### Scenario: Text, measured on the original
- **WHEN** `MultiByteToWideChar(65001, 0, s, -1, buf, n)` converts the UTF-8 bytes of `日本` and `WideCharToMultiByte` converts the result to 65001 and to 932
- **THEN** the bytes are those of `日本` in UTF-8 and in Shift_JIS, each with a zero byte

#### Scenario: The address trick
- **WHEN** a script calls `RtlMoveMemory` declared `"pll"` with a buffer of 4 bytes, `bitmap.object_id * 2 + 16` and 4
- **THEN** the buffer is unchanged and nothing raises

### Requirement: DL of RGSS3
In a game of RGSS3 the module `DL` SHALL exist with the numeric constants of the original's
method list and the classes `DLError`, `DLTypeError`, `Handle`, `CFunc` and `CPtr`, and
every function of the module and every constructor and class method of those three classes
SHALL raise `DL::DLError` with a message that names the call. Games of RGSS1 and RGSS2
SHALL have no `DL`.

#### Scenario: Constants, measured on the original
- **WHEN** a script evaluates `[DL::TYPE_VOID, DL::TYPE_INT, DL::TYPE_DOUBLE, DL::SIZEOF_VOIDP, DL::SIZEOF_LONG_LONG, DL::ALIGN_SHORT, DL::RTLD_NOW, DL::DLError.superclass, DL::DLTypeError.superclass]`
- **THEN** the result is `[0, 4, 8, 4, 8, 2, -1, StandardError, DL::DLError]`

#### Scenario: Refused
- **WHEN** a script evaluates `DL.dlopen("user32")`, `DL::Handle.new("user32")`, `DL::CFunc.new(0)`, `DL::CPtr.malloc(8)` and `DL.malloc(8)`
- **THEN** each raises `DL::DLError` whose message names the call

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 or RGSS2 evaluates `defined?(DL)`
- **THEN** the result is `nil`

### Requirement: Win32API of XP and VX over Ruby 1.9.2
The tests of the emulated `Win32API` functions of RGSS1 and RGSS2 SHALL pass over both
interpreters with their expected values unedited, without an entry of the allow-list of
Ruby 1.9.2, and `Win32API` of RGSS1 and RGSS2 SHALL keep its behaviour.

#### Scenario: Keyboard state
- **WHEN** the tests of the toggle, of `SetKeyboardState` and of `GetKeyboardState` run over Ruby 1.9.2
- **THEN** they pass, and the allow-list has no entry for `win32api`

#### Scenario: Unchanged
- **WHEN** the existing tests of `tests/win32api.rs` run over Ruby 1.8.7
- **THEN** they pass unedited but for the Ruby snippets of the three tests above
