//! Starting and dropping the interpreter leaves the dispositions the process started with,
//! and the signal mask the interpreter thread started with.
//!
//! One test per file: the comparison needs a process in which nothing else starts Ruby.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use vkxp_ruby192_sys::{Interpreter, Options};

/// The disposition of every signal from 1 to 31 that can have one: all but SIGKILL and
/// SIGSTOP.
fn all_dispositions() -> Vec<(libc::c_int, usize, libc::c_int, Vec<libc::c_int>)> {
    (1..=31)
        .filter(|&signal| signal != libc::SIGKILL && signal != libc::SIGSTOP)
        .map(|signal| {
            let (handler, flags, mask) = common::disposition(signal);
            (signal, handler, flags, mask)
        })
        .collect()
}

#[test]
fn dispositions_and_mask_survive_start_and_shutdown() {
    // The process gets the default disposition of every signal Ruby takes, whatever it
    // inherited: a shell ignores SIGINT and SIGQUIT in a background command, and `nohup`
    // ignores SIGHUP. SIGSEGV, SIGBUS and SIGPIPE stay as the Rust runtime set them up before
    // `main` (handlers for its stack guard, and "ignore").
    for signal in [
        libc::SIGHUP,
        libc::SIGINT,
        libc::SIGQUIT,
        libc::SIGUSR1,
        libc::SIGUSR2,
        libc::SIGALRM,
        libc::SIGTERM,
        libc::SIGCHLD,
        libc::SIGVTALRM,
    ] {
        // SAFETY: the default disposition installs no handler, and this process has one
        // test, so nothing else depends on these dispositions.
        let previous = unsafe { libc::signal(signal, libc::SIG_DFL) };
        assert_ne!(previous, libc::SIG_ERR, "signal {signal}");
        assert_eq!(common::disposition(signal).0, libc::SIG_DFL, "{signal}");
    }
    let before = all_dispositions();
    // The interpreter thread starts with the mask of the thread that creates it.
    let mask_before = common::blocked_signals();

    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    assert_eq!(all_dispositions(), before, "after start");
    let mask_inside = interpreter.run(|_| common::blocked_signals());
    assert_eq!(
        mask_inside, mask_before,
        "the interpreter thread blocks what it blocked before ruby_init"
    );

    let value = interpreter.run(|ruby| common::eval_inspect(ruby, "100.times { 1.abs }; :ran"));
    assert_eq!(value, Ok(":ran".to_owned()));
    assert_eq!(all_dispositions(), before, "after running a script");

    drop(interpreter);
    assert_eq!(all_dispositions(), before, "after shutdown");
}
