//! Handlers that the host installed before starting the interpreter survive it, with their
//! flags and masks.
//!
//! One test per file: the comparison needs a process in which nothing else starts Ruby.

mod common;

use std::sync::atomic::{AtomicI32, Ordering};

use vkxp_ruby_sys::{Interpreter, Options};

static DELIVERED: AtomicI32 = AtomicI32::new(0);

extern "C" fn host_handler(signal: libc::c_int) {
    DELIVERED.store(signal, Ordering::SeqCst);
}

/// Installs `host_handler` for a signal with the given flags and one signal in its mask.
fn install(signal: libc::c_int, flags: libc::c_int, masked: libc::c_int) {
    // SAFETY: the action is fully initialised before use, and the handler only stores to an
    // atomic, which is async-signal-safe.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = host_handler as extern "C" fn(libc::c_int) as usize;
        action.sa_flags = flags;
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaddset(&mut action.sa_mask, masked);
        assert_eq!(libc::sigaction(signal, &action, std::ptr::null_mut()), 0);
    }
}

#[test]
fn host_handlers_survive_start_and_shutdown() {
    install(libc::SIGTERM, libc::SA_RESTART, libc::SIGUSR2);
    install(
        libc::SIGUSR1,
        libc::SA_RESTART | libc::SA_NODEFER,
        libc::SIGHUP,
    );
    // SAFETY: ignoring a signal is always allowed.
    unsafe { libc::signal(libc::SIGINT, libc::SIG_IGN) };
    let before = common::dispositions();

    let term = before
        .iter()
        .find(|entry| entry.0 == libc::SIGTERM)
        .unwrap();
    assert_eq!(term.1, host_handler as extern "C" fn(libc::c_int) as usize);
    assert_ne!(term.2 & libc::SA_RESTART, 0);
    assert_eq!(term.3, [libc::SIGUSR2]);

    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    assert_eq!(common::dispositions(), before, "after start");
    let after = common::dispositions();
    let term = after.iter().find(|entry| entry.0 == libc::SIGTERM).unwrap();
    assert_eq!(term.1, host_handler as extern "C" fn(libc::c_int) as usize);
    assert_ne!(term.2 & libc::SA_RESTART, 0, "the restart flag is kept");

    // The terminate signal reaches the host's handler, not Ruby's, and Ruby does not see it.
    // SAFETY: raising a signal that has a handler installed.
    unsafe { libc::raise(libc::SIGTERM) };
    assert_eq!(DELIVERED.load(Ordering::SeqCst), libc::SIGTERM);
    let value = interpreter.run(|_| common::eval_inspect("100.times { 1.abs }; :undisturbed"));
    assert_eq!(value, Ok(":undisturbed".to_owned()));

    let value = interpreter.run(|_| common::eval_inspect("Thread.new { 21 * 2 }.value"));
    assert_eq!(value, Ok("42".to_owned()));
    drop(interpreter);
    assert_eq!(common::dispositions(), before, "after shutdown");
}
