# Design

## Measured rules

### Transition steps

A transition of duration N shows N pictures, steps `s = 0 .. N-1`, one per frame; the new
picture alone appears with the next frame after it. Step 0 is exactly the held picture, with
and without a mask.

Plain: `an = 255 * s / N` (integer division), as today.

With a mask of level L at the pixel, for `s >= 1`:

    T = 256 * s / N - 1          (integer division)

and the presenter's mix with that T and the vagueness is unchanged. At the last step this T
equals the `(255 * s - 1) / N` used until now for every N from 1 to 2000, which is why the
earlier measurement, which could only see the last step, did not show the difference.

| N | T at s = 1, 2, 3, ... |
|---|---|
| 2 | 127 |
| 3 | 84, 169 |
| 5 | 50, 101, 152, 203 |
| 8 | 31, 63, 95, 127, 159, 191, 223 |
| 20 | 11, 24, 37, 50, 63, 75, 88, 101, 114, 127, 139, 152, 165, 178, 191, 203, 216, 229, 242 |
| 50 | 4, 9, 14, 19, 24, 29, 34, 39, 45, 50 (first ten) |

For N above 256 the formula gives -1 at the first steps; durations above 50 were not
measured. The presenter takes a threshold from 0 to 255, so the session floors T at 0 there.

An omitted vagueness is 20: with `Graphics.transition(5, mask)` the band between the old and
the new picture spans 20 mask levels.

### Volume

The original computes, in integers, `v = volume * fade / 100` with `fade` running from 100
down to 0 during a fade, and gives the back end an attenuation of `-35 * (100 - v)`
hundredths of a decibel, or silence for `v = 0`. As a gain:

    gain = 0                           for v = 0
    gain = 10 ^ (-0.0175 * (100 - v))  otherwise

| volume | gain |
|---|---|
| 100 | 1.000 |
| 90 | 0.668 |
| 80 | 0.447 |
| 50 | 0.133 |
| 20 | 0.0398 |
| 1 | 0.0185 |
| 0 | 0 |

This was read from the code of RGSS104E, not measured by ear or by recording.

## Decisions

- Step 0 of a masked transition is given to the display as a mix that shows the held frame
  alone, by whatever value of `Mix` already means that; the presenter does not change.
- The mixer applies the curve to the product of volume and fade, so a fade is even in
  decibels: half-way through a fade of a track at volume 100 the gain is that of volume 50.
  The fade factor is taken as the continuous fraction of the fade time, not rounded to
  whole percent as the original's timer does: the steps of one percent are a property of its
  timer, not something a game can observe.

## Follow-ups

(none yet)
