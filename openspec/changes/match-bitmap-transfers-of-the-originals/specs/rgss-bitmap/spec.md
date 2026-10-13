## MODIFIED Requirements

### Requirement: Drawing methods
`clear`, `fill_rect` (with a rectangle and a colour, or with x, y, width, height and a colour),
`get_pixel`, `set_pixel`, `blt(x, y, source, source_rect, opacity = 255)`,
`stretch_blt(dest_rect, source, source_rect, opacity = 255)` and `hue_change(hue)` SHALL
perform the operation of the same name defined by `core-bitmap`. An argument of the wrong class
SHALL raise `TypeError`, and a disposed source bitmap SHALL raise nothing. The opacity SHALL
be passed on as the integer given.

#### Scenario: Fill and read
- **WHEN** `b = Bitmap.new(4, 4); b.fill_rect(1, 1, 2, 2, Color.new(255, 0, 0)); b.fill_rect(Rect.new(0, 0, 1, 1), Color.new(0, 255, 0, 128))` is evaluated
- **THEN** `b.get_pixel(1, 1)` equals `Color.new(255, 0, 0, 255)`, `b.get_pixel(0, 0)` equals `Color.new(0, 255, 0, 128)` and `b.get_pixel(3, 3)` equals `Color.new(0, 0, 0, 0)`

#### Scenario: Transfer
- **WHEN** a 2 by 2 red bitmap is transferred with `blt(1, 1, source, source.rect)` into a 4 by 4 transparent bitmap
- **THEN** pixels (1, 1) to (2, 2) of the destination are red and the others transparent

#### Scenario: Wrong class
- **WHEN** `b.blt(0, 0, "not a bitmap", Rect.new(0, 0, 1, 1))` is evaluated
- **THEN** a `TypeError` is raised

#### Scenario: Disposed source, measured on the originals
- **WHEN** `s.dispose; b.blt(0, 0, s, Rect.new(0, 0, 2, 2))` is evaluated on a bitmap `b` whose pixels have alpha 128
- **THEN** no error is raised and the four pixels from (0, 0) have alpha 191 with their colour unchanged

#### Scenario: Opacity above 255, measured on the originals
- **WHEN** `b.blt(0, 0, source, source.rect, 256)` and `b.blt(0, 0, source, source.rect, 255.9)` are evaluated
- **THEN** the first changes nothing and the second draws at opacity 255
