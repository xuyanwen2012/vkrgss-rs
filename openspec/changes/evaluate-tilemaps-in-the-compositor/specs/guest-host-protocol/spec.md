# Spec Delta

## ADDED Requirements

### Requirement: Table resources
A frame packet SHALL be able to create, replace and destroy tables: arrays of 16-bit signed
integers with one to three dimensions, identified by numbers of their own, applied in order
before any op is drawn, like bitmap changes. An op that names a table the host does not have
SHALL make the packet fail with a typed error and produce no frame.

#### Scenario: Round trip
- **WHEN** a packet that creates a 20 by 15 by 3 table and a 400-entry table is serialised and read back
- **THEN** it equals the original

#### Scenario: Unknown table
- **WHEN** a tilemap op names a table that was never created
- **THEN** the backend returns a typed error and produces no frame

### Requirement: Tilemap layer op
The tilemap layer op SHALL be a plain-data op with a clip rectangle, the tileset bitmap, up to
seven autotile bitmaps, the table of tile numbers, the table of priorities, the scroll origin,
the map layer from 0 to 2, the priority from 0 to 5 that it draws, and the autotile animation
step. The protocol version SHALL be 6, and the op SHALL no longer be a reserved kind.

#### Scenario: Round trip
- **WHEN** a packet with a tilemap layer op is serialised and read back
- **THEN** the op equals the original

#### Scenario: Version
- **WHEN** the protocol version constant is read
- **THEN** it is 6
