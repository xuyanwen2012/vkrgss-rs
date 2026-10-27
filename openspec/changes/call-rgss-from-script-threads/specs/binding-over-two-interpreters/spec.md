## ADDED Requirements

### Requirement: RGSS classes from script threads
Over Ruby 1.9.2 the RGSS classes SHALL behave when called from a script thread as the original
runtime measured for this change does, and SHALL NOT fail because the caller is not the main
script's thread. Objects collected while a script thread runs SHALL be freed as those collected
on the main script's thread are.

#### Scenario: A bitmap and a sprite in a thread
- **WHEN** a game of VX Ace evaluates `Thread.new { b = Bitmap.new(8, 8); b.fill_rect(0, 0, 4, 4, Color.new(255, 0, 0)); s = Sprite.new; s.bitmap = b; [b.width, Graphics.brightness] }.value`
- **THEN** the result is `[8, 255]`, as on the original, and no exception is raised

#### Scenario: Collection in a thread
- **WHEN** a script thread of a game of VX Ace makes and drops a hundred bitmaps and sprites and runs `GC.start`, and the main script then calls `Graphics.update`
- **THEN** the arena holds none of those bitmaps and the scene none of those sprites
