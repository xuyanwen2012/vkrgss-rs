# Design

## Context

- `xtask/src/oracle` starts `Xvfb` on a display number it picks by looking for a free one,
  then connects with `x11rb`. Two runs started together can pick the same number; one `Xvfb`
  fails to start and its run later reports `the display :N: Connection refused`.
- `Xvfb -displayfd <fd>` makes the server choose the first free number atomically and write it
  to the file descriptor when it is ready to accept connections.
- The Wine server is per prefix, and prefixes live below each run's output directory.

## Decisions

- Start `Xvfb` with `-displayfd` on a pipe, read the number with a timeout, and use it for the
  connection and for Wine's `DISPLAY`. Reading the number also replaces any wait-until-ready
  polling.
- Keep the child handle and kill only that process at the end; shut the Wine server down with
  the run's own prefix in the environment, as now.
- The test for the first scenario uses a small stand-in program in place of `Xvfb` where no
  real server can run, plus an ignored test with the real `Xvfb` like the other original-side
  test. The flow's sandbox cannot start `Xvfb` (no write access to `/tmp`); the coordinator
  runs the ignored test.
