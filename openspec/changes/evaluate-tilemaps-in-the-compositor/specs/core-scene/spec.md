# Spec Delta

## ADDED Requirements

### Requirement: Tilemaps are extracted as tilemap ops
A tilemap SHALL be extracted as tilemap layer ops and not as bitmaps composed by the guest:
for priority 0 one op per map layer that has such tiles in view, at z 0; for each priority
from 1 to 5, one op per visible map row and map layer that has such tiles in that row, clipped
to the row's 32 screen rows and drawn at the row's z, within a priority in the order of the
map's layers. The tables SHALL be sent when they are first used and again only when their
content has changed.

#### Scenario: Same pictures as before
- **WHEN** the tilemap test scenes of this crate (scrolled, wrapped, with autotiles, priorities, translucent stacked tiles and a viewport) are extracted and rendered
- **THEN** every frame equals the one the cached-bitmap implementation produced, pixel for pixel

#### Scenario: Scrolling uploads nothing
- **WHEN** a tilemap's origin moves by 100 pixels over ten frames with unchanged tables and bitmaps
- **THEN** the packets after the first carry no table and no bitmap change for the tilemap

#### Scenario: Table change
- **WHEN** one cell of the map table is changed through the shared table
- **THEN** the next packet replaces that table and the frame shows the new tile

#### Scenario: Rows among sprites
- **WHEN** a sprite stands between two map rows that hold tiles of priority 1
- **THEN** the upper row's tiles are behind it and the lower row's in front, as before
