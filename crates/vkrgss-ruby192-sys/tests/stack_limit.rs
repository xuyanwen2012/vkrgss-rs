//! The interpreter's limit of the machine stack follows the stack of its thread, not the
//! resource limit of the process: with a limit of the process far above the thread's stack,
//! unbounded recursion still raises `SystemStackError` instead of overrunning the stack.
//!
//! One test per file: the test changes a resource limit of the whole process.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use common::eval_inspect;
use vkrgss_ruby192_sys::ffi::VALUE;
use vkrgss_ruby192_sys::{HostResult, Interpreter, MIN_STACK_SIZE, Options, Ruby};

/// A host function whose frame holds 16 KiB and that calls back into the script. A recursion
/// through it is deep in the machine stack and shallow in Ruby's own stack: without a limit
/// of the machine stack that fits the thread, it runs off the thread's stack long before
/// Ruby's own stack is full.
fn deep(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let mut frame = [0_u8; 16 << 10];
    std::hint::black_box(&mut frame);
    // SAFETY: the receiver is live for the duration of the call.
    let value = unsafe { ruby.funcall(recv, c"vkrgss_rec_host", &[]) }?;
    std::hint::black_box(&mut frame);
    Ok(value)
}

const MACHINE_STACK_RECURSION: &str = "def vkrgss_rec_host; vkrgss_deep; end; vkrgss_rec_host";

#[test]
fn recursion_is_bounded_when_the_process_limit_is_above_the_thread_stack() {
    // The soft limit goes as far above the thread's stack as the hard limit allows.
    let wanted: libc::rlim_t = 16 * MIN_STACK_SIZE as libc::rlim_t;
    // SAFETY: `limit` is a valid buffer, and the new limit is at most the hard one.
    let raised = unsafe {
        let mut limit: libc::rlimit = std::mem::zeroed();
        assert_eq!(libc::getrlimit(libc::RLIMIT_STACK, &mut limit), 0);
        limit.rlim_cur = if limit.rlim_max == libc::RLIM_INFINITY {
            wanted
        } else {
            wanted.min(limit.rlim_max)
        };
        assert_eq!(libc::setrlimit(libc::RLIMIT_STACK, &limit), 0);
        limit.rlim_cur
    };
    if raised <= MIN_STACK_SIZE as libc::rlim_t {
        eprintln!(
            "the hard limit of the stack size ({raised} bytes) is not above the thread's \
             stack; the limit could not be raised and the test shows nothing new"
        );
    }

    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    common::assert_recursion_is_bounded(&interpreter);
    interpreter.run(|ruby| {
        ruby.define_global_function(c"vkrgss_deep", deep)
            .expect("the host function is defined")
    });
    for round in 0..3 {
        let outcome = interpreter.run(|ruby| eval_inspect(ruby, MACHINE_STACK_RECURSION));
        let error = outcome.expect_err("unbounded recursion cannot return");
        assert!(
            error.starts_with("SystemStackError: "),
            "round {round} raised {error}"
        );
        let after = interpreter.run(|ruby| eval_inspect(ruby, "1 + 1"));
        assert_eq!(after, Ok("2".to_owned()), "after round {round}");
    }
}
