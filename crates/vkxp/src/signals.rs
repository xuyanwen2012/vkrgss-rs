//! The signal mask of the program's threads.
//!
//! This is platform code for Linux, the one platform the interpreter builds for.
//!
//! Ruby switches its threads from a SIGVTALRM handler, driven by a process-wide timer, and
//! the kernel may deliver that signal to any thread that does not block it. So the main
//! thread blocks it before any other thread exists, every thread inherits that, and only the
//! interpreter thread takes it back at the start of the game.

use std::mem::MaybeUninit;
use std::ptr;

fn change_virtual_timer(how: libc::c_int) {
    let mut set = MaybeUninit::<libc::sigset_t>::uninit();
    // SAFETY: `set` is valid for writes; `sigemptyset` initialises it before `sigaddset` and
    // `pthread_sigmask` read it, SIGVTALRM is a valid signal number, and a null old set is
    // allowed. The call changes only the mask of the calling thread.
    let status = unsafe {
        libc::sigemptyset(set.as_mut_ptr());
        libc::sigaddset(set.as_mut_ptr(), libc::SIGVTALRM);
        libc::pthread_sigmask(how, set.as_ptr(), ptr::null_mut())
    };
    assert_eq!(status, 0, "pthread_sigmask cannot change the mask");
}

/// Blocks SIGVTALRM in the calling thread and in every thread it spawns from now on.
pub(crate) fn block_virtual_timer() {
    change_virtual_timer(libc::SIG_BLOCK);
}

/// Lets SIGVTALRM reach the calling thread again; for the interpreter thread.
pub(crate) fn unblock_virtual_timer() {
    change_virtual_timer(libc::SIG_UNBLOCK);
}
