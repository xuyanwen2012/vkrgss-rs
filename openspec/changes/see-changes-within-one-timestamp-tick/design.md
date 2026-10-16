# Design

## Context

`Listing::read` takes the directory's modification time, then its entries; `is_current`
compares the time now with the stored one. Measured on the Jetson: `stat` of a directory
after three `touch` calls in a row gives times 4 ms apart (…14.5606, …14.5646, …14.5686),
so two changes within 4 ms share one time.

## Decisions

- **Racy listings.** `Listing` also stores when it was read (`SystemTime::now()` taken
  just before the modification time). A listing is current only if the directory's time is
  unchanged and that time lies before the moment of reading by more than a margin. The
  margin covers the coarsest clock the program may meet: use 2 seconds, which also covers
  file systems with 1-second or 2-second timestamps (FAT, some network file systems). A
  listing that is not trusted is simply read again at the next lookup; it is still cached,
  so the cost is one `read_dir` per lookup in a directory changed in the last 2 seconds.
- **Clock jumps.** If the directory's time lies in the future relative to the moment of
  reading, the listing is not trusted either (same rule).
- **Test.** A unit test of `dir.rs` (or `files.rs`) that does not rely on the kernel's
  clock: list a directory, set its modification time to the stored moment of reading with
  `File::set_modified` on the directory (or `filetime`-like code through `std`), add a file,
  set the time back to the same value, and check that the file is found. Without the fix the
  test fails on every machine. The two tests that failed on the Jetson stay as they are.
- **Scope.** Only the listing cache. The archive and the rest of the lookup are unchanged.

## Out of scope

Watching directories with inotify.

## Follow-ups

None yet.
