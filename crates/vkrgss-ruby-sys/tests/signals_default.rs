//! Starting and dropping the interpreter leaves the dispositions the process started with.
//!
//! One test per file: the comparison needs a process in which nothing else starts Ruby.

mod common;

use vkrgss_ruby_sys::{Interpreter, Options};

#[cfg(not(target_os = "android"))]
use libc::ITIMER_VIRTUAL;

/// The virtual interval timer of the kernel, which the `libc` crate does not define for
/// Android.
#[cfg(target_os = "android")]
const ITIMER_VIRTUAL: libc::c_int = 1;

#[test]
fn dispositions_of_the_process_survive_start_and_shutdown() {
    let before = common::dispositions();

    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    assert_eq!(common::dispositions(), before, "after start");

    // A second Ruby thread makes Ruby arm its process-wide virtual timer.
    let value = interpreter.run(|_| common::eval_inspect("Thread.new { 21 * 2 }.value"));
    assert_eq!(value, Ok("42".to_owned()));
    assert_eq!(common::dispositions(), before, "after running Ruby threads");

    drop(interpreter);
    assert_eq!(common::dispositions(), before, "after shutdown");

    // SAFETY: plain queries of the timer and of one disposition into valid buffers.
    unsafe {
        let mut timer: libc::itimerval = std::mem::zeroed();
        assert_eq!(libc::getitimer(ITIMER_VIRTUAL, &mut timer), 0);
        assert_eq!(
            (timer.it_interval.tv_sec, timer.it_interval.tv_usec),
            (0, 0),
            "the virtual timer is disarmed after shutdown"
        );
        let mut action: libc::sigaction = std::mem::zeroed();
        assert_eq!(
            libc::sigaction(libc::SIGVTALRM, std::ptr::null(), &mut action),
            0
        );
        assert_eq!(
            action.sa_sigaction,
            libc::SIG_DFL,
            "SIGVTALRM is back to its default"
        );
    }
}
