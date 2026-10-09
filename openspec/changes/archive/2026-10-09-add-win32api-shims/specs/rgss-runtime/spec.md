# Spec Delta

## ADDED Requirements

### Requirement: Emulated Win32API functions
`Win32API` objects for the functions of the emulation table SHALL behave as the Windows
function does for the uses listed here, whatever the letter case of the library name and with
or without its `.dll` suffix, and SHALL write results into string arguments in place.

#### Scenario: Reading the game configuration file
- **WHEN** `GetPrivateProfileStringA` is called with section `Game`, key `Title`, a default, a buffer of 256 bytes and the file name `.\Game.ini`
- **THEN** the buffer starts with the title in the file's encoding followed by a zero byte, and the return value is the title's length in bytes

#### Scenario: Missing key
- **WHEN** the key does not exist
- **THEN** the buffer holds the default and its length is returned

#### Scenario: Integer value
- **WHEN** `GetPrivateProfileIntA` reads a key whose value is `42`
- **THEN** it returns 42, and the given default for a missing key

#### Scenario: Writing
- **WHEN** `WritePrivateProfileStringA` sets a key in a file inside the game directory and the key is read back
- **THEN** the new value is returned

#### Scenario: Text conversion
- **WHEN** `MultiByteToWideChar` with code page 65001 converts the UTF-8 bytes of `测试` and `WideCharToMultiByte` with code page 936 converts the result
- **THEN** the final bytes are the GBK encoding of `测试`, and each call with a zero-size output returns the size needed

#### Scenario: Screen size and ticks
- **WHEN** `GetSystemMetrics` is called with 0 and with 1, and `GetTickCount` twice with frames in between
- **THEN** the metrics are 640 and 480 or larger, and the second tick count is not smaller than the first

#### Scenario: Window handle
- **WHEN** `FindWindowA` is called with the class `RGSS Player` and `GetClientRect` with the returned handle and a 16-byte buffer
- **THEN** the handle is non-zero and the buffer holds the rectangle 0, 0, 640, 480 as four 32-bit integers

### Requirement: Files reached through Win32API stay in the game directory
Profile functions SHALL read and write only files inside the game directory; a path outside
it SHALL be treated as a missing file for reading and SHALL fail for writing.

#### Scenario: Outside path
- **WHEN** `WritePrivateProfileStringA` names `C:\Windows\win.ini` or `../other.ini`
- **THEN** nothing is written and the return value is 0
