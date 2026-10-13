## MODIFIED Requirements

### Requirement: Disposal
After a bitmap is disposed, every operation on it other than disposing it again and asking
whether it is disposed SHALL fail with a disposed error and SHALL change nothing. Using a
disposed bitmap as the source of a transfer SHALL NOT fail: every pixel of the destination
rectangle inside the destination SHALL be blended onto itself at the given opacity by the
arithmetic of the block transfer.

#### Scenario: Disposing twice
- **WHEN** a disposed bitmap is disposed again
- **THEN** nothing happens and no error is reported

#### Scenario: Drawing after disposal
- **WHEN** a pixel of a disposed bitmap is written
- **THEN** the operation fails with a disposed error

#### Scenario: Disposed source
- **WHEN** a disposed bitmap is used as the source of a block transfer at opacity 255 onto pixels of alpha 0, 128 and 255
- **THEN** no error is reported, the colours are unchanged and the alphas are 0, 191 and 255

#### Scenario: Samples of a disposed source
- **WHEN** each committed sample of a transfer from a disposed source measured on the originals is evaluated
- **THEN** every channel of every pixel equals the measured value

### Requirement: Block transfer
A block transfer SHALL draw a rectangle of a source bitmap at a position in the destination with
an opacity from 0 to 255, combining each source pixel with the destination pixel by straight-alpha
source-over. Only the part of the source rectangle that lies inside the source bitmap SHALL be
drawn, at the offset it has within the rectangle, and only where it lands inside the destination.

#### Scenario: Opaque copy
- **WHEN** a fully opaque source rectangle is transferred at opacity 255
- **THEN** the destination pixels it covers equal the source pixels

#### Scenario: Zero opacity
- **WHEN** a rectangle is transferred at opacity 0
- **THEN** the destination is unchanged

#### Scenario: Onto a transparent destination
- **WHEN** a source pixel with alpha 128 is transferred at opacity 255 onto a transparent pixel
- **THEN** the destination pixel has the source's red, green and blue and alpha 128

#### Scenario: Source rectangle partly outside the source
- **WHEN** the source rectangle starts 2 pixels left of the source bitmap
- **THEN** its first two columns draw nothing and the third column lands 2 pixels right of the given position

#### Scenario: Source and destination are the same bitmap
- **WHEN** the row of pixels `a b c d e f` is transferred within its bitmap from columns 0 to 3 to columns 1 to 4 at opacity 255
- **THEN** the row is `a a a a a f`, and a transfer from columns 1 to 4 to columns 0 to 3 of the same row gives `b c d e e f`

### Requirement: Colour components stored as in the original
A colour component written by `fill_rect`, `set_pixel` or text drawing SHALL be truncated
toward zero and taken modulo 256 for red, green and blue, and an alpha below 0 SHALL be stored
as 0. `Color` SHALL limit red, green and blue to -255 to 255 and alpha to 0 to 255 on
construction and assignment, and `Tone` SHALL keep its limits.

#### Scenario: Negative red
- **WHEN** `Color.new(-5, 0, 0)` is created and written with `set_pixel`
- **THEN** the colour's red is `-5.0` and the stored pixel's red is 251

#### Scenario: Upper bound
- **WHEN** `Color.new(300, -5, 0)` and `Tone.new(300, -300, 0)` are created
- **THEN** their first two components are 255 and -5, and 255 and -255

#### Scenario: Lower bound, measured on the originals
- **WHEN** `Color.new(-300, 0, 0, -5)` is created and its red is then set to -1000
- **THEN** its red is `-255.0` both times and its alpha `0.0`

## ADDED Requirements

### Requirement: Opacity limit and mirrored clip of a transfer
A transfer with an opacity above 255 SHALL draw nothing. A transfer of equal sizes with a
negative source width or height SHALL be clipped before it is mirrored: per axis the low end
of the source span is cut at 0 and the destination span moved with it, then the destination is
cut at 0, then the source at its size, then the destination at its size, and what remains is
drawn mirrored within itself.

#### Scenario: Opacity above 255, measured on the originals
- **WHEN** a rectangle is transferred at opacity 256, 300 and 65536
- **THEN** the destination is unchanged each time

#### Scenario: Mirrored and cut, measured on the originals
- **WHEN** each committed sample of a mirrored transfer of equal sizes whose source rectangle leaves the source bitmap is evaluated, along each axis and along both
- **THEN** every pixel equals the measured value
