//! How a guest process ends with the process that started it, whose game may be caught in a
//! loop, and how a copy of it stops doing so.
//!
//! This is platform code. On Linux and Android the kernel does it: the parent-death signal of
//! `prctl`, SIGKILL, which ends the process when the thread of the parent that started it
//! ends. macOS has no such signal; there a thread of the guest waits in a kqueue for the
//! parent process to exit (`EVFILT_PROC` with `NOTE_EXIT`) and then raises SIGKILL, which
//! ends the whole process.

/// Asks for this process to be ended with SIGKILL when its parent ends.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn die_with_parent() {
    // SAFETY: `PR_SET_PDEATHSIG` takes a signal number as its one argument and changes only
    // a setting of the calling process.
    let status = unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) };
    assert_eq!(status, 0, "prctl cannot set the parent-death signal");
}

/// What a copy of the guest process does first: it stops being ended with its parent.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn release_copy() {
    // SAFETY: `PR_SET_PDEATHSIG` with 0 clears a setting of the calling process.
    unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, 0) };
}

/// Asks for this process to be ended with SIGKILL when its parent ends: starts the thread that
/// waits for that. The thread blocks every signal, so that SIGVTALRM, which the interpreter
/// thread is to take, never reaches it; this may be called before the main thread blocks
/// that signal.
#[cfg(target_os = "macos")]
pub fn die_with_parent() {
    use std::mem::MaybeUninit;
    use std::ptr;

    // SAFETY: `getppid` cannot fail and has no effect.
    let parent = unsafe { libc::getppid() };
    // SAFETY: `kqueue` takes no argument; the descriptor is checked below.
    let queue = unsafe { libc::kqueue() };
    assert!(queue >= 0, "kqueue cannot be created");
    let change = libc::kevent {
        ident: parent as libc::uintptr_t,
        filter: libc::EVFILT_PROC,
        flags: libc::EV_ADD | libc::EV_ONESHOT,
        fflags: libc::NOTE_EXIT,
        data: 0,
        udata: ptr::null_mut(),
    };
    // SAFETY: `queue` is open, the one change is valid for reads, and no event is asked for.
    let status = unsafe { libc::kevent(queue, &change, 1, ptr::null_mut(), 0, ptr::null()) };
    if status < 0 {
        let error = std::io::Error::last_os_error();
        // The parent has ended already.
        if error.raw_os_error() == Some(libc::ESRCH) {
            kill_self();
        }
        panic!("kevent cannot watch the parent process: {error}");
    }

    let mut all = MaybeUninit::<libc::sigset_t>::uninit();
    let mut previous = MaybeUninit::<libc::sigset_t>::uninit();
    // SAFETY: `sigfillset` initialises `all` before `pthread_sigmask` reads it, and `previous`
    // is valid for writes. The call changes only the mask of the calling thread, which the
    // thread spawned below inherits.
    let status = unsafe {
        libc::sigfillset(all.as_mut_ptr());
        libc::pthread_sigmask(libc::SIG_SETMASK, all.as_ptr(), previous.as_mut_ptr())
    };
    assert_eq!(status, 0, "pthread_sigmask cannot block the signals");
    std::thread::Builder::new()
        .name("parent watch".to_owned())
        .spawn(move || {
            // SAFETY: an all-zero `kevent` is a valid place for the kernel to write one into.
            let mut event: libc::kevent = unsafe { std::mem::zeroed() };
            // SAFETY: `queue` stays open in this process, `event` is valid for one event, and
            // no change is passed. Without a timeout the call returns an event or an error;
            // no signal interrupts it, all being blocked.
            let count = unsafe { libc::kevent(queue, ptr::null(), 0, &mut event, 1, ptr::null()) };
            if count > 0 {
                kill_self();
            }
            tracing::warn!(
                "the guest process no longer watches its parent: {}",
                std::io::Error::last_os_error()
            );
        })
        .expect("the thread that watches the parent process starts");
    // SAFETY: `previous` was initialised by the call above; the call changes only the mask of
    // the calling thread.
    let status =
        unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, previous.as_ptr(), ptr::null_mut()) };
    assert_eq!(status, 0, "pthread_sigmask cannot restore the mask");
}

/// What a copy of the guest process does first: it stops being ended with its parent. A copy
/// has neither the thread that watches nor its kqueue, which `fork` does not pass on, so there
/// is nothing to do.
#[cfg(target_os = "macos")]
pub(crate) fn release_copy() {}

#[cfg(target_os = "macos")]
fn kill_self() -> ! {
    // SAFETY: SIGKILL raised in the calling thread ends the process; nothing is touched.
    // `raise`, not `kill`: the guest sends no signal with `kill`.
    unsafe { libc::raise(libc::SIGKILL) };
    // The signal ends the process, if not already before `kill` returned.
    loop {
        std::thread::park();
    }
}
