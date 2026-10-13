## ADDED Requirements

### Requirement: Sprite properties of RGSS2
In RGSS2 `Sprite` SHALL define `bush_opacity` (default 128), `wave_amp` (0), `wave_length`
(180) and `wave_speed` (360) as integers kept as written without a limit, and `wave_phase`
(0.0) as a Float kept modulo 360 in [0, 360); `bitmap=` SHALL reset `src_rect` only for
another bitmap object. In RGSS1 none of the five SHALL be defined.

#### Scenario: Kept as written, measured on the original
- **WHEN** a game of RGSS2 sets `bush_opacity = 300`, `wave_amp = -1000` and `wave_length = 0`, reading each
- **THEN** the values are 300, -1000 and 0

#### Scenario: Phase, measured on the original
- **WHEN** a game of RGSS2 sets `wave_phase` to 370.5, -1 and 1000, reading it after each
- **THEN** the values are 10.5, 359.0 and 280.0

#### Scenario: Same bitmap, measured on the original
- **WHEN** a game of RGSS2 evaluates `s.bitmap = b; s.src_rect.set(1, 2, 3, 4); s.bitmap = b; s.src_rect`
- **THEN** the rectangle is still (1, 2, 3, 4)

#### Scenario: Not pending any more
- **WHEN** the method test of RGSS2 runs
- **THEN** the five properties are checked and are not in its pending list

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Sprite.new.respond_to?(:wave_amp)`
- **THEN** the result is `false`
