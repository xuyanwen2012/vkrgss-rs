# Tidy the states and the leftovers of the measured rules

## Why

The changes of 2026-10-10 left five small things behind, each noted by a reviewer or found
while checking the result on a real game:

- A notice about states stays in the window title for as long as the game sends no frame,
  because the window loop wakes only for a pending tick.
- The answer `StateFailed` does not say what failed. The session guesses from whether it
  waits for a load of that slot, so a failed save that arrives while a load of the same slot
  waits is taken for the failed load, and the ticks go on before the state is loaded.
- The directory of the guest's socket stays behind when `vkxp` is ended by a signal.
- `Tilemap::strips` takes a parameter it no longer uses.
- The comment on `Sprite::bush_depth` describes the rule that the measurement replaced.

## What Changes

- The session reports the next time at which it has something to do, a pending tick or the
  end of a notice, and the window loop waits until then.
- **BREAKING** (protocol): `StateFailed` says whether a save or a load failed; the protocol
  version becomes 9. The session acts on that instead of guessing.
- Starting a guest process removes the socket directories that earlier runs of the same user
  left behind and whose process no longer exists.
- `Tilemap::strips` loses its parameter; the comment on `Sprite::bush_depth` states the
  measured rule.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `guest-host-protocol`: the failed answer names what failed; version 9.
- `host-session`: notices end on time without frames; failed answers are told apart.
- `game-launch`: socket directories of ended runs are removed.

## Impact

`vkxp-proto` (message, codec, fixtures that carry the version), `vkxp-binding`, `vkxp-host`
(session, windowed loop, demo guest, tests), `vkxp` (process relay), `vkxp-core` (one
signature, one comment), README. No picture changes.
