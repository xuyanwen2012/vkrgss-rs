# Tasks

## 1. Engine (`third_party`, `vkxp-ruby-sys`)

- [x] 1.1 Vendor the Oniguruma subset with its manifest and licence text and build it from the build script. Verify: the vendored-source scenario of `specs/embedded-ruby/spec.md`; `cargo build -p vkxp-ruby-sys` without warnings from the build script.
- [x] 1.2 Patch `re.c` and the multi-byte helpers to use it, drop `regex.c` from the build, and record the patches. Verify: the pattern-that-stopped-a-game and multi-byte scenarios.
- [x] 1.3 Commit the measured results as test data and compare. Verify: the measured-results scenario; differences that come from the engine's version are listed under Follow-ups in `design.md` with their lines.
- [x] 1.4 Run the upstream suites. Verify: the interpreter-suites scenario.

## 2. Integration check

- [x] 2.1 Run the full gate. Verify: `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate embed-oniguruma --strict` passes.
