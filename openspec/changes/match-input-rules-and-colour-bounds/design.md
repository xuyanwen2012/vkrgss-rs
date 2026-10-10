# Design

## Measured rules

All counting is per `Input.update`. Button numbers are those of the `Input` constants (DOWN 2,
LEFT 4, RIGHT 6, UP 8, A 11, B 12, C 13, X 14, Y 15, Z 16, L 17, R 18, SHIFT 21, CTRL 22,
ALT 23, F5 to F9 25 to 29).

### Repeat

State: `rep_button` (none at start) and `rep_count` (0).

    Input.update, after the snapshot is latched:
      for each button in ascending number:
        if it is triggered at this update: rep_button = it; rep_count = 0
      if rep_button is held: rep_count += 1
      else:                  rep_button = none; rep_count = 0
    repeat?(b) = b == rep_button and (rep_count == 1 or (rep_count >= 16 and (rep_count - 16) % 4 == 0))

So one button held alone repeats on its 1st, 16th, 20th, 24th update, as before. A second
button that is triggered takes the repeat away, and the first never gets it back while it
stays held, not even after the second is released. Of buttons triggered in the same update
only the one with the highest number repeats (`trigger?` is true for all of them).

| update | held | repeat? |
|---|---|---|
| 1 | DOWN | DOWN |
| 2..7 | DOWN | none |
| 8 | DOWN, LEFT | LEFT |
| 9..22 | DOWN, LEFT | none (DOWN's 16th update is 16: no repeat) |
| 23, 27, 31 | DOWN, LEFT | LEFT |

| update | held | repeat? |
|---|---|---|
| 1 | DOWN | DOWN |
| 2 | DOWN, LEFT | LEFT |
| 3 | DOWN | none |
| 4..40 | DOWN | none |

| held from the same update | repeat? at that update |
|---|---|
| DOWN, LEFT | LEFT |
| LEFT, UP | UP |
| B, C | C |

### Directions

State: `axis_flag`, 0 at start, kept for the life of the state (a release of everything does
not clear it).

    dx = (RIGHT held) - (LEFT held);  dy = (DOWN held) - (UP held)      # opposites cancel
    dir8 = 0 for dx == 0 and dy == 0, else the keypad number of (dx, dy)
    dir4, computed in Input.update:
      if dx != 0 and dy != 0:  axis_flag == 0: horizontal wins;  axis_flag == 1: vertical wins
      else if dx != 0:         axis_flag = 1
      else if dy != 0:         axis_flag = 0
      (else unchanged)

`dir8` is what vkxp does today. `dir4` replaces the comparison of trigger times.

| step | held | dir4 | dir8 | axis_flag after |
|---|---|---|---|---|
| 1 | DOWN | 2 | 2 | 0 |
| 2 | DOWN, LEFT | 4 | 1 | 0 |
| 3 | LEFT | 4 | 4 | 1 |
| 4 | DOWN, LEFT | 2 | 1 | 1 |
| 5 | none | 0 | 0 | 1 |
| 6 | LEFT, UP (together) | 8 | 7 | 1 |
| 7 | UP | 8 | 8 | 0 |
| 8 | none | 0 | 0 | 0 |
| 9 | RIGHT, UP (together) | 6 | 9 | 0 |

| step | held | dir4 | dir8 |
|---|---|---|---|
| 1 | DOWN | 2 | 2 |
| 2 | DOWN, UP | 0 | 0 |
| 3 | DOWN, UP, LEFT | 4 | 4 |
| 4 | UP, LEFT | 8 | 7 |
| 5 | LEFT | 4 | 4 |

Three keys give the one that is not cancelled, four give 0. The value of the flag before the
first single-axis input of a process was not measured; 0 fits every sample.

### Colour bounds

| expression | components |
|---|---|
| `Color.new(300, -300, 0.1, -7)` | 255.0, -255.0, 0.1, 0.0 |
| `c.red = 1000; c.green = -1000; c.alpha = -3.5` | 255.0, -255.0, 0.0 |

A tone already has these bounds. What a negative component does in a pixel operation
(truncated toward zero and taken modulo 256 by `set_pixel` and `fill_rect`) is measured and
implemented already and does not change.

## Decisions

- A reset of the input state (F12) starts `rep_button`, `rep_count` and `axis_flag` afresh,
  as the rest of the state does.
- `xtask/src/oracle/probe.rb` computes `repeat?` and `dir4` by the same rules, so a schedule
  that holds two buttons means the same on both sides as it would with the real module.
- Confirmed notes lose the word `PLACEHOLDER` and say "measured on the original runtime"
  instead; no behaviour changes there.

## Follow-ups

(none yet)
