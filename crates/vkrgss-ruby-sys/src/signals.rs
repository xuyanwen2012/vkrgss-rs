//! Saving and restoring the signal dispositions that `ruby_init` replaces.
//!
//! This is platform code for x86_64 and aarch64 Linux, for aarch64 Android and for aarch64
//! macOS, the targets the crate builds for.

use std::mem::MaybeUninit;
use std::ptr;

use libc::c_int;

#[cfg(not(target_os = "android"))]
use libc::ITIMER_VIRTUAL;

/// The virtual interval timer of the kernel, which the `libc` crate does not define for
/// Android.
#[cfg(target_os = "android")]
const ITIMER_VIRTUAL: c_int = 1;

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
/// it. To be called on the interpreter thread, as in `vkrgss-ruby192-sys`, where the answer
/// depends on the script.
pub fn threads_lost_in_a_copy() -> bool {
    false
}

/// `struct sigaction` as the Linux kernel defines it for `rt_sigaction`, alike on x86_64 and
/// aarch64.
///
/// The dispositions go through the system call directly instead of through `sigaction` of
/// glibc, because glibc adds its own restorer and the `SA_RESTORER` flag to everything it
/// installs. A default disposition that was never touched has neither, so putting it back
/// through glibc would not leave it exactly as it was.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[repr(C)]
#[derive(Clone, Copy)]
struct KernelAction {
    handler: usize,
    flags: libc::c_ulong,
    restorer: usize,
    mask: u64,
}

/// The size of the kernel's signal set, which `rt_sigaction` wants as its last argument.
#[cfg(any(target_os = "linux", target_os = "android"))]
const KERNEL_SIGSET_SIZE: usize = size_of::<u64>();

#[cfg(any(target_os = "linux", target_os = "android"))]
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

#[cfg(any(target_os = "linux", target_os = "android"))]
fn set(signal: c_int, action: &KernelAction) {
    // SAFETY: `action` is a disposition this process had before, read by `get` with its
    // restorer, or the plain "ignore" of `ignore`; installing it gives the process nothing it
    // did not already have.
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

#[cfg(any(target_os = "linux", target_os = "android"))]
fn same(left: &KernelAction, right: &KernelAction) -> bool {
    left.handler == right.handler && left.flags == right.flags
}

/// The plain "ignore", without flags or mask.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn ignore() -> KernelAction {
    KernelAction {
        handler: libc::SIG_IGN,
        flags: 0,
        restorer: 0,
        mask: 0,
    }
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
    // SAFETY: `action` is a disposition this process had before, read by `get`, or the plain
    // "ignore" of `ignore`; installing it gives the process nothing it did not already have.
    let status = unsafe { libc::sigaction(signal, ptr::from_ref(action), ptr::null_mut()) };
    assert_eq!(status, 0, "sigaction cannot set signal {signal}");
}

#[cfg(target_os = "macos")]
fn same(left: &KernelAction, right: &KernelAction) -> bool {
    left.sa_sigaction == right.sa_sigaction && left.sa_flags == right.sa_flags
}

/// The plain "ignore", without flags or mask.
#[cfg(target_os = "macos")]
fn ignore() -> KernelAction {
    // SAFETY: an all-zero `sigaction` is valid: the default disposition, no flags, no mask.
    let mut action: KernelAction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = libc::SIG_IGN;
    action
}

/// The dispositions of the process at one moment.
pub(crate) struct Dispositions {
    taken: [KernelAction; SIGNALS_TAKEN_BY_RUBY.len()],
    virtual_timer: KernelAction,
    /// Whether Ruby keeps its handler of SIGINT while it runs.
    interrupt: bool,
}

impl Dispositions {
    /// Records the dispositions of [`SIGNALS_TAKEN_BY_RUBY`] and of SIGVTALRM. With
    /// `interrupt` Ruby keeps its handler of SIGINT until it is finalised.
    pub(crate) fn capture(interrupt: bool) -> Self {
        Self {
            taken: SIGNALS_TAKEN_BY_RUBY.map(get),
            virtual_timer: get(libc::SIGVTALRM),
            interrupt,
        }
    }

    /// Puts the dispositions of [`SIGNALS_TAKEN_BY_RUBY`] back, except that of SIGINT when
    /// Ruby keeps it. SIGVTALRM is left to Ruby.
    pub(crate) fn restore(&self) {
        self.restore_except(self.interrupt);
    }

    /// Puts the dispositions of [`SIGNALS_TAKEN_BY_RUBY`] back, except that of SIGINT with
    /// `keep_interrupt`.
    fn restore_except(&self, keep_interrupt: bool) {
        for (signal, action) in SIGNALS_TAKEN_BY_RUBY.iter().zip(&self.taken) {
            if !(keep_interrupt && *signal == libc::SIGINT) {
                set(*signal, action);
            }
        }
    }

    /// For shutdown: stops the virtual timer that Ruby's threads arm, gives SIGVTALRM its
    /// recorded disposition back, then restores the rest, SIGINT included.
    pub(crate) fn restore_after_finalize(&self) {
        // SAFETY: an all-zero `itimerval` is valid and means "disarm"; a null old value is
        // allowed.
        unsafe {
            let disarm: libc::itimerval = std::mem::zeroed();
            libc::setitimer(ITIMER_VIRTUAL, &disarm, ptr::null_mut());
        }
        if !same(&get(libc::SIGVTALRM), &self.virtual_timer) {
            // A tick may still be pending. Ignoring the signal first discards it, so that it
            // cannot be delivered to a recorded disposition that would end the process.
            set(libc::SIGVTALRM, &ignore());
            set(libc::SIGVTALRM, &self.virtual_timer);
        }
        // Ruby keeps nothing once it is finalised.
        self.restore_except(false);
    }
}
