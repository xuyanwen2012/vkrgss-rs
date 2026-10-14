//! As `fork_frames`, with `restart_thread_timer()` called in every copy: in a process whose
//! script never made a thread the call changes nothing.
//!
//! One test per file: the process forks, and has one interpreter.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::sync::atomic::Ordering;

use common::frames;

#[test]
fn every_copy_that_restarts_the_timer_finishes_with_the_digest_of_the_original() {
    frames::RESTART_TIMER.store(true, Ordering::SeqCst);
    let outcome = frames::run_with_copies(12, false, 1, 12);
    assert_eq!(outcome.copies.len(), 12, "one copy per chosen frame");
    for (frame, digest, clean) in &outcome.copies {
        assert_eq!(
            digest, &outcome.original,
            "the copy made at frame {frame} disagrees"
        );
        assert!(clean, "the copy made at frame {frame} did not exit with 0");
    }
    assert!(
        outcome.original.starts_with("13 ") && outcome.original.contains(" 36 "),
        "{}",
        outcome.original
    );
}
