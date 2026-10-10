# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Transfers within one bitmap work in place
A block transfer or stretched block transfer whose source and destination are the same bitmap
SHALL write the destination rows from top to bottom and the pixels of a row from left to
right, reading each source pixel as the bitmap is at that moment, pixels already written by
the same transfer included.

#### Scenario: Measured transfers
- **WHEN** each committed case measured on the original runtime (shifts in every direction, both opacities, stretched to the same size, to twice and to half the size, a mirrored transfer) is carried out
- **THEN** every pixel of the bitmap equals the measured one
