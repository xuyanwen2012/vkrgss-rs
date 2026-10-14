## ADDED Requirements

### Requirement: Tilemap class of RGSS3
In RGSS3 `Tilemap` SHALL define the methods the original runtime of VX Ace defines, with the
same arities: those of the class of RGSS2 with `flags` and `flags=` in the place of
`passages` and `passages=`. `bitmaps` SHALL return one `Tilemap::BitmapArray` of nine slots
that keep `nil` or a `Bitmap` by reference. In RGSS1 and RGSS2 `flags` SHALL NOT be defined
and the class SHALL stay as it is.

#### Scenario: Methods, measured on the original
- **WHEN** a game of RGSS3 evaluates `Tilemap.instance_methods(false).sort.map { |m| [m, Tilemap.instance_method(m).arity] }`
- **THEN** the result is `[[:bitmaps, 0], [:clone, 0], [:dispose, 0], [:disposed?, 0], [:dup, 0], [:flags, 0], [:flags=, 1], [:flash_data, 0], [:flash_data=, 1], [:map_data, 0], [:map_data=, 1], [:ox, 0], [:ox=, 1], [:oy, 0], [:oy=, 1], [:update, 0], [:viewport, 0], [:viewport=, 1], [:visible, 0], [:visible=, 1]]`

#### Scenario: Constructor and array, measured on the original
- **WHEN** a game of RGSS3 evaluates `[Tilemap.instance_method(:initialize).arity, Tilemap::BitmapArray.instance_methods(false).sort.map { |m| [m, Tilemap::BitmapArray.instance_method(m).arity] }]`
- **THEN** the result is `[-1, [[:[], 1], [:[]=, 2]]]`

#### Scenario: A new tilemap, measured on the original
- **WHEN** a game of RGSS3 evaluates `t = Tilemap.new; [t.map_data, t.flags, t.flash_data, t.viewport, t.ox, t.oy, t.visible, (0..9).map { |i| t.bitmaps[i] }.compact, t.bitmaps.class, t.bitmaps.equal?(t.bitmaps)]`
- **THEN** the result is `[nil, nil, nil, nil, 0, 0, true, [], Tilemap::BitmapArray, true]`

#### Scenario: Names of the other runtimes, measured on the original
- **WHEN** a game of RGSS3 evaluates `[:passages, :passages=, :tileset, :autotiles, :priorities].map { |m| Tilemap.new.respond_to?(m) }` and `Object.const_defined?(:TilemapAutotiles)`
- **THEN** the results are five times `false`, and `false`

#### Scenario: Viewport, measured on the original
- **WHEN** a game of RGSS3 evaluates `v = Viewport.new(1, 2, 3, 4); t = Tilemap.new(v); [t.viewport.equal?(v), Tilemap.new(nil).viewport, (t.viewport = nil), t.viewport]`
- **THEN** the result is `[true, nil, nil, nil]`

#### Scenario: Slots, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.bitmaps[9] = b; t.bitmaps[-1] = b; [t.bitmaps[9], t.bitmaps[-1], (t.bitmaps[3] = b; t.bitmaps[3].equal?(b)), (t.bitmaps[3] = nil; t.bitmaps[3])]` for a tilemap `t` and a bitmap `b`
- **THEN** the result is `[nil, nil, true, nil]`

#### Scenario: Wrong arguments, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.bitmaps[3] = 5`, `t.bitmaps["a"]`, `t.viewport = 5`, `Tilemap.new(5)`, `Tilemap.new(Viewport.new(0, 0, 8, 8), 1)`, `t.ox = "3"`, `t.ox = nil` and `t.ox = 2**31`
- **THEN** they raise `TypeError`, `TypeError`, `TypeError`, `TypeError`, `ArgumentError`, `TypeError`, `TypeError` and `RangeError`

#### Scenario: Plain properties, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.ox = 1.9; t.oy = -1.9; a = [t.ox, t.oy]; t.ox = 2**30 - 1; t.oy = 1e3; t.visible = nil; b = t.visible; t.visible = 5; a + [t.ox, t.oy, b, t.visible, t.update]`
- **THEN** the result is `[1, -1, 1073741823, 1000, false, true, nil]`

#### Scenario: No copies and no dump, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.dup`, `t.clone` and `Marshal.dump(t)`
- **THEN** each raises `TypeError`, the first two with the messages `can't dup Tilemap` and `can't clone Tilemap`

#### Scenario: XP and VX
- **WHEN** a game of RGSS1 and a game of RGSS2 evaluate `Tilemap.new.respond_to?(:flags)`, and the game of RGSS2 `Tilemap.new.respond_to?(:passages)`
- **THEN** the results are `false`, `false` and `true`

### Requirement: Tables of a tilemap of RGSS3
In RGSS3 `map_data=` and `flags=` SHALL take a `Table` of any dimensions and keep it by
reference, SHALL accept `nil` while the property never held a table, and once it holds one
SHALL raise `TypeError` with the message `can't convert NilClass into Table` for `nil` and
keep the table. `flash_data=` SHALL always accept `nil`. A value that is neither a `Table`
nor `nil` SHALL raise `TypeError`.

#### Scenario: Nil on a fresh tilemap, measured on the original
- **WHEN** a game of RGSS3 evaluates `t = Tilemap.new; t.flags = nil; t.map_data = nil; t.flash_data = nil; [t.flags, t.map_data, t.flash_data]`
- **THEN** no error is raised and the result is `[nil, nil, nil]`

#### Scenario: Nil once a table is set, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.flags = f; t.map_data = m` for two tables and then `t.flags = nil` and `t.map_data = nil`
- **THEN** each of the two raises `TypeError` with the message `can't convert NilClass into Table`, and `[t.flags.equal?(f), t.map_data.equal?(m)]` is `[true, true]`

#### Scenario: Flash data, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.flash_data = Table.new(2, 2); t.flash_data = nil; t.flash_data`
- **THEN** no error is raised and the result is `nil`

#### Scenario: Any dimensions, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.flags = Table.new(4, 4); t.map_data = Table.new(3); [t.flags.ysize, t.map_data.xsize]`
- **THEN** no error is raised and the result is `[4, 3]`

#### Scenario: Other values, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.flags = 5`, `t.map_data = 5` and `t.flash_data = "a"`
- **THEN** each raises `TypeError`

#### Scenario: Kept alive
- **WHEN** a game of RGSS3 assigns a new table to `flags` of a tilemap it keeps, keeps no other reference to the table, forces garbage collections and evaluates `t.flags[1]`
- **THEN** the value is the one the table was given before the collections

### Requirement: A tilemap of RGSS3 is drawn by the rules of RGSS3
`Tilemap.new` of a game of RGSS3 SHALL make a tilemap of the scene that is drawn by the rule
set of RGSS3: from a map of four layers, of which those that the table lacks are empty, from
the table of `flags`, and not at all while `visible` is false. A change made through the
`Table` object of `flags` or of `map_data` SHALL show in the next frame without `update`.

#### Scenario: Hidden, measured on the original
- **WHEN** a game of RGSS3 gives a tilemap a red sheet of 64 by 32 in slot 5 and a map of 1 by 1 by 4 with tile 1 in layer 0, and reads the pixel at (0, 0) at once, after `visible = false` and after `visible = true`
- **THEN** the pixels are red, black and red

#### Scenario: Star tile of the lowest layer, measured on the original
- **WHEN** the same tilemap gets a flags table of 8192 entries with `flags[1] = 0x10`
- **THEN** the pixel at (0, 0) is red

#### Scenario: Star tile of the third layer, measured on the original
- **WHEN** a game of RGSS3 gives a tilemap the red sheet, a map of 1 by 1 by 4 with tile 1 in layer 2 and a flags table of zeros, shows a green sprite of 32 by 32 at z 100, and reads the pixel at (0, 0) at once and after `flags[1] = 0x10` through the table object, without `update`
- **THEN** the pixels are green and red

#### Scenario: Shadow of the fourth layer, measured on the original
- **WHEN** the tilemap of the first scenario gets `map_data[0, 0, 3] = 1` through the table object, without `update`
- **THEN** the pixel at (0, 0) is `[126, 0, 0, 255]` and the pixels at (16, 0) and (0, 16) are red

#### Scenario: A map of three layers, measured on the original
- **WHEN** a game of RGSS3 gives a tilemap the red sheet and a map of 1 by 1 by 3 with tile 1 in layer 0
- **THEN** no error is raised and the pixels at (0, 0) and (16, 16) are red

#### Scenario: A map of fewer layers
- **WHEN** a game of RGSS3 gives a tilemap the red sheet and a map of 1 by 1 with tile 1, and then a map `Table.new(1)` with tile 1
- **THEN** no error is raised and the pixel at (0, 0) is red with each

#### Scenario: No flags, measured on the original
- **WHEN** the tilemap of the first scenario, whose `flags` was never set, is drawn, and again after `flags = Table.new(1)`
- **THEN** the pixel at (0, 0) is red both times

#### Scenario: VX
- **WHEN** a game of RGSS2 gives a tilemap the red sheet, a map of 1 by 1 by 3 with tile 1 in layer 0 and `visible = false`
- **THEN** the pixel at (0, 0) is red

### Requirement: Disposed tilemap of RGSS3
In RGSS3 every property and `update` of a disposed tilemap SHALL raise `RGSSError` with the
message `disposed tilemap`, while `disposed?`, `dispose` and `bitmaps` work and a slot reads
what it held. A write to a slot of the array of a disposed tilemap SHALL raise nothing and
SHALL be kept, where the original runtime ends with a page fault.

#### Scenario: Disposed, measured on the original
- **WHEN** a game of RGSS3 evaluates `t.bitmaps[1] = b; r = [t.disposed?, t.dispose, t.disposed?, t.dispose]` and then each of `t.ox`, `t.ox = 1`, `t.update`, `t.map_data`, `t.flags`, `t.flags = Table.new(1)`, `t.flash_data`, `t.visible`, `t.viewport` and `t.viewport = nil`
- **THEN** `r` is `[false, nil, true, nil]` and each of the ten raises `RGSSError` with the message `disposed tilemap`

#### Scenario: The array of a disposed tilemap, measured on the original
- **WHEN** the game then evaluates `[t.bitmaps.class, t.bitmaps[1].equal?(b), t.bitmaps[9]]`
- **THEN** the result is `[Tilemap::BitmapArray, true, nil]`

#### Scenario: A write after dispose
- **WHEN** the game then evaluates `t.bitmaps[2] = b; t.bitmaps[1] = nil; [t.bitmaps[2].equal?(b), t.bitmaps[1]]` and shows a frame
- **THEN** no error is raised, the result is `[true, nil]`, and the frame shows nothing of the tilemap

#### Scenario: Not drawn any more
- **WHEN** the tilemap of "Hidden" is disposed
- **THEN** the pixel at (0, 0) is black
