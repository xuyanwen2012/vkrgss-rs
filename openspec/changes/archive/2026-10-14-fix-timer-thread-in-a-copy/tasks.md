# Tasks

## 1. Handover of the GVL

- [x] 1.1 Measure the cause with temporary prints and record it in `design.md`. Verify: the measured facts are in `design.md`, and no print is left in the vendored source (`git diff` of `third_party/ruby-1.9.2/thread_pthread.c` is empty).
- [x] 1.2 Patch `thread.c` (`gvl-handover`, entry 17 of `PATCHES.md`), call `ruby_vkxp_gvl_atfork` in `restart_thread_timer()`, and add `busy_main_thread.rs`. Verify: `cargo test -p vkxp-ruby-sys --test manifest` and `cargo test -p vkxp-ruby192-sys` pass; `fork_busy_main_after_copy` passes 100 runs in a row on its own and 20 in a row while `cargo test --workspace` runs beside it.

## 2. Integration check

- [x] 2.1 Run the full gate. Verify: `CCACHE_DISABLE=1 VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate` exits 0 and `openspec validate fix-timer-thread-in-a-copy --strict` passes.
