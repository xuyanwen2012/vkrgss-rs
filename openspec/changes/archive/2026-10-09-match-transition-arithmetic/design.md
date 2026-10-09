# Design

## Context

- Measurements: `local/measurements/README.md`, section "Graphics.transition", with
  `transition-samples.txt` and `transition-mask-samples.txt` (in this clone, ignored by git;
  selections are committed as test data). The original could only be observed at the step at
  which the call returns, step N-1 of N; the formulas for the other steps are the natural
  generalisation (PLACEHOLDER for steps other than N-1).
- The present pass reads the frames with `texelFetch` as integers and mixes before scaling,
  with a float weight and a mask formula taken from mkxp; a small mask is stretched.
- The session steps the progress as a fraction; the headless display produces dumps through
  the same pass.
- In the measurement runs vkxp printed, for every transition with a picture, that the
  transition's mask does not exist, and showed a cross-fade. The pictures existed in the game
  directory. The cause is not known.

## Decisions

- The presenter's interface takes integers: `an` for the cross-fade; `T`, vagueness and the
  mask for the masked form. The shader computes exactly the formulas of the spec in integer
  arithmetic; mask coordinates are frame pixel coordinates modulo the mask size.
- Mask luminance: `floor(0.2989 r + 0.5866 g + 0.1145 b)` in fixed point chosen so that it
  reproduces the committed grey and colour samples as far as the measurement says is possible;
  grey masks are exact.
- Find the missing-mask defect first with an end-to-end reproduction (a generated game, as in
  the scenario) and fix it where it is, guest or host.
- A vagueness below 0 is treated as 0.

## Follow-ups

- Exact luminance of coloured masks; steps other than N-1 cannot be measured with the present
  harness.
- Found while implementing. The missing-mask report was a false one: the guest disposes of
  the picture when `Graphics.transition` returns, the destroy delta travels with the next
  frame, and the session looked for the mask on that frame too, which is past the last step
  and shown alone. The frames inside the transition were masked all along.
- At step 0 the threshold is 0, so with a vagueness above 0 the pixels where the mask is
  exactly black already show the newest frame in the first frame of a transition. This is
  what the generalised formula gives; it could not be measured.
- The alpha of a mask pixel is not looked at. The measured masks were opaque.
- When the held frame has another size than the newest, the two are scaled separately and
  mixed in floating point with the same weights; only frames of one size are mixed exactly.
- The committed colour samples leave out the colours the luminance formula does not
  reproduce (1 of the 300 in the sample file).
