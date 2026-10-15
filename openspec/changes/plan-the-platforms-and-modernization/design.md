# Design

## Context

Source: the audit page of 2026-10-14 and the grilling with the owner on 2026-10-15 (four
rounds, Q1 to Q26). This file is the record of the answers; `docs/decisions.md` takes them
over when the change is applied.

## Decisions

### Platforms (Q1, Q11, Q11b)

Order: P1 ARM Linux, P2 macOS, P3 Android, P4 Windows. Each platform is its own set of
changes; finishing one does not commit to the next. Windows is last; the owner has no
Windows machine yet.

### ARM Linux, P1 (Q19, Q20, Q23)

- Acceptance: on aarch64, the frames of the seven XP games and of the VX and VX Ace games of
  the corpus at the compared frame numbers are identical byte for byte to those of x86_64,
  and a save written at the same frame is identical byte for byte. The original runtime
  cannot run there, so x86_64 is the reference.
- Both interpreters get a verified aarch64 `config.h` (1.8.7 borrows the x86_64 one today);
  the collector stress runs on aarch64.
- CI: the job `gate-aarch64` on GitHub's hosted `ubuntu-24.04-arm` with lavapipe becomes
  required. No self-hosted runner on the owner's devices.
- Hardware: the two Jetson Orin Nano (`duck-stable`, `duck-naughty`; JetPack 7.2, NVIDIA
  Vulkan driver, headless) may be used at any time for tests, never as runners.

### macOS, P2 (Q9, Q21, Q24)

- Apple silicon, MoltenVK. MoltenVK has no present timing; pacing falls back to the
  timer. Portability enumeration and subset; the bitmap slot limit taken from the device.
- Tests only on the owner's MacBook Air, by hand, now and then, and only after asking the
  owner each time. No CI runner for macOS, hosted or self-hosted.
- Release: a Developer ID (99 USD a year) is bought when the macOS release is made; the
  build is signed and notarized. Not in the Mac App Store for now.
- Prerequisite: the guest of RGSS1 and RGSS2 becomes a program of its own instead of the
  host executing itself (audit item 20), so that no window framework is linked before fork.

### Android, P3 (Q12, Q13, Q16, Q17, Q22)

- Starts with a feasibility measurement, like V0: the interpreters on bionic, a separate
  guest process under seccomp, Vulkan 1.3 on the test phones. The scope is fixed after it.
- First version: games are imported as a zip into the app's private directory (no storage
  access framework); input by on-screen buttons and gamepad; no save states.
- Vulkan 1.3 required, as on the desktop; no 1.1 path.
- Guest: a separate process with seccomp; Landlock is absent from most Android kernels, the
  private directory isolates the files.
- Distribution: APK on GitHub releases, installable through Obtainium; self-signed. Google
  Play may come later.
- Test devices: Pixel 7a (Tensor G2, Mali-G710) and Galaxy S24+ SM-S926B (Exynos 2400,
  Xclipse 940), both on USB at the owner's desktop, free to use at any time; the Android
  emulator for routine tests.

### Windows, P4 (Q2, Q3)

- Save states (F3, F4) are never supported on Windows: the interpreter's state cannot be
  serialized and emulating fork is not supported by the system.
- Whether the guest is a 32-bit process (Ruby 1.8.7 needs `long` as wide as a pointer) is
  decided when Windows work starts.

### Floats in saves (Q4, Q15)

Saves are identical byte for byte across machines and platforms. Both interpreters take
`Math` and `Float#**` from CORE-MATH, vendored as C (MIT), correctly rounded. A pure Rust
correctly rounded library would be preferred, but none covers the functions with a
guarantee today. Before vendoring, check that CORE-MATH covers every function Ruby's `Math`
uses (including `lgamma`, `tgamma`, `erf`, `erfc`); a gap is decided then. Where the oracle
shows that a game depends on a result of the original that differs from the correctly
rounded one, that function alone is matched to the original.

### Vulkan (Q5)

Vulkan 1.3 is the floor. 1.4 features (push descriptors, maintenance5 and 6) are used where
the device has them, once `ash` has the bindings or a feature is needed; no hand-written
bindings for 1.4 now. Requiring 1.4 is rejected.

### Display (Q6, Q14)

No HDR output. Native Wayland (winit already uses it) gains: content type "game"
(variable refresh, lower latency), fractional scaling (`wp_fractional_scale_v1`) and
server-side decorations (`xdg-decoration`). X11 stays supported.

### Audio (Q7)

The package carries GeneralUser GS with its licence as the default SoundFont; a user's SF2
replaces it; DLS (`gm.dls`) support may follow.

### Linux packages (Q8)

Flatpak (Flathub) and AUR beside the tarball, which is built against an older glibc.
Flatpak only if the guest's Landlock and seccomp work under Flatpak's own seccomp filter;
otherwise AUR alone.

### CI (Q10)

The merge gate of CI keeps the fast checks (about ten minutes, parallel jobs); the fast part
of the collector stress stays in it. The rest of the stress, the sanitizers and fuzzing run
in the nightly workflow, which opens an issue when it fails. `cargo xtask gate` stays one
command that runs everything locally.

### Order of work (Q25)

1. `fuzz-and-review-ruby-1-9-2` (proposed).
2. Correctly rounded libm.
3. CI split into parallel jobs.
4. The guest of RGSS1 and RGSS2 as a program of its own.
5. P1 ARM Linux, then P2 macOS, then P3 Android.

The Vulkan items of the audit (synchronization validation, `vkQueueSubmit2` with timeline
semaphores, batched upload barriers, swapchain maintenance as KHR, timing properties and
VRR) and the Wayland items go in between; they do not depend on a platform.

## Out of scope

Implementing any of it. This change records decisions only.

## Follow-ups

None yet.
