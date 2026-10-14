## ADDED Requirements

### Requirement: Reset functions of RGSS3
In a game of RGSS3 `Graphics.__reset__` and `Audio.__reset__` SHALL take no argument and
return `nil`. The first SHALL dispose every `Bitmap`, `Sprite`, `Viewport`, `Window`,
`Plane` and `Tilemap`, set the brightness to 255 and the frame rate to 60 and end a freeze,
and SHALL leave the frame count, the size of the screen, the values of `Font` and the audio
as they are. The second SHALL stop the audio of all four channels.

#### Scenario: Arity and value, measured on the original
- **WHEN** a script of a game of RGSS3 evaluates `[Graphics.method(:__reset__).arity, Graphics.__reset__, Audio.method(:__reset__).arity, Audio.__reset__]`
- **THEN** the result is `[0, nil, 0, nil]`

#### Scenario: What is disposed, measured on the original
- **WHEN** a script makes a free `Bitmap`, a `Sprite` of a subclass with a bitmap, a `Viewport`, a `Window` with a skin and contents, a `Plane` and a `Tilemap`, calls `Graphics.__reset__` and asks each for `disposed?`
- **THEN** every answer is `true`, the sprite still returns its bitmap, and `sprite.x` raises `RGSSError` with the message `disposed sprite`

#### Scenario: What is set back, measured on the original
- **WHEN** a script sets `Graphics.brightness = 100` and `Graphics.frame_rate = 120`, calls `Graphics.freeze` and then `Graphics.__reset__`
- **THEN** `Graphics.brightness` is 255 and `Graphics.frame_rate` is 60, and the display is not frozen: the frame of the next `Graphics.update` is shown

#### Scenario: What stays, measured on the original
- **WHEN** a script sets `Graphics.frame_count = 1000`, calls `Graphics.resize_screen(320, 240)`, sets `Font.default_size = 30`, keeps a `Color` read from a sprite and calls `Graphics.__reset__`
- **THEN** `Graphics.frame_count` is 1000, `[Graphics.width, Graphics.height]` is `[320, 240]`, `Font.default_size` is 30 and the `Color` has its values

#### Scenario: Audio, measured on the original
- **WHEN** a script plays a music, a background sound, a music effect and a sound effect and calls `Audio.__reset__`
- **THEN** the host receives a stop for each of the four channels, and `Graphics.__reset__` alone sends none

#### Scenario: Redefined by a script, measured on the original
- **WHEN** a script redefines `Audio.__reset__` and `Graphics.__reset__` to append their names to a global array and the block of `rgss_main` of the stand-in raises `RGSSReset` once
- **THEN** the array is `["Audio", "Graphics"]`

#### Scenario: Games of RGSS1 and RGSS2
- **WHEN** a script of a game of RGSS1 or of RGSS2 evaluates `Graphics.respond_to?(:__reset__)` over Ruby 1.8.7
- **THEN** the result is `false`
