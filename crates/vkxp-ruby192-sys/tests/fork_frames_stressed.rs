//! As `fork_frames`, with a collection at every allocation: a shorter script of the same
//! shape runs under `GC.stress = true`, and the host function copies the process once.
//!
//! One test per file: the process forks, and has one interpreter.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use common::frames;

#[test]
fn the_copy_finishes_with_the_digest_of_the_original_under_collector_stress() {
    // Three Fibers that each yield five times, and one copy at the third loop.
    let outcome = frames::run_with_copies(5, true, 3, 3);
    assert_eq!(outcome.copies.len(), 1, "one copy");
    let (frame, digest, clean) = &outcome.copies[0];
    assert_eq!(*frame, 3);
    assert_eq!(digest, &outcome.original, "the copy disagrees");
    assert!(clean, "the copy did not exit with 0");
    assert!(
        outcome.original.starts_with("6 ") && outcome.original.contains(" 15 "),
        "{}",
        outcome.original
    );
}
