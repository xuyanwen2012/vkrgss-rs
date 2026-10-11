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
every pull request; [releasing.md](releasing.md) describes the release package.

## Checks outside the gate

Two checks are run by hand, because one is slow and needs another compiler and the other
needs the network. Neither is a step of the merge gate.

**The upstream Ruby suites under sanitizers.** The interpreter, Oniguruma and Ruby's own test
executable are compiled again with AddressSanitizer and UndefinedBehaviorSanitizer, and the
suites the gate runs for their results are run for invalid memory accesses and undefined
behaviour:

```sh
cargo test -p vkxp-ruby-sys --test sanitized -- --ignored
```

It needs `clang` with its sanitizer runtimes (`compiler-rt` on Fedora) and takes about a
minute. It fails on a suite that does not pass and on every sanitizer report that
`crates/vkxp-ruby-sys/tests/sanitized.allow` does not list; an entry there states why the
report is harmless. The full reports are left in `target/tmp/sanitized/reports/`.

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
| `--rtp NAME=DIR` | The directory of the run-time package that `Game.ini` names `NAME`. Repeatable. |
| `--input SCHEDULE\|@FILE` | The buttons held from given frames on, in the syntax described under [Running the demo](running.md#running-the-demo), or a file that holds such a schedule. |
| `--builtin-scripts FILE` | The built-in scripts file for vkxp, instead of its default location. |
| `--out DIR` | Where everything is written. Defaults to `target/oracle`. |
| `--vkxp-only` | Run only vkxp. Needs neither Wine nor Xvfb nor the DLL. |
| `--locale LOCALE` | `LC_ALL` of the original, which decides the code page its file names are read in: `zh_CN.UTF-8`, the default, for a Chinese game, `ja_JP.UTF-8` for a Japanese one. |
| `--timeout SECONDS` | How long the original may take to reach one frame. Defaults to 120. |

Each side runs a copy of the game, `<out>/original/game` and `<out>/vkxp/game`, with a probe
script added in front of the scripts of its archive. The probe seeds the random numbers with
a fixed value, answers `Input.press?`, `Input.trigger?`, `Input.repeat?`, `Input.dir4` and
`Input.dir8` from the schedule instead of the keyboard, and counts frames as the host of vkxp
does: one for every `Graphics.update` and `d` for `Graphics.transition(d)`. The game directory
given is never written to, and the output directory may not be inside it.

A game that ships its data in `Game.rgssad` (RGSSAD version 1) is compared as it is: the
probe is added to the script archive inside the encrypted archive of each copy, every other
entry is written back unchanged, and no loose script archive is added, because the original
reads its scripts only from the archive when there is one. To make such a game from an
unpacked one, `cargo xtask pack-rgssad <game directory>` writes every file below the
directory's `Data` and `Graphics` folders into `Game.rgssad`, under its path with
backslashes, and then removes the two folders. It changes the directory it is given, so give
it a copy; it refuses a directory that already has an archive or has no `Game.ini`.

The original side needs `wine`, `wineboot`, `wineserver` and `Xvfb` on `PATH` and a game that
has its `Game.exe`. The command starts a virtual display of its own, makes a Wine prefix at
`<out>/prefix` on the first run (which takes a while), registers the run-time packages and
replacements for common game fonts in it, and runs `Game.exe` with software rendering only.
At each frame asked for the probe holds the game until the command has read the picture of
the game window from the display; after the last one the game ends. A frame that falls
inside a transition is captured when the transition has ended, because the original cannot
be held in the middle of one. What Wine and the game print is in `<out>/original/wine.log`,
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
| `<out>/summary.json` | for every frame the three file names, `differing_pixels` and `largest_difference` (of any colour channel) |

It exits with status 0 when both runs completed, whatever the differences are; a missing
prerequisite or a run that fails is an error that names what was missing or what happened.
With `--vkxp-only` there is nothing to compare, and the summary lists the pictures of vkxp.
The pictures show the game's and the run-time package's art, so they stay out of the
repository, like everything else below `target/`.

The tests of the harness run the vkxp side on games they generate. The one test of the
original side is ignored by default, because it needs Wine, Xvfb and files that are not in the
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
