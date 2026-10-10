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

/// Asks the kernel to end this process when the thread of the parent process that started it
/// ends; for the guest process, whose game may be caught in a loop.
pub(crate) fn die_with_parent() {
    // SAFETY: `PR_SET_PDEATHSIG` takes a signal number as its one argument and changes only
    // a setting of the calling process.
    let status = unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) };
    assert_eq!(status, 0, "prctl cannot set the parent-death signal");
}

/// Makes the kernel take the children of this process away when they end, so that none of
/// them stays behind as a zombie; for a guest process, whose children are its copies.
pub(crate) fn ignore_children() {
    // SAFETY: `SIG_IGN` is a valid disposition for SIGCHLD, and setting it runs no handler.
    let previous = unsafe { libc::signal(libc::SIGCHLD, libc::SIG_IGN) };
    assert_ne!(previous, libc::SIG_ERR, "signal cannot ignore SIGCHLD");
}
