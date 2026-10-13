## ADDED Requirements

### Requirement: Tilemap of RGSS2 in the scene
A tilemap of RGSS2 SHALL be extracted as two plane ops among the elements of its viewport, the
lower at z 0 and the upper at z 200, from the origin `ox + viewport.ox`, `oy + viewport.oy`
with the map wrapped, whatever its `visible` is. Its counter SHALL advance only by its
update, giving the animation step `counter / 30` and the flash level
`4 + [0, 1, 2, 3, 4, 3, 2, 1][(counter / 4) % 8]`.

#### Scenario: Two planes, measured on the original
- **WHEN** a tilemap with a star tile in layer 2 shares a viewport with sprites at z 100 and z 300
- **THEN** the first sprite is under the star tile and over the other tiles, and the second is over both

#### Scenario: Invisible, measured on the original
- **WHEN** `visible = false` is set on a tilemap
- **THEN** the next frame shows the tilemap as before

#### Scenario: Counter, measured on the original
- **WHEN** a tilemap is updated 29 times, then once more, with frames in between that do not update it
- **THEN** the animation step is 0 until the thirtieth update and 1 after it

#### Scenario: Scrolling
- **WHEN** ox is -40 on a map 5 cells wide
- **THEN** the first cell is 3 and the plane's corner is 24 pixels left of the viewport's corner

### Requirement: Latched viewport origin of a tilemap of RGSS2
A tilemap of RGSS2 SHALL take the ox and oy of its viewport as they were at its last refresh:
a write of `ox`, `oy`, `map_data`, `flash_data`, `passages` or a bitmap slot, an update, or a
change of the viewport's ox or oy; assigning another viewport SHALL NOT refresh.

#### Scenario: Other viewport, measured on the original
- **WHEN** a tilemap in a viewport with ox 0 is assigned a viewport with ox 16 and a frame is taken, then `ox = ox` is evaluated and another is taken
- **THEN** the first frame shows the map at the origin of the old viewport and the second moved by 16
