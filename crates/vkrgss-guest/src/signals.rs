//! The signal settings of a guest: the mask of the threads beside the interpreter's, and what
//! a guest process asks of the kernel.
//!
//! This is platform code for Linux, Android and macOS, the platforms the interpreter builds
//! for.
//!
//! Ruby switches its threads from a SIGVTALRM handler, driven by a process-wide timer, and
//! the kernel may deliver that signal to any thread that does not block it. So the main
//! thread blocks it before any other thread exists, every thread inherits that, and only the
//! interpreter thread takes it back at the start of the game.
//!
//! A script takes SIGINT as `Interrupt`, so in script mode the main thread blocks SIGINT
//! once the interpreter thread exists, and the kernel delivers it to a thread of the
//! interpreter.

use std::mem::MaybeUninit;
use std::ptr;

fn change_mask(signal: libc::c_int, how: libc::c_int) {
    let mut set = MaybeUninit::<libc::sigset_t>::uninit();
    // SAFETY: `set` is valid for writes; `sigemptyset` initialises it before `sigaddset` and
    // `pthread_sigmask` read it, the callers pass a valid signal number, and a null old set
    // is allowed. The call changes only the mask of the calling thread.
    let status = unsafe {
        libc::sigemptyset(set.as_mut_ptr());
        libc::sigaddset(set.as_mut_ptr(), signal);
        libc::pthread_sigmask(how, set.as_ptr(), ptr::null_mut())
    };
    assert_eq!(status, 0, "pthread_sigmask cannot change the mask");
}

/// Blocks SIGVTALRM in the calling thread and in every thread it spawns from now on.
pub fn block_virtual_timer() {
    change_mask(libc::SIGVTALRM, libc::SIG_BLOCK);
}

/// Lets SIGVTALRM reach the calling thread again; for the interpreter thread.
pub fn unblock_virtual_timer() {
    change_mask(libc::SIGVTALRM, libc::SIG_UNBLOCK);
}

/// Blocks SIGINT in the calling thread and in every thread it spawns from now on; for the
/// main thread of script mode once the interpreter thread exists.
pub fn block_interrupt() {
    change_mask(libc::SIGINT, libc::SIG_BLOCK);
}

/// Has this process ended when the parent process that started it ends; for the guest
/// process, whose game may be caught in a loop. See [`crate::parent_watch`].
pub fn die_with_parent() {
    crate::parent_watch::die_with_parent();
}

/// Makes the kernel take the children of this process away when they end, so that none of
/// them stays behind as a zombie; for a guest process, whose children are its copies.
pub fn ignore_children() {
    // SAFETY: `SIG_IGN` is a valid disposition for SIGCHLD, and setting it runs no handler.
    let previous = unsafe { libc::signal(libc::SIGCHLD, libc::SIG_IGN) };
    assert_ne!(previous, libc::SIG_ERR, "signal cannot ignore SIGCHLD");
}
