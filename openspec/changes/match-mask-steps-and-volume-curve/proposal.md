# Proposal

## Why

Two `PLACEHOLDER` rules were settled against the original runtime. The intermediate steps of
a transition were watched from outside the game window while the transition ran (22
transitions, 407040 pixels, no mismatch with the rule below): the plain cross-fade is what
vkxp does, but with a transition picture the threshold of the intermediate steps is another
formula, the first step shows the old picture untouched, and an omitted vagueness is 20,
not 40. The volume of a sound is not a linear factor: the original hands the audio back end
an attenuation of 0.35 dB for every unit of volume below 100 (read from the code of
RGSS104E), so a volume of 80 is far quieter than vkxp plays it.

## What Changes

- The session computes the mask threshold of step `s` as `256 * s / N - 1` and shows the
  held frame alone at step 0.
- `Graphics.transition` without a vagueness passes 20.
- The mixer turns volume and fade into a gain in decibels.

Out of scope: the mix under the mask in the presenter (measured before, unchanged); the
second ramp of the original that brings music back after a music effect; how a volume
outside 0 to 100 is treated, which stays as it is.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `host-session`: transition step values.
- `rgss-modules`: the default vagueness.
- `audio-playback`: the volume curve.

## Impact

- `crates/vkxp-host` (`session.rs` and its tests), `crates/vkxp-binding` (`graphics.rs` and
  its tests), `crates/vkxp-audio` (`mixer.rs` and its tests), the README where it states the
  old rules.
