//! Saving and restoring the signal dispositions that `ruby_init` replaces.
//!
//! This is platform code for x86_64 Linux, the one target the crate builds for.

use std::mem::MaybeUninit;
use std::ptr;

use libc::c_int;

/// The signals whose handlers `ruby_init` installs, replacing the handler, flags and mask of
/// whatever the process had: hang-up, interrupt, quit, the two user-defined signals, alarm,
/// terminate, bus error, segmentation fault and broken pipe.
pub const SIGNALS_TAKEN_BY_RUBY: [c_int; 10] = [
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
];

unsafe extern "C" {
    /// Arms the timer that switches the interpreter's threads, if the game has made any.
    /// The interpreter calls it itself after its own `fork`.
    fn rb_thread_start_timer();
}

/// Restarts the timer of the interpreter's threads in a copy of the process that `fork` has
/// just made: timers are not inherited. To be called in the copy on the interpreter thread,
/// which is the one thread a copy has. Without a second thread of the interpreter it does
/// nothing.
pub fn restart_thread_timer() {
    // SAFETY: `rb_thread_start_timer` is called on the interpreter thread, as the
    // interpreter itself calls it after a `fork`: timers are not inherited.
    unsafe { rb_thread_start_timer() }
}

/// Whether a copy of the process made by `fork` now would lack threads a script has made.
/// Never here: the threads of Ruby 1.8 are memory of the interpreter and are copied with
/// it. To be called on the interpreter thread, as in `vkxp-ruby192-sys`, where the answer
/// depends on the script.
pub fn threads_lost_in_a_copy() -> bool {
    false
}

/// `struct sigaction` as the x86_64 Linux kernel defines it for `rt_sigaction`.
///
/// The dispositions go through the system call directly instead of through `sigaction` of
/// glibc, because glibc adds its own restorer and the `SA_RESTORER` flag to everything it
/// installs. A default disposition that was never touched has neither, so putting it back
/// through glibc would not leave it exactly as it was.
#[repr(C)]
#[derive(Clone, Copy)]
struct KernelAction {
    handler: usize,
    flags: libc::c_ulong,
    restorer: usize,
    mask: u64,
}

/// The size of the kernel's signal set, which `rt_sigaction` wants as its last argument.
const KERNEL_SIGSET_SIZE: usize = size_of::<u64>();

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

fn set(signal: c_int, action: &KernelAction) {
    // SAFETY: `action` is a disposition this process had before, read by `get` with its
    // restorer, or the plain "ignore" built in `restore_after_finalize`; installing it gives
    // the process nothing it did not already have.
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

fn same(left: &KernelAction, right: &KernelAction) -> bool {
    left.handler == right.handler && left.flags == right.flags
}

/// The dispositions of the process at one moment.
pub(crate) struct Dispositions {
    taken: [KernelAction; SIGNALS_TAKEN_BY_RUBY.len()],
    virtual_timer: KernelAction,
}

impl Dispositions {
    /// Records the dispositions of [`SIGNALS_TAKEN_BY_RUBY`] and of SIGVTALRM.
    pub(crate) fn capture() -> Self {
        Self {
            taken: SIGNALS_TAKEN_BY_RUBY.map(get),
            virtual_timer: get(libc::SIGVTALRM),
        }
    }

    /// Puts the dispositions of [`SIGNALS_TAKEN_BY_RUBY`] back. SIGVTALRM is left to Ruby.
    pub(crate) fn restore(&self) {
        for (signal, action) in SIGNALS_TAKEN_BY_RUBY.iter().zip(&self.taken) {
            set(*signal, action);
        }
    }

    /// For shutdown: stops the virtual timer that Ruby's threads arm, gives SIGVTALRM its
    /// recorded disposition back, then restores the rest.
    pub(crate) fn restore_after_finalize(&self) {
        // SAFETY: an all-zero `itimerval` is valid and means "disarm"; a null old value is
        // allowed.
        unsafe {
            let disarm: libc::itimerval = std::mem::zeroed();
            libc::setitimer(libc::ITIMER_VIRTUAL, &disarm, ptr::null_mut());
        }
        if !same(&get(libc::SIGVTALRM), &self.virtual_timer) {
            // A tick may still be pending. Ignoring the signal first discards it, so that it
            // cannot be delivered to a recorded disposition that would end the process.
            let ignore = KernelAction {
                handler: libc::SIG_IGN,
                flags: 0,
                restorer: 0,
                mask: 0,
            };
            set(libc::SIGVTALRM, &ignore);
            set(libc::SIGVTALRM, &self.virtual_timer);
        }
        self.restore();
    }
}
