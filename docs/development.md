# Development

## Merge gate

One command checks everything a change must pass. Its test steps render with Vulkan, so the
loader restriction described under [Prerequisites](building.md#prerequisites) is part of it:

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate
```

It runs these steps in order, the two test steps together, stops at the first failure and
names the step that failed. It prints how long each step took, after the step and once more
for all of them at the end:

| Step | Command |
|---|---|
| `fmt` | `cargo fmt --all --check` |
| `clippy` | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| `test` | `cargo test --workspace --locked --no-fail-fast` |
| `test-unvalidated` | `cargo test -p vkrgss-vk -p vkrgss-host --locked --no-fail-fast` with `VK_LOADER_LAYERS_DISABLE='*validation*'` |
| `doc` | `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings` |
| `openspec` | `openspec validate --all` |

The `cargo xtask` alias and the `clippy`, `test`, `test-unvalidated` and `doc` steps pass
`--locked`, so the gate does not rewrite `Cargo.lock` and fails when the lock file is out of
date.

The `test` and `test-unvalidated` steps pass `--no-fail-fast`: every test binary runs although
an earlier one failed, so the output of one run names all that fail. The step still fails.

The two test steps run at the same time, and so do the binaries within them, as many at a time
as the machine has hardware threads. The gate builds the binaries of each step with its command
and `--no-run`, then runs each binary as `cargo test` does: one process per binary, in the
directory of its package, with `CARGO`, `CARGO_MANIFEST_DIR` and `CARGO_MANIFEST_PATH` set.
The documentation tests of a step are one more process, `cargo test --doc` with the step's
arguments, which leaves out the packages whose manifest turns them off (`doctest = false`), as
`cargo test` does. The output of each binary is printed in one piece when it ends and kept in
`target/gate/<step>/`, and the failure of the step names every binary that failed. The
duration of each test step counts from the start of both until its last binary ended. The
gate clears `PROPTEST_CASES`, so that the property tests run the cases they name. Tests that
run at the same time must not share a file: the scratch directories below `target/tmp/` of
the tests of Ruby 1.9.2 end in `192` where those of Ruby 1.8.7 have the same name.

`test-unvalidated` runs the Vulkan tests again as players run, without the validation layer:
the loader does not load a disabled layer although it is installed. The layer wraps every
handle in a unique number, so a fault that needs a driver to hand out the handle of a
destroyed object again, such as a descriptor that is not rewritten, cannot show while the
layer is loaded.

In the `test` step the validation layer checks synchronization too: where it offers
`VK_EXT_layer_settings`, a backend created with a validation log, as every test of
`vkrgss-vk` creates one, turns on its setting `validate_sync`. A missing or wrong barrier is
then reported as a hazard, `SYNC-HAZARD-*` in front of the message, and fails the test like
any other validation error. A layer without that extension validates without it, which the
log says once.

The toolchain is pinned in `rust-toolchain.toml`. The `openspec` CLI must be on `PATH`.

`cargo xtask gate --quick` is a shorter check for the work on a change; it is not the gate
([building.md](building.md#checking-a-change)).

### Parts of the gate

`cargo xtask gate --job <name>` runs one part of the gate, with the same report of
durations; the test steps of a part run at the same time, as in the gate. Together the parts run exactly the steps of `cargo xtask gate`: every check,
package and test target is in one part, which a test of `xtask` checks, so that nothing
falls between them. An unknown name fails and lists the parts.

| Part | Steps |
|---|---|
| `lint` | `fmt`, `clippy`, `doc`, `openspec` |
| `ruby` | `test` of `vkrgss-ruby-sys` and `vkrgss-ruby192-sys` except `upstream_stress` |
| `ruby-stress` | `test` of `vkrgss-ruby192-sys --test upstream_stress`, the quick files of `test/ruby` under collector stress ([checks outside the gate](#checks-outside-the-gate)) |
| `vulkan` | `test` of `vkrgss-vk` |
| `vulkan-unvalidated` | `test-unvalidated` |
| `rest` | `test` of every other package |

A new package is in `rest` without a change. A new test target of `vkrgss-ruby192-sys` is
named in the part `ruby` in `xtask/src/gate.rs`, because cargo cannot leave out one test
target; the test fails until it is.

### Continuous integration

The workflow `.github/workflows/ci.yml` runs the gate on GitHub for every push to `main` and
every pull request; [releasing.md](releasing.md) describes the release package. It runs each
part of the gate as a job of its own, named after the part, so that they run in parallel and
the checks of a pull request take about ten minutes instead of half an hour. The jobs are
the same apart from the part. Each caches the objects of the compilers with sccache: cargo
starts rustc through `RUSTC_WRAPPER=sccache`, and the `cc` crate puts that wrapper in front
of `gcc-13` too, so that the two interpreters are compiled again only when their sources
change. `Swatinem/rust-cache` keeps the registry and the dependencies in the build directory;
a checkout gives every file a new modification time, by which cargo decides to rebuild the
packages of the workspace.

Beside the parts on x86_64 it has the job `gate-aarch64`, the same steps on an aarch64 runner
(`ubuntu-24.04-arm`), which runs the whole gate in one job. aarch64 is not supported: that
job is allowed to fail, does not fail the workflow, and its log is where the state of aarch64
can be seen.

A newer push to a pull request cancels the run of the one before; a run for a push to `main`
is never cancelled. The workflow `.github/workflows/nightly.yml` runs the four
[checks outside the gate](#checks-outside-the-gate) once a day and when started by hand,
each as a job of its own: `audit`, `stress`, `sanitized` and `sanitized192` (the sanitized
suites of the two interpreters) and `fuzz`, the last once for every target with
`--seconds 300`. When a fuzz job fails, it uploads the inputs its target put aside,
`target/tmp/fuzz/<target>/artifacts/`, as an artifact of the run that is kept for 30 days.
No pull request waits for it, so its failure does not
block merging. When one of its jobs fails, its last job, `issue`, opens the issue "Nightly
checks failed" with the link to the run, or comments on that issue while it is open; it is
the only job of the workflows that may write issues.

The workflows name every action by its commit hash, with the tag the hash was resolved
from as a comment, install the OpenSpec CLI, `cargo-deny` and sccache at fixed versions,
and set `CC=gcc-13` in every job that builds, the compiler the release is built with
([building.md](building.md#prerequisites)). A test of `xtask` (`xtask/tests/workflows.rs`)
fails on a workflow that does not. A new version is taken by changing the hash and the
comment together.

## Checks outside the gate

Four checks are run by hand and by the workflow `nightly.yml`, because three are slow and
need another compiler and the fourth needs the network. None is a step of the merge gate.

**The upstream Ruby suites under sanitizers.** The interpreter, Oniguruma and Ruby's own test
executable are compiled again with AddressSanitizer and UndefinedBehaviorSanitizer, and the
suites the gate runs for their results are run for invalid memory accesses and undefined
behaviour:

```sh
cargo test -p vkrgss-ruby-sys --test sanitized -- --ignored upstream_suites_pass_under_the_sanitizers
```

It needs `clang` with its sanitizer runtimes (`compiler-rt` on Fedora) and takes about a
minute. It fails on a suite that does not pass and on every sanitizer report that
`crates/vkrgss-ruby-sys/tests/sanitized.allow` does not list; an entry there states why the
report is harmless. The full reports are left in `target/tmp/sanitized/reports/`.

The second interpreter, Ruby 1.9.2 of `vkrgss-ruby192-sys`, has a sanitized run of its own:
its interpreter, encodings and `Zlib` and the test executable are compiled again in the same
way, and `bootstraptest` and `test/ruby` run with what `tests/upstream.allow` leaves out:

```sh
cargo test -p vkrgss-ruby192-sys --test sanitized -- --ignored
```

It takes about two minutes and fails on a suite that does not pass and on every report that
`crates/vkrgss-ruby192-sys/tests/sanitized.allow` does not list. Its reports are left in
`target/tmp/sanitized192/reports/`. The options of the sanitizers, that directory included,
are compiled into the test executable too, because the suites start interpreters with an
emptied environment, whose reports would otherwise go to their error stream unchecked;
`tests/sanitized.rs` says why each is set, and an ignored test of it shows that such a
report is collected and checked.

**The tests under collector stress.** The interpreter's collector is conservative: a value
stays alive only while a copy of it lies on the machine stack, in a register or in a
reachable object. Code that holds a value only through a pointer into it, the buffer of a
string or the slots of an array, loses it when a collection comes at the wrong moment, which
depends on the compiler and on the machine. The environment variable `VKRGSS_GC_STRESS=<n>`,
with `n` of 1 or more, makes every interpreter that `vkrgss-ruby-sys` starts collect at every
n-th allocation of an object or of memory, so that such a fault shows at once; unset, empty
or 0 changes nothing. The variable is read in every build, also by the release executable,
so a game can be run under it, very slowly: `VKRGSS_GC_STRESS=1 vkrgss <game directory>`.

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask stress
```

runs six steps, all but the fourth under the variable, goes on after one that fails and
names all that failed:

| Step | What runs | `n` |
|---|---|---|
| `suites` | the upstream Ruby suites in the sanitized build, where the use of the buffer of a collected string is a report of AddressSanitizer, and a test that such a report appears for code that keeps only the buffer pointer of a fresh string (`cargo test -p vkrgss-ruby-sys --locked --test sanitized -- --ignored under_collector_stress`) | 32 |
| `binding` | `cargo test -p vkrgss-binding --locked --no-fail-fast` | 1 |
| `guest` | the tests of the executable that start a guest, and the tests of the guest records of the program and of the guest executable of RGSS1 and RGSS2, which make cargo build that executable first, `cargo test -p vkrgss -p vkrgss-guest-rgss12 --locked --test end_to_end --test guest_record`; the guest processes inherit the variable | 1 |
| `suites-1.9.2` | the files of `test/ruby` of Ruby 1.9.2 that are slow under `GC.stress = true` (`cargo test -p vkrgss-ruby192-sys --locked --test upstream_stress -- --ignored`) | every allocation |
| `binding-1.9.2` | the tests of the binding over Ruby 1.9.2, `cargo test -p vkrgss-binding-rgss3 --locked --no-fail-fast` | every allocation |
| `guest-1.9.2` | the tests of the executable that run a game of RGSS3 through the guest executable, and the tests of the guest records of both executables, `cargo test -p vkrgss -p vkrgss-guest-rgss3 --locked --test rgss3 --test guest_record`; the guest processes inherit the variable | every allocation |

The values of `n` are constants in `xtask/src/stress.rs`. The first step needs what the
sanitized run needs; the last three need no sanitizer and run the ordinary build. The
command takes about three quarters of an hour on the development machine. A test
that is left out is named in `xtask/src/stress.skip` with the reason. A lost value is a
defect also without the variable; it is fixed with a guard (`RB_GC_GUARD` in the vendored
tree, recorded as a patch, or a root in the binding). The reports of the first step are left
in `target/tmp/sanitized/stress/reports/`.

The second interpreter, Ruby 1.9.2 of `vkrgss-ruby192-sys`, has the stress of the release
itself, `GC.stress = true`, which collects at every allocation; `VKRGSS_GC_STRESS` set to a
number of 1 or more switches it on in an interpreter that the crate starts, and there is no
"every n-th" form. Its stressed selection of `test/ruby` is not a check outside the gate:
the files that take under a minute run with the workspace tests, in the part `ruby-stress`
of the gate (`cargo test -p vkrgss-ruby192-sys --test upstream_stress`, about three minutes), together
with a copy of the process made by `fork` between two frames of a script with Fibers and a
host function that is suspended with its Fiber, both under stress. Only the seven slow files
(`test_array.rb`, `test_assignment.rb`, `test_fiber.rb`, `test_literal.rb`,
`test_process.rb`, `test_sprintf_comb.rb`, `test_system.rb`) are left to the step
`suites-1.9.2` above. `test_integer_comb.rb`, `test_m17n_comb.rb` and `test_yield.rb` did
not finish in 14 minutes under stress when the release was studied and are named in
`xtask/src/stress.skip`. A value lost in this interpreter is fixed with a guard recorded in
[third_party/ruby-1.9.2/PATCHES.md](../third_party/ruby-1.9.2/PATCHES.md). The step
`binding-1.9.2` is where the Rust and C code of the binding meets that collector at every
allocation; a test it leaves out is a line `binding-1.9.2 <test>` of the skip list, and a
lost value is never a reason for one. Without the variable, the tests `stress` and
`scene_stress` of `vkrgss-binding-rgss3` switch `GC.stress` on around every snippet
themselves, in the gate.

**The fuzz targets.** Three parts of each vendored interpreter read what comes with a game:
`Marshal.load` reads save files and the data files (`.rxdata`, `.rvdata`, `.rvdata2`), the
parser reads the scripts, and the regular expression engine compiles the patterns in both.
A fuzz target feeds one of them arbitrary bytes, under libFuzzer, AddressSanitizer and
UndefinedBehaviorSanitizer. The targets `marshal`, `parse` and `regex` are those of Ruby
1.8.7 and Oniguruma 5.9.6, the targets `marshal192`, `parse192` and `regex192` those of Ruby
1.9.2 with its own engine:

```sh
cargo xtask fuzz marshal --seconds 300
cargo xtask fuzz marshal192 --seconds 300
```

| Target | What an input is | Largest input | Time limit of one input |
|---|---|---|---|
| `marshal` | the argument of `Marshal.load` | 4096 bytes | 10 s |
| `parse` | a script, compiled as `eval` compiles a string and not run | 4096 bytes | 10 s |
| `regex` | a pattern up to the first line break, compiled as `Regexp.new` compiles it, and after it the subject it is matched against | 512 bytes | 3 s |
| `marshal192` | as `marshal` | 4096 bytes | 10 s |
| `parse192` | a script of source encoding UTF-8, as vkrgss compiles the scripts of RGSS3 through `eval`, compiled into an instruction sequence and not run | 4096 bytes | 10 s |
| `regex192` | as `regex`, the pattern and the subject in UTF-8 | 512 bytes | 3 s |

The targets are one C file each in `crates/vkrgss-ruby-sys/fuzz/`, with the seeds they start
from in `seeds/<target>/`, and those of Ruby 1.9.2 the C file of the name without `192` in
`crates/vkrgss-ruby192-sys/fuzz/`, with its seeds in `seeds/<file>/`. The command builds the
interpreter and the engine as the sanitized run of that interpreter does, with libFuzzer's
coverage instrumentation, and runs the target for 60 seconds or for the time given, under
`target/tmp/fuzz/<target>/`: the corpus that the run grows stays in `corpus/` there, for the
next run. It needs `clang` with its sanitizer runtimes and libFuzzer (`compiler-rt` on
Fedora) and says so when they are missing. A name that is not a target is an error that
names the nine, these six and the three of the file readers below.

A Ruby exception is not a finding. A sanitizer report or a crash is: it ends the command
with a failure, the report is on the error stream and the input in `artifacts/`. Three
things are not findings, and the run goes on in a new process after them, with the input
put aside in `artifacts/`: an input that runs into its time limit, which a pattern that
backtracks does by its nature; a process that runs into libFuzzer's memory limit of 2 GiB,
which happens because the interpreter never frees a symbol and the parser does not free the
tables of the scopes that a syntax error leaves open; and an input that exhausts the
machine stack by nesting. The last is AddressSanitizer's report of a stack overflow with,
among the first eight frames of its trace, a function that `tests/fuzz.rs` lists for the
target as recursing on nesting: `r_object0` and its callers for `Marshal.load`, the
recursive functions of the parser (and of the compiler of Ruby 1.9.2) for a script, and
those of the engine's parser and compiler (`parse_exp`, `parse_subexp`, `setup_tree`,
`get_min_match_length` and others) for a pattern and for a script, whose pattern literals
the parser compiles. Its input is renamed from libFuzzer's `crash-` to `stack-`. A stack
overflow anywhere else is a finding. The line that ends a run counts each of the three.
The inputs are small because nesting recurses: in the sanitized build of
Ruby 1.8.7, with a stack of 8 MiB, `Marshal.load` exhausts the stack at about 21,800 nested arrays (an
input of 43 KiB) and the engine at about 3,190 nested groups (6.4 KiB), which the limits
keep an input far from; the parser refuses deep nesting itself. Groups that refer to each
other, as `(\2)(\1)`, make Oniguruma 5.9.6 recurse without end in `get_min_match_length`
(upstream 7fe8c3f, left; see [vendored-security.md](vendored-security.md)), so the target
`regex` meets such an exhaustion in inputs of any length. The targets of Ruby 1.9.2
have the same limits. A single allocation of
more than 256 MiB gets a null pointer, so a length field that asks for more raises
`NoMemoryError`.

A defect that a target finds is fixed in the vendored tree as a recorded patch, and its
input, reduced, goes to `fuzz/fixed/<target>/` of `crates/vkrgss-ruby-sys`, or to
`fuzz/fixed/<file>/` of `crates/vkrgss-ruby192-sys`, where a test of the normal test step
runs it. [vendored-security.md](vendored-security.md) lists what the first runs found,
together with the review of the security fixes published for the three vendored trees after
their releases.

**The fuzz targets of the file readers.** Three more targets feed arbitrary bytes to the
readers of what a game ships beside its scripts: `jpeg` to the JPEG decoder (the decoder that
follows the original, and `jpeg-decoder` where that one gives up), `bmp` to the BMP decoder,
and `rgssad` to the reader of encrypted archives, which reads every input as an archive of
version 1 and of version 3 and decrypts the data of every entry:

```sh
cargo xtask fuzz jpeg --seconds 300
```

They are the binaries of the package `xtask/fuzz/`, a workspace of its own, which the command
builds with libFuzzer's coverage instrumentation through `libfuzzer-sys` on the pinned
toolchain, without a sanitizer, into `target/fuzz/`, and runs as the targets above, under
`target/tmp/fuzz/<target>/`. An input is at most 64 KiB. A target fails on a panic, on a
decode that returns part of an image, and on more memory held at once than the stated bounds,
which a counting allocator (`crates/vkrgss-image/tests/counting/mod.rs`, shared with the
memory tests of `vkrgss-image`) measures:

| Target | Seeds | Memory held at once |
|---|---|---|
| `jpeg` | the JPEG files of `crates/vkrgss-image/testdata/jpeg/` | 4 times the RGBA output of the largest image, plus 1 MiB |
| `bmp` | `xtask/fuzz/seeds/bmp/` | 4 times the output of the decoded image, plus 1 MiB; on an error, as `jpeg` |
| `rgssad` | `xtask/fuzz/seeds/rgssad/` | 64 bytes for each byte of the input, plus 1 MiB |

The decoder that follows the original keeps a JPEG within 4 times its own output plus 1 MiB,
which the tests of `vkrgss-image` check; the files it leaves to `jpeg-decoder` are held to the
bound of the largest image alone. libFuzzer's memory limit is set to 8 GiB, above what the
largest image needs. Building needs a C++ compiler for libFuzzer.

**The dependency audit.** `cargo deny` checks every crate of `Cargo.lock` against the
security advisory database, the licence allow-list and the allowed sources in `deny.toml`:

```sh
cargo xtask audit
```

It runs `cargo deny --locked check --warn unmaintained`, which downloads the advisory
database. A vulnerability, a licence that is not on the list or a crate from anywhere but
crates.io fails it; an unmaintained crate is a warning. Without `cargo-deny` the command
says how to install it (`cargo install --locked cargo-deny`) and fails.

## The glue of the two interpreters

The Rust code that enters and leaves Ruby, host functions and their trampolines, the
definitions, rooting, `protect` and the calls built on it, is in `crates/vkrgss-ruby-glue`
once for both interpreters. It is generic over the trait `Interpreter` of that crate, which
`src/capi.rs` of `vkrgss-ruby-sys` and of `vkrgss-ruby192-sys` implements with the
declarations of its `ffi`. A fix to that code is a fix for both; a difference between the
interpreters is an item of the trait, implemented in both `capi.rs`. What remains of each
interpreter crate is its build, its `ffi`, its interpreter thread and its signals. The glue
has no interpreter of its own and no tests that run Ruby: the tests of both interpreter
crates exercise it, `tests/protect.rs` the jumps through a host function among them, so a
change to it runs `cargo test -p vkrgss-ruby-sys -p vkrgss-ruby192-sys`, which the quick
gate does for a change of the glue.

## The tests of the binding over two interpreters

The RGSS classes have one source, `crates/vkrgss-binding/src`, which `vkrgss-binding` compiles
over Ruby 1.8.7 and `vkrgss-binding-rgss3` over Ruby 1.9.2. The unit tests are in that source
and run in both packages. For every test file `crates/vkrgss-binding/tests/<name>.rs` the
second package has a file of the same name that loads it as the module `t`; a test of `xtask`
fails for a file without that counterpart, and the counterpart is these lines:

```rust
//! `tests/<name>.rs` of `vkrgss-binding`, over Ruby 1.9.2.
#![cfg(not(vkrgss_interpreter_absent))]
extern crate vkrgss_binding_rgss3 as vkrgss_binding;
#[path = "../../vkrgss-binding/tests/<name>.rs"]
mod t;
```

So the `test` step of the gate builds and runs the tests of the binding twice, and
`cargo test -p vkrgss-binding-rgss3` runs those over Ruby 1.9.2 alone. The source has no `cfg`
and no feature that tells the interpreters apart; the one `cfg` in it,
`vkrgss_interpreter_absent`, is set by the build script of the second package for a target
without Ruby 1.9.2 and empties the package there.

The tests reach the interpreter through the harness `vkrgss_binding::testing`. A test process
has one interpreter, of one runtime: `testing::start_as(runtime)` starts it the first time it
is called and returns the `Harness`, whose methods (`eval`, `run`, `game_dir`,
`bitmap_count`) and which the functions of `testing::host` that reach the interpreter take
as their first argument are the only way to it. So every test declares its runtime, and the
order in which the tests run decides nothing; asking for a second runtime in the same
process panics with a message that names both. A test file usually starts with

```rust
fn harness() -> &'static Harness {
    testing::start_as(Runtime::Rgss1)
}
```

The harness generates its game directory next to the test executable, in a directory named
after the executable and the process id, so that processes of one executable do not share
it; `cargo clean` removes these directories.

A test or a snippet case that cannot pass over Ruby 1.9.2 yet is named in
`crates/vkrgss-binding-rgss3/tests/ruby192.allow`, whose head describes the format. A reason
is one of three kinds: the test expects behaviour of RGSS1 or RGSS2 that a named section of
a measurement of the original runtime of RGSS3 states otherwise; its Ruby, or the text it
expects from Ruby, is that of 1.8 and the construct is named; or it holds the symbol
numbering of Ruby 1.8.1. A crash, a hang, a value lost under stress or a wrong pixel is a
fault and is fixed, never listed.

- A listed test has `vkrgss_binding::pending_over_ruby_1_9_2!()` as its first statement
  (`crate::` in a unit test). Over Ruby 1.9.2 the test executable is started again for that
  one test, and the test counts as passed while it fails there; its output then gives the
  reason of its entry. Over Ruby 1.8.7 the marker does nothing and the list is not read.
- A listed case of `tests/cases` or `tests/scene_cases` is evaluated like the others and is
  reported only when it is as expected.
- When later work makes a listed test or case pass, the run fails and names it. Remove its
  line from the list, and for a test also the marker; the entry goes when its last line
  does. A marker without an entry is inert, so the test then has to pass in place.

The main of a guest has one source too, `crates/vkrgss-guest-main/src`, which
`vkrgss-guest-main` compiles over Ruby 1.8.7 and `vkrgss-guest-main-rgss3` over Ruby 1.9.2, with
the same rules: no source of its own in the second package, the unit tests in both, and a
counterpart for every test file, should there be one.

## The tests of the guest executables

A guest process is a guest executable that the program finds in the directory of its own
file: `vkrgss-guest-rgss12` for a game of RGSS1 or RGSS2, `vkrgss-guest-rgss3` for a game of
RGSS3. The tests of `crates/vkrgss` find them as a player's program does: cargo puts every
binary of the workspace into one directory, and builds the binary of each selected package
that has an integration test before any test runs. Two commands are always right:

```sh
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test -p vkrgss -p vkrgss-guest-rgss12 -p vkrgss-guest-rgss3 --locked
VK_LOADER_DRIVERS_SELECT='*lvp*' cargo test --workspace --locked
```

The tests that run a game of RGSS3 are the file `tests/rgss3.rs`. `cargo test -p vkrgss`
alone builds the program and uses whatever guest executables an earlier build left beside
it; without the one of RGSS3, every test of `tests/rgss3.rs` fails before it starts a child
and names `cargo test -p vkrgss -p vkrgss-guest-rgss3`, and without the one of RGSS1 and
RGSS2 a test of `tests/end_to_end.rs` with the guest in a process fails with the program's
message, which names `vkrgss-guest-rgss12` and the command that builds it.
`tests/end_to_end.rs` runs no game of RGSS3. The games of RGSS3 are written by test code
without an interpreter (`tests/support/ace.rs`), with the stand-in of the built-in scripts
of the binding's test support, which was written for the tests. In the same way `cargo run
-p vkrgss` builds the program alone; `cargo build -p vkrgss-guest-rgss12 -p
vkrgss-guest-rgss3` builds the guest executables beside it.

## Testing with another compiler

The interpreter's collector scans the machine stack, so whether a value survives a
collection can depend on what the compiler keeps there: a fault of that kind showed with
gcc 13.3 on the CI runner and not with the gcc of the development machine. Two ways to
build and test the interpreter with a compiler other than the default one.

With clang, which the build script takes from `CC`, in a target directory of its own:

```sh
CC=clang CARGO_TARGET_DIR=target/clang cargo test -p vkrgss-binding -p vkrgss-ruby-sys --locked
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
      cargo test -p vkrgss-binding -p vkrgss-ruby-sys --locked --offline'
```

The collection stress test of `vkrgss-binding` (`tests/stress.rs`) is the one that finds such
faults; a fix is a `RB_GC_GUARD` recorded as a patch in
[third_party/ruby-1.8.7/PATCHES.md](../third_party/ruby-1.8.7/PATCHES.md).

## Comparing with the original runtime

`cargo xtask oracle` runs one game in the original runtime and in vkrgss under the same
conditions and reports how chosen frames differ. The original runtime, the run-time package
and the game are yours and stay where they are: the command takes their locations, works on
copies of the game and writes only below its output directory.

```sh
cargo xtask oracle <game directory> --frames 10,20,200 \
    --dll <RGSS DLL> --rtp Standard=<RTP directory> --input "60:RIGHT, 100:"
```

| Option | Meaning |
|---|---|
| `--frames N[,N...]` | The frames to capture, numbered from 0 as `vkrgss --dump` numbers them. Required. |
| `--dll FILE` | The RGSS DLL that the game's `Game.ini` names with `Library=`. Required unless `--vkrgss-only` is given, except for a game of VX Ace, which runs with the DLL it has at the path of its `Library=` when the option is left out. |
| `--runtime rgss1\|rgss2\|rgss3` | The runtime of the game. Only needed when the file that `Scripts=` of `Game.ini` names is none of a `.rxdata` (RGSS1, RPG Maker XP), a `.rvdata` (RGSS2, RPG Maker VX) and a `.rvdata2` (RGSS3, RPG Maker VX Ace). |
| `--rtp NAME=DIR` | The directory of the run-time package that `Game.ini` names `NAME`, with `RTP1` to `RTP3` in a game of XP and with `RTP` in a game of VX or VX Ace. Repeatable. |
| `--input SCHEDULE\|@FILE` | The buttons held from given frames on, in the syntax described under [Running the demo](running.md#running-the-demo), or a file that holds such a schedule. |
| `--font FILE` | A font file that both runtimes draw text with, as described below. Repeatable. |
| `--builtin-scripts FILE` | The built-in scripts file for vkrgss, instead of its default location. |
| `--out DIR` | Where everything is written. Defaults to `target/oracle`. |
| `--vkrgss-only` | Run only vkrgss. Needs neither Wine nor Xvfb nor the DLL. |
| `--original-only` | Run only the original. Nothing is compared, and vkrgss is not built. Not together with `--vkrgss-only`. |
| `--no-threads` | On both sides, make `Thread.new` run its block at once in the caller and `sleep` return at once inside such a block. For a game whose Ruby threads end the original under Wine. |
| `--locale LOCALE` | `LC_ALL` of the original, which decides the code page its file names are read in: `zh_CN.UTF-8`, the default, for a Chinese game, `ja_JP.UTF-8` for a Japanese one, `en_US.UTF-8` for the English games of VX Ace that were measured. |
| `--timeout SECONDS` | How long the original may take to reach one frame. Defaults to 120. |

Each side runs a copy of the game, `<out>/original/game` and `<out>/vkrgss/game`, with a probe
script added in front of the scripts of its archive. The probe seeds the random numbers with
a fixed value, answers `Input.press?`, `Input.trigger?`, `Input.repeat?`, `Input.dir4` and
`Input.dir8` from the schedule instead of the keyboard, and counts frames as the host of vkrgss
does: one for every `Graphics.update` and `d` for `Graphics.transition(d)`. The game directory
given is never written to, and the output directory may not be inside it. Every file of a
copy has the modification time of its source, on Linux and on macOS alike, and the files the
harness writes there itself (the script archive with the probe, the DLL it places) have
2000-01-01 00:00:00 UTC, so that a game that loads its newest save loads the same one on
every platform and on both sides.

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

A game of RPG Maker VX Ace (RGSS3) is run with the rules of VX and these differences:

- Its run-time package is registered under `HKLM\Software\Enterbrain\RGSS3\RTP`, by the
  name `RTP=` of its `Game.ini` gives (`--rtp RPGVXAce=<directory>`); a game with `RTP=`
  empty needs none. Its encrypted archive is `Game.rgss3a`.
- The run-time package holds no DLL: the game ships its own at the path that `Library=` of
  its `Game.ini` names, for example `Library=System\RGSS300.dll`, relative to the game
  directory. The command puts the DLL there in the copy of the original's side, each folder
  of the path matched without regard to case and made when missing, and nowhere else. With
  `--dll` that file replaces the game's; without it the game's own file is used. A missing
  `Library=` line, a path with a drive, a leading separator, `.` or `..`, and a missing file
  without `--dll` end the run before Wine starts.
- The screen is 544 by 416 at the start, and a game may resize it with
  `Graphics.resize_screen` at any time. The probe reports the size of the screen with each
  frame it holds, and the command captures the window of that size. vkrgss dumps its pictures
  at the size it has; pictures of different sizes are reported as not comparable.
- The probe is a third one, written for Ruby 1.9.2. It counts frames as the probe of VX does,
  which was measured to be the same in VX Ace, and takes a button as its Symbol (`:DOWN`),
  as the constants of `Input` are in RGSS3, or as its Integer of RGSS2 (2), as the original
  does; anything else is not pressed.
- Its default font, VL Gothic, is an open font that the run-time package has in its `Fonts`
  folder: `--font <RTP directory>/Fonts/VL-Gothic-Regular.ttf` gives both sides that very
  file.
- Ruby threads run in the original of VX Ace, at times that are not repeatable;
  `--no-threads` is not needed for the game to run, and makes its runs repeatable.
- vkrgss runs such a game in its guest executable `vkrgss-guest-rgss3`, which the command builds
  together with vkrgss. Until vkrgss has the classes of RGSS3, a real game of VX Ace ends on the
  vkrgss side with an error, which the command reports as the failure of that side;
  `--original-only` runs the original alone.

A game that ships its data in `Game.rgssad`, or `Game.rgss2a` (both RGSSAD version 1), or
`Game.rgss3a` (RGSSAD version 3) is compared as it is: the probe is added to the script
archive inside the encrypted archive of each copy, every other entry is written back
unchanged, and no loose script archive is added, because the original reads its scripts only
from the archive when there is one. An archive of version 3 is written back with its seed,
the key and the place of every entry and the end of its table, so that only the data of the
script archive differ. To make such a game from an
unpacked one, `cargo xtask pack-rgssad <game directory>` writes every file below the
directory's `Data` and `Graphics` folders into `Game.rgssad`, under its path with
backslashes, and then removes the two folders. It changes the directory it is given, so give
it a copy; it refuses a directory that already has an archive or has no `Game.ini`.

The original side needs `wine`, `wineboot`, `wineserver`, `Xvfb` and `bwrap` on `PATH` and a
game that has its `Game.exe`. Every Wine process runs inside a `bwrap` sandbox in which
`/dev/dri` is an empty directory and there is no network: software rendering alone does not
keep the run from the GPUs of the machine, because a game that plays Ogg or MP3 does it
through GStreamer, whose hardware plugins probe VA-API. VA-API is also pointed at a driver
directory that does not exist, and GStreamer, for every runtime, at plugin directories of
its own, `<out>/gstreamer/plugins-<n>`, made afresh on every run with links to the plugins of
the system's GStreamer directories without the hardware plugins (`va`, `vaapi`, `qsv`,
`msdk`, `vulkan`, `kms`, `opengl`, `qmlgl`, `nvcodec`, `video4linux2`, `v4l2codecs`), and at a
registry of its own, `<out>/gstreamer/registry.bin`, so that the user's registry is not
rewritten. These are lines behind the sandbox, which stays the requirement. A game that needs the network on the original side is not
compared. The command starts a virtual display of its own, makes a Wine prefix at
`<out>/prefix` on the first run (which takes a while), registers the run-time packages and
replacements for common game fonts in it, and runs `Game.exe` with software rendering only.
At each frame asked for the probe holds the game until the command has read the picture of
the game window from the display; after the last one the game ends. A frame that falls
inside a transition is captured when the transition has ended, because the original cannot
be held in the middle of one. In a game of VX or VX Ace such a frame is a late frame, unless
it is the last frame of the transition, whose picture is the one the transition ends with: the
picture of the original is kept, the report lists the frame as "not comparable: inside a transition",
and it counts neither as equal nor as different. What Wine and the game print is in `<out>/original/wine.log`,
which is where to look when the command reports that a frame was not reached. vkrgss runs
headless on the CPU device unless the Vulkan loader is already restricted to other drivers.

Text can only be compared when both runtimes draw it with the same font file. `--font FILE`,
which may be given several times, names such a file, a TrueType or OpenType font or a
collection: the files are copied to `<out>/fonts`, which vkrgss searches before the fonts of the
system and sizes by their own line boxes (`--font-own-metrics`), as the original does, and into the font directory of the prefix, where they are registered, and the common
game font names (`SimHei`, `黑体`, `SimSun`, `MS PGothic` and the like) are replaced in the
original by the family name read from the first file. A file that cannot be read or is not a
font ends the run before either runtime starts. Without `--font` the original draws those
names with whatever `Noto Sans CJK` the machine has and vkrgss with its own choice, and the
command prints a line saying that text is not comparable.

The command prints one line per frame and writes:

| File | Contents |
|---|---|
| `<out>/frames/<n>-original.png` | frame `n` of the original |
| `<out>/frames/<n>-vkrgss.png` | frame `n` of vkrgss |
| `<out>/frames/<n>-diff.png` | the absolute difference of red, green and blue, multiplied by 32 |
| `<out>/summary.json` | for every frame the three file names, `differing_pixels` and `largest_difference` (of any colour channel), or `not_comparable` with the reason: a late frame, or two pictures of different sizes; `totals` with the numbers of frames that are equal, different and not comparable; `no_threads` when that option was used |

It exits with status 0 when both runs completed, whatever the differences are; a missing
prerequisite or a run that fails is an error that names what was missing or what happened.
With `--vkrgss-only` or `--original-only` there is nothing to compare, and the summary lists
the pictures of the side that ran.

The limits of the original's side:

- A Ruby thread ends the original under Wine, in XP and in VX. `--no-threads` lets such a game
  go on, but changes it: what the thread did over time happens at once. The option applies to
  both sides, and the report says that it was used. In VX Ace threads run, at times that are
  not repeatable.
- A frame inside a transition cannot be held. In a game of VX or VX Ace it is a late frame,
  as above; in a game of XP its picture is whatever the window shows when the transition has
  ended.
- The game window is the topmost visible window of the size of the frame: for XP and VX the
  fixed frame size of the runtime, so that a game of theirs that changes the size of its
  window (`Graphics.resize_screen` of VX, a resizer of its own) cannot be captured after
  that; for VX Ace the size the probe
  reports, so that `Graphics.resize_screen` is followed.
- Real key presses, F12 and the reset are not sent: the probe answers `Input` on both sides.
  Neither `Graphics.play_movie` nor audio is compared.
The pictures show the game's and the run-time package's art, so they stay out of the
repository, like everything else below `target/`.

The tests of the harness run the vkrgss side on games they generate; those of VX Ace, which
need the guest executable of RGSS3, run where the tests of `crates/vkrgss/tests/rgss3.rs` do,
with built-in scripts that are the tests' own stand-in. The one test of the
original side is ignored by default, because it needs Wine, Xvfb, `bwrap` and files that are not in the
repository; it captures frames 10 and 20 of a real game below `target/oracle-test`. Its three
variables hold absolute paths:

```sh
VKRGSS_ORACLE_DLL=<RGSS DLL> VKRGSS_ORACLE_RTP=<RTP directory> VKRGSS_ORACLE_GAME=<game directory> \
    cargo test -p xtask -- --ignored the_original_is_captured
```

## Comparing the platforms

`cargo xtask xarch` runs the comparison games in vkrgss alone, on this machine, on the Mac or
on an Android phone, and compares two such runs: with the same game, frames, input, run-time
packages, fonts and built-in scripts, every frame and every save is to be identical on x86_64
Linux, macOS and the phones.

```sh
cargo xtask xarch run <out>                       # this machine, or the Mac
cargo xtask xarch android <serial> <out>          # a phone on USB, as `adb devices` lists it
cargo xtask xarch compare <out of this machine> <out of another>
```

The same games are the regression check of a change not meant to change output, with
`xarch baseline` and `xarch check` against a recorded run of vkrgss
([building.md](building.md#output-of-the-comparison-games)).

The games are listed in `local/xarch.toml` of the workspace, which is not committed, or in the
file that `--list FILE` names; names of games after the output directory run only those.
[`xtask/xarch.example.toml`](../xtask/xarch.example.toml) shows the keys; paths in the file are
relative to its directory. Each game names its directory, the frames to capture, the schedule
of `--input`, the run-time packages of `[rtp]` it uses, and whether it draws text with the
fonts of `fonts`; `[builtin_scripts]` names the built-in scripts file of each runtime, which
both kinds of run are given, so that they run the same scripts.

`run` runs each game as `cargo xtask oracle --vkrgss-only` does, into `<out>/<game>/`.
`android` builds the APK and installs it as `android-run` does, pushes the run-time packages,
the fonts and the built-in scripts into `files/xarch/` of the app (again only what changed),
and runs each game on the phone: the copy that the oracle makes for vkrgss, probe included,
with a `vkrgss.toml` that asks for the same frames and options, `font_own_metrics` with the
fonts. The copy as the run left it, with the saves, comes back to `<out>/<game>/vkrgss/game`
and the frames to `<out>/<game>/frames/`, where the oracle puts them, with the app's log and
logcat beside them. The phone must be unlocked; the command wakes its screen before each game,
and the app keeps it on while a game runs. `compare` decodes every frame of vkrgss of the first
directory and compares its pixels with the same frame of the second, compares every save file
the game wrote byte for byte, prints each difference and a line per game, and fails when
anything differs or is missing.

Text is drawn with the families each system has where a game asks for one the fonts of the list
do not cover: Liberation and DejaVu on Linux, the fonts of macOS on the Mac, `Roboto`,
`Noto Serif` and `Droid Sans Mono` on Android. The comparison games draw their text with the
fonts of the list, or show none of the other families in the frames compared.

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
