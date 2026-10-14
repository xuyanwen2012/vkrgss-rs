//! A copy of the process made by `fork` between two frames of a script with Fibers runs the
//! script on to the same result as the process it was copied from.
//!
//! The copy is made by a host function on the interpreter thread while the main thread is
//! the only Ruby thread. Nothing is done about the interpreter's threads in the copy: there
//! is no timer thread to restart, and `rb_thread_atfork` is not called, which would reseed
//! the random numbers.
//!
//! One test per file: the process forks, and has one interpreter.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use common::frames;

#[test]
fn every_copy_finishes_with_the_digest_of_the_original() {
    // Three Fibers that each yield twelve times, and a copy at each of the loops 1 to 12.
    let outcome = frames::run_with_copies(12, false, 1, 12);
    assert_eq!(outcome.copies.len(), 12, "one copy per chosen frame");
    for (frame, digest, clean) in &outcome.copies {
        assert_eq!(
            digest, &outcome.original,
            "the copy made at frame {frame} disagrees"
        );
        assert!(clean, "the copy made at frame {frame} did not exit with 0");
    }
    // The script ran as written: 13 loops, 36 steps.
    assert!(
        outcome.original.starts_with("13 ") && outcome.original.contains(" 36 "),
        "{}",
        outcome.original
    );
}
