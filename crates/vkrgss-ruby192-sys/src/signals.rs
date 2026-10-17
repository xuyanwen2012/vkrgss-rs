//! Saving and restoring the signal state that `ruby_init` replaces.
//!
//! This is platform code for x86_64 and aarch64 Linux and for aarch64 macOS, the targets the
//! crate builds for.

use std::mem::MaybeUninit;
use std::ptr;

use libc::c_int;

/// The signals whose dispositions `ruby_init` changes: it installs handlers for hang-up,
/// interrupt, quit, the two user-defined signals, alarm, terminate, bus error, segmentation
/// fault and broken pipe, replacing the handler, flags and mask of whatever the process had,
/// and it gives the child signal its default disposition when it was ignored.
pub const SIGNALS_TAKEN_BY_RUBY: [c_int; 11] = [
    libc::SIGHUP,
    libc::SIGINT,
    libc::SIGQUIT,
    libc::SIGUSR1,
    libc::SIGUSR2,
    libc::SIGALRM,
    libc::SIGTERM,
    libc::SIGBUS,
    libc::SIGSEGV,
    libc::SIGPIPE,
    libc::SIGCHLD,
];

unsafe extern "C" {
    /// Whether the main thread is the only living Ruby thread.
    fn rb_thread_alone() -> c_int;
    /// Forgets the timer thread without stopping it. The interpreter calls it itself after
    /// its own `fork`.
    fn rb_thread_reset_timer_thread();
    /// Forgets the threads the copied process counted as waiting for the interpreter's
    /// lock (patch `gvl-handover`). The interpreter calls it itself after its own `fork`.
    fn ruby_vkxp_gvl_atfork();
}

/// Makes script threads work in a copy of the process that `fork` has just made. The timer
/// thread of the copied process does not exist in the copy; this forgets it and makes its
/// lock and condition variable afresh (patch `timer-thread-in-a-copy`), and the interpreter
/// then starts one when a script next creates a thread (patch `lazy-timer-thread`). It also
/// forgets the threads of the copied process that waited for the interpreter's lock, which
/// a thread of the copy that gives way at the end of its time slice would otherwise wait
/// for (patch `gvl-handover`). To be called in the copy on the interpreter thread, which is
/// the one thread a copy has. Nothing changes when no script ever made a thread.
pub fn restart_thread_timer() {
    // SAFETY: called on the interpreter thread, which holds the interpreter's lock and is
    // the one thread of the copy, as the interpreter itself calls both after a `fork`; they
    // clear variables and make a lock and condition variables afresh.
    unsafe {
        rb_thread_reset_timer_thread();
        ruby_vkxp_gvl_atfork();
    }
}

/// Whether a copy of the process made by `fork` now would lack threads a script has made:
/// true exactly while a Ruby thread other than the main one is alive. A Ruby thread of 1.9
/// is a thread of the operating system, and a copy has the calling thread only. To be
/// called on the interpreter thread.
pub fn threads_lost_in_a_copy() -> bool {
    // SAFETY: called on the interpreter thread, which holds the interpreter's lock; the
    // function reads the count of living threads.
    unsafe { rb_thread_alone() == 0 }
}

/// `struct sigaction` as the Linux kernel defines it for `rt_sigaction`, alike on x86_64 and
/// aarch64.
///
/// The dispositions go through the system call directly instead of through `sigaction` of
/// glibc, because glibc adds its own restorer and the `SA_RESTORER` flag to everything it
/// installs. A default disposition that was never touched has neither, so putting it back
/// through glibc would not leave it exactly as it was.
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Clone, Copy)]
struct KernelAction {
    handler: usize,
    flags: libc::c_ulong,
    restorer: usize,
    mask: u64,
}

/// The size of the kernel's signal set, which `rt_sigaction` wants as its last argument.
#[cfg(target_os = "linux")]
const KERNEL_SIGSET_SIZE: usize = size_of::<u64>();

#[cfg(target_os = "linux")]
fn get(signal: c_int) -> KernelAction {
    let mut action = MaybeUninit::<KernelAction>::zeroed();
    // SAFETY: a null new action only queries; `action` is valid for the kernel's structure to
    // be written, and every signal passed here is a valid, catchable signal number.
    let status = unsafe {
        libc::syscall(
            libc::SYS_rt_sigaction,
            signal,
            ptr::null::<KernelAction>(),
            action.as_mut_ptr(),
            KERNEL_SIGSET_SIZE,
        )
    };
    assert_eq!(status, 0, "rt_sigaction cannot query signal {signal}");
    // SAFETY: the successful call above initialised it.
    unsafe { action.assume_init() }
}

#[cfg(target_os = "linux")]
fn set(signal: c_int, action: &KernelAction) {
    // SAFETY: `action` is a disposition this process had before, read by `get` with its
    // restorer; installing it gives the process nothing it did not already have.
    let status = unsafe {
        libc::syscall(
            libc::SYS_rt_sigaction,
            signal,
            ptr::from_ref(action),
            ptr::null_mut::<KernelAction>(),
            KERNEL_SIGSET_SIZE,
        )
    };
    assert_eq!(status, 0, "rt_sigaction cannot set signal {signal}");
}

/// `struct sigaction` of the C library of macOS, which has no `rt_sigaction`. Its `sigaction`
/// adds no restorer and no flag to what it installs, so a disposition read and set again
/// through it stays exactly as it was.
#[cfg(target_os = "macos")]
type KernelAction = libc::sigaction;

#[cfg(target_os = "macos")]
fn get(signal: c_int) -> KernelAction {
    let mut action = MaybeUninit::<KernelAction>::zeroed();
    // SAFETY: a null new action only queries; `action` is valid for the structure to be
    // written, and every signal passed here is a valid, catchable signal number.
    let status = unsafe { libc::sigaction(signal, ptr::null(), action.as_mut_ptr()) };
    assert_eq!(status, 0, "sigaction cannot query signal {signal}");
    // SAFETY: the successful call above initialised it.
    unsafe { action.assume_init() }
}

#[cfg(target_os = "macos")]
fn set(signal: c_int, action: &KernelAction) {
    // SAFETY: `action` is a disposition this process had before, read by `get`; installing it
    // gives the process nothing it did not already have.
    let status = unsafe { libc::sigaction(signal, ptr::from_ref(action), ptr::null_mut()) };
    assert_eq!(status, 0, "sigaction cannot set signal {signal}");
}

/// The signal mask of the calling thread.
fn thread_mask() -> libc::sigset_t {
    let mut mask = MaybeUninit::<libc::sigset_t>::zeroed();
    // SAFETY: a null new set only queries, and `mask` is valid for the set to be written.
    let status = unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, ptr::null(), mask.as_mut_ptr()) };
    assert_eq!(status, 0, "pthread_sigmask cannot query the mask");
    // SAFETY: the successful call above initialised it.
    unsafe { mask.assume_init() }
}

/// The dispositions of the process and the signal mask of the interpreter thread at one
/// moment.
pub(crate) struct State {
    taken: [KernelAction; SIGNALS_TAKEN_BY_RUBY.len()],
    /// SIGVTALRM, for which the interpreter installs an empty handler: it wakes a Ruby
    /// thread that blocks by sending it this signal.
    wakeup: KernelAction,
    mask: libc::sigset_t,
}

impl State {
    /// Records the dispositions of [`SIGNALS_TAKEN_BY_RUBY`] and of SIGVTALRM, and the
    /// signal mask of the calling thread, which must be the interpreter thread.
    pub(crate) fn capture() -> Self {
        Self {
            taken: SIGNALS_TAKEN_BY_RUBY.map(get),
            wakeup: get(libc::SIGVTALRM),
            mask: thread_mask(),
        }
    }

    /// Puts the dispositions and the mask back. `ruby_init` blocks every signal on the
    /// interpreter thread, because it means its timer thread to receive them; with the
    /// dispositions back in the host's hands the thread gets the mask it started with.
    ///
    /// SIGVTALRM goes back to the host as well. The interpreter installs its handler again
    /// when a script creates the first Ruby thread (patch
    /// `thread-wakeup-signal-with-first-thread`), and never sends the signal before.
    pub(crate) fn restore(&self) {
        for (signal, action) in SIGNALS_TAKEN_BY_RUBY.iter().zip(&self.taken) {
            set(*signal, action);
        }
        set(libc::SIGVTALRM, &self.wakeup);
        // SAFETY: `mask` is a set this thread had before; a null old set is allowed.
        let status =
            unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, &self.mask, ptr::null_mut()) };
        assert_eq!(status, 0, "pthread_sigmask cannot set the mask");
    }
}
