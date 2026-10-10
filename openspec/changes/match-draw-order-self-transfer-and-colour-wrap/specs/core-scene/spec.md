# Spec Delta

## MODIFIED Requirements

### Requirement: Draw order
Elements SHALL be drawn in ascending order of z, so that an element with a larger z appears in
front. Among elements with equal z, the one that was created more recently SHALL appear in
front; assigning a z does not change the order among equals. A viewport takes its place among
the top-level elements by its own z and its own creation.

#### Scenario: Larger z in front
- **WHEN** two overlapping opaque sprites have z 1 and z 2
- **THEN** the overlap shows the sprite with z 2, whichever was created first

#### Scenario: Equal z
- **WHEN** two overlapping opaque sprites have the same z
- **THEN** the overlap shows the one created later

#### Scenario: Moving to an occupied z
- **WHEN** the earlier-created of two sprites is moved to another z and then back to the shared z
- **THEN** the overlap still shows the later-created sprite

#### Scenario: Setting the same z again
- **WHEN** the earlier-created of two sprites with equal z has its z set to the value it already has
- **THEN** the overlap still shows the later-created sprite

#### Scenario: Measured orders
- **WHEN** each row of the unit-test table of the design (sprites, a plane, windows, viewports and a tilemap, with z taken away and back) is built and extracted
- **THEN** the elements are drawn in the order the table gives for the original

#### Scenario: Viewport older than a top-level sprite
- **WHEN** a viewport is created, then a top-level sprite, then a sprite in the viewport, all with z 0
- **THEN** the top-level sprite is drawn over the sprite in the viewport
