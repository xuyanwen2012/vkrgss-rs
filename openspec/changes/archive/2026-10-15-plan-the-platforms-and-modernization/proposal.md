# Plan the platforms and the modernization

## Why

The modernization audit of 2026-10-14 (three read-only reviews: Vulkan backend; build, CI and
code; platform layer and presentation) listed six defects, twenty improvements for Linux,
platform work and ten questions for the owner. The six defects are fixed (0.3.1 and the three
changes archived on 2026-10-15). The owner answered the questions in a grilling on
2026-10-15. Nothing of it is recorded in the repository yet: `docs/decisions.md` still says
"Linux is the only platform supported now" and has no plan beyond VX Ace.

## What Changes

`docs/decisions.md` records the decisions of 2026-10-15:

- Platforms after Linux x86_64, in this order: ARM Linux (aarch64), macOS (Apple silicon),
  Android (aarch64), Windows (x86_64). Each is a milestone row P1 to P4 with its acceptance.
- Floats in saves are identical byte for byte across machines and platforms: both
  interpreters take their mathematical functions from a vendored correctly rounded libm.
- Vulkan 1.3 stays the floor; 1.4 features are used where the device has them, later.
- No HDR output. Native Wayland gains the content type "game", fractional scaling and
  server-side decorations; X11 stays.
- The package carries a SoundFont; users may bring their own.
- Linux packages: Flatpak and AUR beside the tarball.
- The merge gate of CI keeps the fast checks; the slow ones run nightly.
- The order of work before and between the platforms.
- Paid signing only for the macOS release; no store for macOS yet.

## Capabilities

### New Capabilities

- `supported-platforms`: which platforms are planned, in which order, and how each is
  accepted.

### Modified Capabilities

None.

## Impact

`docs/decisions.md` (goal and scope, platform defaults, milestones, known risks),
`docs/compatibility.md` (one sentence on planned platforms), `CHANGELOG.md` is not touched
(no change to the program). No code.
