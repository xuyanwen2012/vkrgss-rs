## ADDED Requirements

### Requirement: Tilemap class of RGSS2
In RGSS2 `Tilemap` SHALL define `bitmaps`, which returns one `Tilemap::BitmapArray` of nine
slots that keep `nil` or a `Bitmap` by reference, and `passages` and `passages=` for a
`Table` kept by reference, and SHALL NOT define `tileset`, `autotiles` or `priorities`;
`TilemapAutotiles` SHALL NOT be defined. In RGSS1 the class SHALL stay as it is.

#### Scenario: Slots, measured on the original
- **WHEN** a game of RGSS2 evaluates `t.bitmaps[9] = b; t.bitmaps[9]`, `t.bitmaps[-1]`, `t.bitmaps[1] = b; t.bitmaps[1].equal?(b)` and `t.bitmaps.equal?(t.bitmaps)`
- **THEN** the results are `nil`, `nil`, `true` and `true`

#### Scenario: Tables, measured on the original
- **WHEN** a game of RGSS2 sets `t.map_data = table` and then `t.map_data = nil`, and likewise `passages`, and `t.flash_data = table; t.flash_data = nil`
- **THEN** the first two raise `TypeError` and keep the table, and the third succeeds

#### Scenario: Disposed, measured on the original
- **WHEN** a game of RGSS2 disposes a tilemap and evaluates `t.bitmaps[1] = b; t.bitmaps[1].equal?(b)` and `t.passages`
- **THEN** the first is `true` and the second raises `RGSSError`

#### Scenario: Names of RGSS1, measured on the original
- **WHEN** a game of RGSS2 evaluates `Tilemap.new.respond_to?(:tileset)` and `Object.const_defined?(:TilemapAutotiles)`
- **THEN** both are `false`, and the method test of RGSS2 names none of the tilemap's methods as pending

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Tilemap.new.respond_to?(:bitmaps)`
- **THEN** the result is `false`
