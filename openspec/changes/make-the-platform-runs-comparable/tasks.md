# Tasks

## 1. Oracle file times

- [ ] 1.1 Times of the copy and of the probe's files; test. Verify: scenario "Times of the copy" (unit test on Linux); `ref/tools/oracle/regress.sh` comparisons on x86 still pass, tower's new summary recorded.

## 2. Android options and fonts

- [x] 2.1 `font_own_metrics`; the Android families in the substitution lists. Verify: scenario "Arial on Android" on both phones; the text tests pass on Linux.

## 3. The strip

- [x] 3.1 Find and fix the white strip of `mh` frame 599 on Android. Verify: the frame is identical to x86 on both phones; the cause and the fix recorded in `design.md`.

## 4. xarch command

- [x] 4.1 `cargo xtask xarch run|android|compare`, the local game list, docs. Verify: scenario "All platforms" for this machine and both phones; the Mac run is done at the merge by the owner's session (say so).

## 5. Integration check

- [ ] 5.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate make-the-platform-runs-comparable --strict` passes.
