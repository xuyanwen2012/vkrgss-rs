## ADDED Requirements

### Requirement: Names interned at first use in RGSS2
In a game of RGSS2 the runtime SHALL intern, at the moments the original runtime does and in
its order, the names `owner` and `observer`, the six names `__hdll__`, `__dll__`,
`__dllname__`, `__proc__`, `__import__`, `__export__` of `Win32API`, and `bmp0` to `bmp8` of a
tilemap's bitmap slots, and SHALL intern no name of its own at run time that takes a number
of the sequence.

#### Scenario: First uses, measured on the original
- **WHEN** a game of RGSS2 notes the number of a new name before and after each of `Color.new(0, 0, 0)`, `Bitmap.new(1, 1)`, `Sprite.new`, and `Win32API.new("kernel32", "GetTickCount", "", "L")`
- **THEN** the number advances by 0, 2, 0 and 6 beyond the name noted

#### Scenario: Tilemap, measured on the original
- **WHEN** a game of RGSS2 notes the number of a new name before and after `Tilemap.new`, then after `t.bitmaps[3]`, and after `t.bitmaps[3] = nil`
- **THEN** the number advances by 0, by 1 and by 0 beyond the name noted

#### Scenario: Every measured step
- **WHEN** the steps of the committed probe of first uses run in a game of RGSS2
- **THEN** the number of a new name after each step equals the number measured on the original, except at the steps the test lists with their reason

#### Scenario: XP
- **WHEN** the existing test of the symbol numbers of a game of RGSS1 runs
- **THEN** it passes without a change to its expected numbers
