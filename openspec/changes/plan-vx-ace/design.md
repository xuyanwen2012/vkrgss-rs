## Context

Two studies of 2026-10-13, kept outside the repository with their tools and logs, are the
input (a flow reads them at `../../ref/tools/oracle/measurements/ace-v0-REPORT.md` and
`../../ref/tools/ruby192-feasibility/REPORT.md`):

- The original runtime of VX Ace under Wine. It starts, a probe script can be injected
  (loose `Scripts.rvdata2` and inside `Game.rgss3a`), frames shown by `update`, `wait`,
  `fadeout` and `fadein` are captured exactly (not inside a transition, as with VX), input
  is given at chosen frames, and repeated runs with a fixed seed give byte-identical
  captures (6551 frames, twice at each of two seeds). The frame rules, the frame rate and
  the sizes are those of RGSS2. The interpreter says `ruby 1.9.2p0 (2010-08-18 revision
  29036) [i386-mswin32_90]`; 2170 symbols exist before the first script and the next
  serial is 2086; the default external and source encoding is UTF-8; only four encodings
  are listed; there is no prelude; `Marshal` is 4.8; `String#hash` differs in every
  process; Fibers work with a small stack (a method calling itself reaches depth 284);
  Ruby threads run and are not repeatable against frames.
- Ruby 1.9.2-p0 as a vendored static library. It builds from the release tarball with a
  checked-in `config.h` and eight small patches, without `configure` or a bootstrap Ruby;
  it needs `-std=gnu89 -fwrapv`; Fibers and two values lost to the conservative collector
  needed fixes with a current compiler; a script with Fibers on a host-created thread
  survives `fork` between two frames; 868 global symbols are defined by both interpreters.

Corpus obtained (each ships its DLL in `System/`; the run-time package holds none):
Crysalis (unencrypted, DLL 3.0.0.1, main test game), In Search of Immortality
(`Game.rgss3a`, 3.0.1.1, resizes to 640x480), Capella's Promise 1.09 (`Game.rgss3a`,
3.0.1.1).

## Goals / Non-Goals

Goals: the decisions and milestones of VX Ace are on record, so that the changes that
implement it can cite them.

Non-goals: any code; any statement that VX Ace runs.

## Decisions

Decided by the owner on 2026-10-13 (the first two by choice among laid-out options, the
rest by accepting the direction of the studies under the goal the owner set that day: VX Ace
finished and published):

1. **Milestones and release.** A0: the original runtime of VX Ace under Wine (done
   2026-10-13, with limits). A1: title screen and menu. A2: walking on a map. A3: battle,
   saving and loading. The standard and the rule of when a milestone is reached are those
   of V1 to V3. Version 0.3.0 is released when A3 is reached.
2. **Guest executable.** A game of RGSS3 runs in a guest executable of its own,
   `vkxp-guest-rgss3`, beside `vkxp`: the binding over Ruby 1.9.2 and the guest role, no
   window, no Vulkan, no audio. The program knows the runtime before it starts a guest and
   starts that executable instead of itself, found beside its own path; when the guest
   attaches, the program checks that it is of its own version. The release archive holds
   both files.
3. **`--guest thread`** with a game of RGSS3 is refused with a message that names
   `--guest process`; without the option a headless run of RGSS3 uses a process.
4. **Interpreter.** ruby-1.9.2-p0 is vendored under `third_party/ruby-1.9.2/` as 1.8.7 is:
   a subset with a manifest, a patch log, a checked-in `config.h`, built by a sys crate of
   its own (`vkxp-ruby192-sys`) that offers the Rust API of `vkxp-ruby-sys`. The stress run
   of the collector is part of the gate for it from the start.
5. **Binding.** One source for the RGSS classes: `vkxp-binding` is compiled a second time as
   a package of its own over `vkxp-ruby192-sys`; what differs by interpreter is behind the
   API of the sys crates or a runtime check. No second copy of the classes.
6. **Encodings.** Only what the original has is linked and listed; an encoding the original
   does not know is unknown in vkxp too. Which ones exactly is measured on the original
   before the change that implements it.
7. **Hash seed.** `String#hash` gets a fixed seed in vkxp: the original's differs per
   process, so there is no value to reproduce, and a run of vkxp stays reproducible.
8. **Script threads.** Ruby threads of RGSS3 are native threads. They run; a run with a
   live script thread is not claimed to be reproducible, and a save state is refused while
   one is alive. The timer thread of the interpreter is not started before a script makes
   a thread.
9. **Word size.** The guest stays a 64-bit build against a 32-bit original; differences a
   script can see (`2**30` is a Fixnum, `1.size`) are recorded as known differences unless
   a game of the corpus depends on one.
10. **Order.** Vendoring and the sys crate, then the binding, are one chain. Beside it,
    at once: the guest role as a library, runtime detection with the archive `Game.rgss3a`
    and the built-in scripts, and the parts of the RGSS3 surface that live in `vkxp-core`.
    Behaviour of the RGSS3 classes is measured on the original before it is implemented,
    as for VX.
11. **The original side** of every comparison runs without GPU device nodes and without
    network (change `run-the-original-without-device-nodes`).
12. **Process.** As before: OpenSpec changes implemented by flows, at most three in
    parallel. The owner's standing go-ahead covers VX Ace through the release of 0.3.0,
    pushing for that release included.

Risks to record: more values lost to the conservative collector with a current compiler
(answered by the stress run in the gate); native threads against `fork` and
reproducibility (decision 8); strings with encodings throughout the binding; glyph shapes,
as before; `DL` reaching into the runtime's memory (a decision of its own when a game
needs it); `pow` of glibc differs from the DLL's C runtime in the last bit for `10.0**23`
(the rest of `Math` is not surveyed).

## Implementation notes

- The section "VX Ace" of `docs/decisions.md` is rewritten from the decisions above; the
  sentence it holds today is replaced. The row A0 names what the record of a milestone
  needs: games and revisions, DLLs, seeds, schedule, frames, the repeated run, and under
  "Not covered" the limits (no frame inside a transition; no save, load or reset measured;
  the two encrypted games ran once each; input in sequences without `Graphics.update`
  differed by one update in two runs).
- "Process": the sentence on the go-ahead is replaced by decision 12; the history that it
  did not cover VX Ace until 2026-10-13 stays in one clause.
