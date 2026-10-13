# Development

## Merge gate

One command checks everything a change must pass. Its test steps render with Vulkan, so the
loader restriction described under [Prerequisites](building.md#prerequisites) is part of it:

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate
```

It runs these steps in order, stops at the first failure and names the step that failed. It
prints how long each step took, after the step and once more for all of them at the end:

| Step | Command |
|---|---|
| `fmt` | `cargo fmt --all --check` |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| `test` | `cargo test --workspace --locked --no-fail-fast` |
| `test-unvalidated` | `cargo test -p vkxp-vk -p vkxp-host --locked --no-fail-fast` with `VK_LOADER_LAYERS_DISABLE='*validation*'` |
| `doc` | `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings` |
| `openspec` | `openspec validate --all` |

The `cargo xtask` alias and the `clippy`, `test`, `test-unvalidated` and `doc` steps pass
`--locked`, so the gate does not rewrite `Cargo.lock` and fails when the lock file is out of
date.

The `test` and `test-unvalidated` steps pass `--no-fail-fast`: every test binary runs although
an earlier one failed, so the output of one run names all that fail. The step still fails.

`test-unvalidated` runs the Vulkan tests again as players run, without the validation layer:
the loader does not load a disabled layer although it is installed. The layer wraps every
handle in a unique number, so a fault that needs a driver to hand out the handle of a
destroyed object again, such as a descriptor that is not rewritten, cannot show while the
layer is loaded.

The toolchain is pinned in `rust-toolchain.toml`. The `openspec` CLI must be on `PATH`.

The workflow `.github/workflows/ci.yml` runs the gate on GitHub for every push to `main` and
every pull request; [releasing.md](releasing.md) describes the release package. Beside the
job `gate` on x86_64 it has the job `gate-aarch64`, the same steps on an aarch64 runner
(`ubuntu-24.04-arm`). aarch64 is not supported: that job is allowed to fail, does not fail
the workflow, and its log is where the state of aarch64 can be seen.

## Checks outside the gate

Four checks are run by hand, because three are slow and need another compiler and the
fourth needs the network. None is a step of the merge gate.

**The upstream Ruby suites under sanitizers.** The interpreter, Oniguruma and Ruby's own test
executable are compiled again with AddressSanitizer and UndefinedBehaviorSanitizer, and the
suites the gate runs for their results are run for invalid memory accesses and undefined
behaviour:

```sh
cargo test -p vkxp-ruby-sys --test sanitized -- --ignored upstream_suites_pass_under_the_sanitizers
```

It needs `clang` with its sanitizer runtimes (`compiler-rt` on Fedora) and takes about a
minute. It fails on a suite that does not pass and on every sanitizer report that
`crates/vkxp-ruby-sys/tests/sanitized.allow` does not list; an entry there states why the
report is harmless. The full reports are left in `target/tmp/sanitized/reports/`.

**The tests under collector stress.** The interpreter's collector is conservative: a value
stays alive only while a copy of it lies on the machine stack, in a register or in a
reachable object. Code that holds a value only through a pointer into it, the buffer of a
string or the slots of an array, loses it when a collection comes at the wrong moment, which
depends on the compiler and on the machine. The environment variable `VKXP_GC_STRESS=<n>`,
with `n` of 1 or more, makes every interpreter that `vkxp-ruby-sys` starts collect at every
n-th allocation of an object or of memory, so that such a fault shows at once; unset, empty
or 0 changes nothing. The variable is read in every build, also by the release executable,
so a game can be run under it, very slowly: `VKXP_GC_STRESS=1 vkxp <game directory>`.

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask stress
```

runs three steps under the variable, goes on after one that fails and names all that failed:

| Step | What runs | `n` |
|---|---|---|
| `suites` | the upstream Ruby suites in the sanitized build, where the use of the buffer of a collected string is a report of AddressSanitizer, and a test that such a report appears for code that keeps only the buffer pointer of a fresh string (`cargo test -p vkxp-ruby-sys --locked --test sanitized -- --ignored under_collector_stress`) | 32 |
| `binding` | `cargo test -p vkxp-binding --locked --no-fail-fast` | 1 |
| `guest` | the tests of the executable that start a guest, `cargo test -p vkxp --locked --test end_to_end` | 1 |

The values of `n` are constants in `xtask/src/stress.rs`. The command needs what the
sanitized run needs and takes about half an hour on the development machine. A test
that is left out is named in `xtask/src/stress.skip` with the reason. A lost value is a
defect also without the variable; it is fixed with a guard (`RB_GC_GUARD` in the vendored
tree, recorded as a patch, or a root in the binding). The reports of the first step are left
in `target/tmp/sanitized/stress/reports/`.

**The fuzz targets.** Three parts of the vendored C code read what comes with a game:
`Marshal.load` reads save files and `.rxdata` files, the parser reads the scripts, and the
regular expression engine compiles the patterns in both. A fuzz target feeds one of them
arbitrary bytes, under libFuzzer, AddressSanitizer and UndefinedBehaviorSanitizer:

```sh
cargo xtask fuzz marshal --seconds 300
```

| Target | What an input is | Largest input | Time limit of one input |
|---|---|---|---|
| `marshal` | the argument of `Marshal.load` | 4096 bytes | 10 s |
| `parse` | a script, compiled as `eval` compiles a string and not run | 4096 bytes | 10 s |
| `regex` | a pattern up to the first line break, compiled as `Regexp.new` compiles it, and after it the subject it is matched against | 512 bytes | 3 s |

The targets are one C file each in `crates/vkxp-ruby-sys/fuzz/`, with the seeds they start
from in `seeds/<target>/`. The command builds the interpreter and the engine as the
sanitized run does, with libFuzzer's coverage instrumentation, and runs the target for 60
seconds or for the time given, under `target/tmp/fuzz/<target>/`: the corpus that the run
grows stays in `corpus/` there, for the next run. It needs `clang` with its sanitizer
runtimes and libFuzzer (`compiler-rt` on Fedora) and says so when they are missing. A name
that is not a target is an error that names the three.

A Ruby exception is not a finding. A sanitizer report or a crash is: it ends the command
with a failure, the report is on the error stream and the input in `artifacts/`. Two
things are not findings, and the run goes on in a new process after them: an input that
runs into its time limit, which a pattern that backtracks does by its nature, and a process
that runs into libFuzzer's memory limit of 2 GiB, which happens because the interpreter
never frees a symbol and the parser does not free the tables of the scopes that a syntax
error leaves open. The inputs are small because nesting recurses: in the sanitized build,
with a stack of 8 MiB, `Marshal.load` exhausts the stack at about 21,800 nested arrays (an
input of 43 KiB) and the engine at about 3,190 nested groups (6.4 KiB), which the limits
keep an input far from; the parser refuses deep nesting itself. A single allocation of
more than 256 MiB gets a null pointer, so a length field that asks for more raises
`NoMemoryError`.

A defect that a target finds is fixed in the vendored tree as a recorded patch, and its
input, reduced, goes to `crates/vkxp-ruby-sys/fuzz/fixed/<target>/`, where a test of the
normal test step runs it. [vendored-security.md](vendored-security.md) lists what the first
runs found, together with the review of the security fixes published for the two vendored
trees after their releases.

**The dependency audit.** `cargo deny` checks every crate of `Cargo.lock` against the
security advisory database, the licence allow-list and the allowed sources in `deny.toml`:

```sh
cargo xtask audit
```

It runs `cargo deny --locked check --warn unmaintained`, which downloads the advisory
database. A vulnerability, a licence that is not on the list or a crate from anywhere but
crates.io fails it; an unmaintained crate is a warning. Without `cargo-deny` the command
says how to install it (`cargo install --locked cargo-deny`) and fails.

## Testing with another compiler

The interpreter's collector scans the machine stack, so whether a value survives a
collection can depend on what the compiler keeps there: a fault of that kind showed with
gcc 13.3 on the CI runner and not with the gcc of the development machine. Two ways to
build and test the interpreter with a compiler other than the default one.

With clang, which the build script takes from `CC`, in a target directory of its own:

```sh
CC=clang CARGO_TARGET_DIR=target/clang cargo test -p vkxp-binding -p vkxp-ruby-sys --locked
```

With the gcc of the CI runner, in a container of Ubuntu 24.04 that uses the Rust toolchain
and the cargo cache of the machine:

```sh
podman run --rm --security-opt label=disable -v "$PWD":/src \
    -v "$HOME/.cargo":/cargo -v "$HOME/.rustup":/rustup:ro ubuntu:24.04 \
    bash -c 'apt-get update -qq && apt-get install -y -qq --no-install-recommends \
        build-essential libcrypt-dev ca-certificates >/dev/null &&
      export PATH=/cargo/bin:$PATH CARGO_HOME=/cargo RUSTUP_HOME=/rustup \
        CARGO_TARGET_DIR=/src/target/ubuntu-gcc && cd /src &&
      cargo test -p vkxp-binding -p vkxp-ruby-sys --locked --offline'
```

The collection stress test of `vkxp-binding` (`tests/stress.rs`) is the one that finds such
faults; a fix is a `RB_GC_GUARD` recorded as a patch in
[third_party/ruby-1.8.7/PATCHES.md](../third_party/ruby-1.8.7/PATCHES.md).

## Comparing with the original runtime

`cargo xtask oracle` runs one game in the original runtime and in vkxp under the same
conditions and reports how chosen frames differ. The original runtime, the run-time package
and the game are yours and stay where they are: the command takes their locations, works on
copies of the game and writes only below its output directory.

```sh
cargo xtask oracle <game directory> --frames 10,20,200 \
    --dll <RGSS DLL> --rtp Standard=<RTP directory> --input "60:RIGHT, 100:"
```

| Option | Meaning |
|---|---|
| `--frames N[,N...]` | The frames to capture, numbered from 0 as `vkxp --dump` numbers them. Required. |
| `--dll FILE` | The RGSS DLL that the game's `Game.ini` names with `Library=`. Required unless `--vkxp-only` is given. |
| `--runtime rgss1\|rgss2` | The runtime of the game. Only needed when the file that `Scripts=` of `Game.ini` names is neither a `.rxdata` (RGSS1, RPG Maker XP) nor a `.rvdata` (RGSS2, RPG Maker VX). |
| `--rtp NAME=DIR` | The directory of the run-time package that `Game.ini` names `NAME`, with `RTP1` to `RTP3` in a game of XP and with `RTP` in a game of VX. Repeatable. |
| `--input SCHEDULE\|@FILE` | The buttons held from given frames on, in the syntax described under [Running the demo](running.md#running-the-demo), or a file that holds such a schedule. |
| `--font FILE` | A font file that both runtimes draw text with, as described below. Repeatable. |
| `--builtin-scripts FILE` | The built-in scripts file for vkxp, instead of its default location. |
| `--out DIR` | Where everything is written. Defaults to `target/oracle`. |
| `--vkxp-only` | Run only vkxp. Needs neither Wine nor Xvfb nor the DLL. |
| `--original-only` | Run only the original. Nothing is compared, and vkxp is not built. Not together with `--vkxp-only`. |
| `--no-threads` | On both sides, make `Thread.new` run its block at once in the caller and `sleep` return at once inside such a block. For a game whose Ruby threads end the original under Wine. |
| `--locale LOCALE` | `LC_ALL` of the original, which decides the code page its file names are read in: `zh_CN.UTF-8`, the default, for a Chinese game, `ja_JP.UTF-8` for a Japanese one. |
| `--timeout SECONDS` | How long the original may take to reach one frame. Defaults to 120. |

Each side runs a copy of the game, `<out>/original/game` and `<out>/vkxp/game`, with a probe
script added in front of the scripts of its archive. The probe seeds the random numbers with
a fixed value, answers `Input.press?`, `Input.trigger?`, `Input.repeat?`, `Input.dir4` and
`Input.dir8` from the schedule instead of the keyboard, and counts frames as the host of vkxp
does: one for every `Graphics.update` and `d` for `Graphics.transition(d)`. The game directory
given is never written to, and the output directory may not be inside it.

A game of RPG Maker VX is run in the same way, with what its runtime has differently: the
window and the pictures are 544 by 416, the run-time package is registered for RGSS2 (for
example `--rtp RPGVX=<directory>`), the encrypted archive is `Game.rgss2a`, and the probe is
another one. That probe repeats a held button at its 24th update and then at every 6th, and
it counts frames as the original of VX advances `Graphics.frame_count`: one for `update`, `n`
for `wait(n)`, `fadeout(n)` and `fadein(n)`, and `n` for `transition(n)` after a `freeze` but
none without one. On the original's side it runs `wait`, `fadeout` and `fadein` as loops of
updates that were measured to show the same pictures, so that a frame inside them can be held,
and it calls `Graphics.frame_reset` around a hold, because the original does not draw the
updates that follow a stall. `--original-only` runs the side of the original alone.

A game that ships its data in `Game.rgssad`, or `Game.rgss2a` (both RGSSAD version 1), is compared as it is: the
probe is added to the script archive inside the encrypted archive of each copy, every other
entry is written back unchanged, and no loose script archive is added, because the original
reads its scripts only from the archive when there is one. To make such a game from an
unpacked one, `cargo xtask pack-rgssad <game directory>` writes every file below the
directory's `Data` and `Graphics` folders into `Game.rgssad`, under its path with
backslashes, and then removes the two folders. It changes the directory it is given, so give
it a copy; it refuses a directory that already has an archive or has no `Game.ini`.

The original side needs `wine`, `wineboot`, `wineserver`, `Xvfb` and `bwrap` on `PATH` and a
game that has its `Game.exe`. Every Wine process runs inside a `bwrap` sandbox in which
`/dev/dri` is an empty directory and there is no network: software rendering alone does not
keep the run from the GPUs of the machine, because a game that plays Ogg or MP3 does it
through GStreamer, whose hardware plugins probe VA-API. VA-API is also pointed at a driver
directory that does not exist. A game that needs the network on the original side is not
compared. The command starts a virtual display of its own, makes a Wine prefix at
`<out>/prefix` on the first run (which takes a while), registers the run-time packages and
replacements for common game fonts in it, and runs `Game.exe` with software rendering only.
At each frame asked for the probe holds the game until the command has read the picture of
the game window from the display; after the last one the game ends. A frame that falls
inside a transition is captured when the transition has ended, because the original cannot
be held in the middle of one. In a game of VX such a frame is a late frame, unless it is the
last frame of the transition, whose picture is the one the transition ends with: the picture
of the original is kept, the report lists the frame as "not comparable: inside a transition",
and it counts neither as equal nor as different. What Wine and the game print is in `<out>/original/wine.log`,
which is where to look when the command reports that a frame was not reached. vkxp runs
headless on the CPU device unless the Vulkan loader is already restricted to other drivers.

Text can only be compared when both runtimes draw it with the same font file. `--font FILE`,
which may be given several times, names such a file, a TrueType or OpenType font or a
collection: the files are copied to `<out>/fonts`, which vkxp searches before the fonts of the
system and sizes by their own line boxes (`--font-own-metrics`), as the original does, and into the font directory of the prefix, where they are registered, and the common
game font names (`SimHei`, `黑体`, `SimSun`, `MS PGothic` and the like) are replaced in the
original by the family name read from the first file. A file that cannot be read or is not a
font ends the run before either runtime starts. Without `--font` the original draws those
names with whatever `Noto Sans CJK` the machine has and vkxp with its own choice, and the
command prints a line saying that text is not comparable.

The command prints one line per frame and writes:

| File | Contents |
|---|---|
| `<out>/frames/<n>-original.png` | frame `n` of the original |
| `<out>/frames/<n>-vkxp.png` | frame `n` of vkxp |
| `<out>/frames/<n>-diff.png` | the absolute difference of red, green and blue, multiplied by 32 |
| `<out>/summary.json` | for every frame the three file names, `differing_pixels` and `largest_difference` (of any colour channel), or `not_comparable` with the reason: a late frame, or two pictures of different sizes; `totals` with the numbers of frames that are equal, different and not comparable; `no_threads` when that option was used |

It exits with status 0 when both runs completed, whatever the differences are; a missing
prerequisite or a run that fails is an error that names what was missing or what happened.
With `--vkxp-only` or `--original-only` there is nothing to compare, and the summary lists
the pictures of the side that ran.

The limits of the original's side:

- A Ruby thread ends the original under Wine, in XP and in VX. `--no-threads` lets such a game
  go on, but changes it: what the thread did over time happens at once. The option applies to
  both sides, and the report says that it was used.
- A frame inside a transition cannot be held. In a game of VX it is a late frame, as above; in
  a game of XP its picture is whatever the window shows when the transition has ended.
- The game window is looked for by the frame size of the runtime. A game that changes the
  size of its window (`Graphics.resize_screen`, a resizer of its own) cannot be captured
  after that.
The pictures show the game's and the run-time package's art, so they stay out of the
repository, like everything else below `target/`.

The tests of the harness run the vkxp side on games they generate. The one test of the
original side is ignored by default, because it needs Wine, Xvfb, `bwrap` and files that are not in the
repository; it captures frames 10 and 20 of a real game below `target/oracle-test`. Its three
variables hold absolute paths:

```sh
VKXP_ORACLE_DLL=<RGSS DLL> VKXP_ORACLE_RTP=<RTP directory> VKXP_ORACLE_GAME=<game directory> \
    cargo test -p xtask -- --ignored the_original_is_captured
```

## The OpenSpec workflow

Behaviour is specified before it is built, with [OpenSpec](https://github.com/Fission-AI/OpenSpec).
[`openspec/specs`](../openspec/specs) holds the specifications, one directory per capability:
requirements with scenarios, many of them rules measured on the original runtime.
A change is a directory under `openspec/changes` with a proposal, a design, the requirements
it adds or modifies and a list of tasks; it is implemented task by task, and
`openspec validate --all`, the last step of the gate, checks every specification and change.
A finished change is merged into the specifications and moved to
[`openspec/changes/archive`](../openspec/changes/archive), which is the history of changes.
[decisions.md](decisions.md) records the decisions that are settled.
