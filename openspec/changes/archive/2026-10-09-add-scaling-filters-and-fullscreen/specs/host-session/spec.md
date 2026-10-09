# Spec Delta

## ADDED Requirements

### Requirement: Initial window size and fullscreen
A windowed run SHALL open its window with a client area of the frame size times the largest
whole number at which it fits the monitor's work area, at least 1. Alt+Enter and F11 SHALL
toggle borderless fullscreen on the monitor the window is on, and leaving fullscreen SHALL
restore the earlier window size. Neither key SHALL reach the game as a button while it toggles.

#### Scenario: Size computation
- **WHEN** the frame is 640 by 480 and the work area is 1920 by 1040
- **THEN** the client area is 1280 by 960, and for a work area of 600 by 400 it is 640 by 480

#### Scenario: Toggle
- **WHEN** Alt+Enter is pressed twice
- **THEN** the window is fullscreen after the first and back at its earlier size after the second, and the snapshots of those frames have neither C nor ALT newly held by that key press
