# Tasks

## 1. Vendored tree (`third_party/ruby-1.9.2`)

- [x] 1.1 Check the SHA-256 of `../../ref/tools/ruby192-feasibility/ruby-1.9.2-p0.tar.gz` against the specification and copy the subset of decision 1 at its release paths: the files the build opens, the files of the upstream tests, the provenance files, the licence files. Verify: `sha256sum` prints the hash of the requirement "Vendored source matches upstream"; no file of `enc/` other than those of decisions 3 and 5 and their headers is present.
- [x] 1.2 Write `MANIFEST.sha256` (upstream hashes), apply the eight patches of the study, and write `PATCHES.md` and `patches/` in the format of 1.8.7: one entry per file, the names of decision 2, each with its reason. Add the test function for this tree to `crates/vkxp-ruby-sys/tests/manifest.rs`. Verify: `cargo test -p vkxp-ruby-sys --locked --test manifest` passes, and fails naming the file when one vendored file is changed by hand and when a file is added (both undone afterwards).
- [x] 1.3 Write `third_party/ruby-1.9.2/README.vkxp.md` and the section of `THIRD-PARTY.md`. Verify: both name the release, its hash, the terms, the parts with terms of their own and the three record files; every link of the new section resolves.

## 2. Crate and build (`crates/vkxp-ruby192-sys`, `xtask`)

- [x] 2.1 Add the package to the workspace with `links = "vkxp_ruby192"`, the configuration header and the build script of decision 3; add patch `static-encodings` and the crate's table of decision 5 with their log entries. Verify: the scenario "No Ruby on the machine"; `cargo build -p vkxp-ruby192-sys --locked` succeeds with `PATH` holding no `ruby`; `cargo test -p vkxp-ruby-sys --locked --test manifest` still passes.
- [x] 2.2 Make the crate empty on other targets (decision 4). Verify: the scenario "Another target"; `cargo check -p vkxp-ruby192-sys --locked --target aarch64-unknown-linux-gnu` succeeds and prints the warning where that Rust target is installed, and where it is not, the task says so in its commit message and the CI job `gate-aarch64` is the check.
- [x] 2.3 Write `ffi.rs` (decision 6) and the tests `layout` and `symbols`. Verify: the scenarios "Headers and declarations agree" and "Exported symbols of a linked executable"; `cargo test -p vkxp-ruby192-sys --locked --test layout --test symbols` passes.
- [x] 2.4 Add the rule on the two interpreters and the new guest crate to `xtask/src/deps.rs`. Verify: both scenarios of "The two interpreters are never linked together"; `cargo test -p xtask --locked deps` passes.

## 3. Start-up and the safe layer

- [x] 3.1 Add patch `host-stack-limit` with its log entry; write `interp.rs` and `signals.rs` with the start-up of decision 7. Verify: the scenarios of "Start-up state", "Dedicated thread and bounded recursion", "Process signals are left alone", "Fixed hash seed" and "No timer thread before a script creates a thread"; `cargo test -p vkxp-ruby192-sys --locked --test interpreter --test stack_limit --test signals_default --test signals_host --test hash_seed --test timer_thread` passes.
- [x] 3.2 Write `protect.rs`, `call.rs`, `host.rs`, `root.rs` and the crate documentation with the table of differences. Verify: the scenarios of "The Rust API of vkxp-ruby-sys" other than "Headers and declarations agree", and of "Zlib is correct on 64-bit"; `cargo test -p vkxp-ruby192-sys --locked --test protect --test calling --test classes` passes and `RUSTDOCFLAGS='-D warnings' cargo doc -p vkxp-ruby192-sys --no-deps --locked` succeeds.
- [x] 3.3 Encodings at start-up and at first use. Verify: the scenarios of "Encodings as the original runtime has them"; `cargo test -p vkxp-ruby192-sys --locked --test encodings` passes.

## 4. Values of the original

- [x] 4.1 The test `original`: identity, `Marshal`, random numbers, float text and rounding. Verify: the scenarios of "Interpreter identity", "Marshal as the original runtime writes it", "Random numbers as the original runtime gives them" and "Float text and rounding as the original runtime gives them", with every expected value copied from the specification; `cargo test -p vkxp-ruby192-sys --locked --test original` passes.

## 5. Upstream suites

- [ ] 5.1 The test executable, the test libraries under `tests/testlib/` and the harness (decision 10); `bootstraptest`. Verify: the scenario "Bootstrap tests"; `cargo test -p vkxp-ruby192-sys --locked --test upstream bootstrap` passes.
- [ ] 5.2 The selection of `test/ruby` and `tests/upstream.allow`. Verify: the scenarios "Core suite", "Reasons" and "SIGINT ignored by the caller"; `cargo test -p vkxp-ruby192-sys --locked --test upstream --test upstream_signals` passes, also when started with SIGINT ignored; every entry of the list has one of the reasons of decision 10.

## 6. Collector stress and copies of the process

- [ ] 6.1 The switch `VKXP_GC_STRESS`. Verify: the scenarios "Every allocation" and "Off"; `cargo test -p vkxp-ruby192-sys --locked --test gc_stress_on --test gc_stress_off` passes.
- [ ] 6.2 The stressed selection in the workspace tests, the step `suites-1.9.2` of `cargo xtask stress` and the entries of `xtask/src/stress.skip` (decision 11). Verify: the scenarios "Stressed selection in the workspace tests" and "Slow files"; `cargo test -p vkxp-ruby192-sys --locked --test upstream_stress` passes; `cargo test -p vkxp-ruby192-sys --locked --test upstream_stress -- --ignored` passes on the development machine; `cargo test -p xtask --locked` passes, `the_stress_run_is_not_a_step` among it unchanged.
- [ ] 6.3 The tests `fiber_host_frames`, `fork_frames` and `fork_frames_stressed`. Verify: the scenario "Host frames across Fiber switches" and both scenarios of "A copy of the process between two frames"; `cargo test -p vkxp-ruby192-sys --locked --test fiber_host_frames --test fork_frames --test fork_frames_stressed` passes ten times in a row.

## 7. Documentation

- [ ] 7.1 `AGENTS.md` (the crates that may hold `unsafe`), `README.md` (the table of crates), `docs/building.md` (the second interpreter, its target), `docs/development.md` (the stressed selection in the tests, the step `suites-1.9.2`). Verify: each names `vkxp-ruby192-sys`; none says that a game of VX Ace runs; `cargo xtask stress` is described with the new step and without a need for sanitizers in it.

## 8. Integration check

- [ ] 8.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate vendor-ruby-1-9-2 --strict` passes.
