# Tasks

## 1. Process functions

- [x] 1.1 Move the stub of `Kernel#open` to `prelude_process_off.rb`; stub `IO.read`, `IO.foreach`, `IO.readlines`, `IO.binread` (where it exists) with the pipe form, and `Process.kill`; check `io.c` of both interpreters for other callers of the pipe form and cover or list them in `design.md`. Verify: the scenarios of the requirement "Process functions are off by default" as tests over Ruby 1.8.7 (`vkrgss-binding/tests`) and Ruby 1.9.2 (`vkrgss-binding-rgss3/tests/process_functions_rgss3.rs`); with process functions enabled, `open("|echo x").read` returns `"x\n"`; the existing scenarios of "Process functions of Ruby 1.9 are off by default" pass unedited.

## 2. Loading code

- [ ] 2.1 `autoload` over Ruby 1.8.7 through the rule of `require`; `require` that records after the load, in `prelude_shared.rb` and `prelude_load_rgss3.rb`; `load` with `wrap`. Verify: the scenarios of the requirement "Loading code" over Ruby 1.8.7 and Ruby 1.9.2; the scenarios of "Loading code in RGSS3" and "Loading files from a script" (`script-mode`) pass unedited; the tests of symbol numbers (`run_game_symbols`, `run_game_first_uses`, `run_game_lazy_names`, `symbol_ids*`) pass unedited.

## 3. Documents

- [ ] 3.1 `docs/running.md` (the row of `--allow-process-functions` names every function that is off; the sandbox section's sentence on what the stubs leave on), `docs/decisions.md` (the Ruby section), `CHANGELOG.md`. Verify: the tests of `xtask` that check the documents pass.

## 4. Checks

- [ ] 4.1 `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0, `VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask xarch check` prints `all identical`, `openspec validate confine-loading-and-process-functions --strict` passes.
