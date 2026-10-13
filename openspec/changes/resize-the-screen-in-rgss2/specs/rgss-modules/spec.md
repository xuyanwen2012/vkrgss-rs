## ADDED Requirements

### Requirement: Resizing the screen
In RGSS2 `Graphics.resize_screen(width, height)` SHALL set the size of the frame to the two
integers, the width limited to 1 to 640 and the height to 1 to 480, SHALL return `nil` and
SHALL send no frame. The following frame messages, `Graphics.width`, `Graphics.height` and
`Graphics.snap_to_bitmap` SHALL have the new size. In RGSS1 the function SHALL NOT be defined.

#### Scenario: Limits, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.resize_screen(w, h); [Graphics.width, Graphics.height]` for (641, 481), (1000, 1000), (0, 0), (-1, -1), (100, 50) and (640, 1)
- **THEN** the results are `[640, 480]`, `[640, 480]`, `[1, 1]`, `[1, 1]`, `[100, 50]` and `[640, 1]`

#### Scenario: Frames and snapshot, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.resize_screen(320, 240)`, takes a snapshot and calls `Graphics.update`
- **THEN** the snapshot is 320 by 240, the frame count rose by one, and the frame message carries the logical size 320 by 240

#### Scenario: Arguments, measured on the original
- **WHEN** a game of RGSS2 evaluates `Graphics.resize_screen(640)`, `Graphics.resize_screen(nil, 480)` and `Graphics.resize_screen(100.9, 50.9)`
- **THEN** the first raises `ArgumentError`, the second `TypeError`, and after the third the size is 100 by 50

#### Scenario: Dump after a resize
- **WHEN** a generated game of RGSS2 with a red sprite at (600, 440) calls `Graphics.update`, `Graphics.resize_screen(640, 480)` and `Graphics.update`, headless with both frames dumped
- **THEN** the first picture is 544 by 416 without red and the second 640 by 480 with red at (600, 440)

#### Scenario: XP
- **WHEN** a game of RGSS1 evaluates `Graphics.resize_screen(640, 480)`
- **THEN** `NoMethodError` is raised
