# Spec Delta

## ADDED Requirements

### Requirement: Win32API name fallback and file functions
A `Win32API` function name that is not in the emulation table SHALL be looked up again with
`A` appended before it is treated as unknown. `CopyFileA` SHALL copy a file when source and
destination are inside the game directory and SHALL return 0 without copying otherwise or when
its third argument is non-zero and the destination exists. `SetPriorityClass` SHALL return 1
and do nothing, and `GetProcessTimes` SHALL fill its four 8-byte buffers with zeros and
return 1.

#### Scenario: Plain name
- **WHEN** `Win32API.new("kernel32", "GetPrivateProfileString", "pppplp", "l")` reads the title from `.\Game.ini`
- **THEN** it behaves as `GetPrivateProfileStringA` and nothing is reported as not emulated

#### Scenario: Copy inside the game directory
- **WHEN** `CopyFileA` is called with `Save\a.rxdata`, `Save\b.rxdata` and 0, and the source exists
- **THEN** the destination has the same bytes and the return value is non-zero

#### Scenario: Copy outside
- **WHEN** `CopyFileA` names a destination outside the game directory
- **THEN** nothing is written and the return value is 0
