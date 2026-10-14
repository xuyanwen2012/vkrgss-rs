//! A script in two parts with a copy of the process between them: the first part runs in
//! the original, a host function then copies the process with `fork` on the interpreter
//! thread and calls `restart_thread_timer()` in the copy, and both run the second part.

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::{Duration, Instant};

use vkxp_ruby192_sys::ffi::{self, VALUE};
use vkxp_ruby192_sys::{Arity, HostResult, Interpreter, Options, Ruby};

use super::eval_inspect;

/// Set in the copy.
static IS_COPY: AtomicBool = AtomicBool::new(false);

/// The copy the original made, or 0.
static COPY: AtomicI32 = AtomicI32::new(0);

/// `copy`, called by the script between its two parts.
fn copy(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    // SAFETY: the interpreter thread holds no lock of this test here, and the copy goes on
    // with this one thread: it only runs the interpreter, writes to a pipe and leaves with
    // `_exit`.
    let process = unsafe { libc::fork() };
    match process {
        0 => {
            IS_COPY.store(true, Ordering::SeqCst);
            vkxp_ruby192_sys::restart_thread_timer();
        }
        process if process > 0 => COPY.store(process, Ordering::SeqCst),
        _ => {
            // SAFETY: the class globals are set while the interpreter runs.
            let class = unsafe { ffi::rb_eRuntimeError };
            return Err(vkxp_ruby192_sys::Raise::new(class, "fork failed"));
        }
    }
    Ok(ffi::Qnil)
}

/// What the second part gave in the original and in the copy, as `inspect` of its value,
/// and whether the copy left with status 0. A copy that has not left when `limit` has
/// passed is ended, and its value is then `(no value)`.
pub(crate) struct Outcome {
    pub(crate) original: Result<String, String>,
    pub(crate) copy: String,
    pub(crate) clean: bool,
}

/// Runs `before`, copies the process, and runs `after` in both.
pub(crate) fn run_around_a_copy(before: &str, after: &str, limit: Duration) -> Outcome {
    let mut ends = [0; 2];
    // SAFETY: `ends` is valid for the two descriptors.
    assert_eq!(unsafe { libc::pipe(ends.as_mut_ptr()) }, 0, "a pipe");
    let [read_end, write_end] = ends;

    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let source = format!("{before}\ncopy\n{after}");
    let original = interpreter.run(move |ruby| {
        ruby.define_global_function_with_arity(c"copy", Arity::Fixed(0), copy)
            .expect("the host function is defined");
        let result = eval_inspect(ruby, &source);
        if IS_COPY.load(Ordering::SeqCst) {
            // A copy of the process has this thread only. It reports and leaves from here,
            // without returning into the test harness, which does not exist in it.
            let (line, status) = match &result {
                Ok(value) => (value.clone(), 0),
                Err(error) => (format!("RAISED {error}"), 1),
            };
            // SAFETY: the descriptor is the write end of the pipe, open in this copy. The
            // `File` is not dropped before `_exit`, which runs no destructor either.
            unsafe {
                let mut pipe = File::from_raw_fd(write_end);
                let written = pipe.write_all(line.as_bytes()).is_ok();
                libc::_exit(if written { status } else { 2 });
            }
        }
        result
    });

    let process = COPY.load(Ordering::SeqCst);
    assert!(process > 0, "the script made no copy: {original:?}");
    let deadline = Instant::now() + limit;
    let mut status = 0;
    let clean = loop {
        // SAFETY: `process` is a child of this process that nobody waited for.
        let waited = unsafe { libc::waitpid(process, &mut status, libc::WNOHANG) };
        if waited == process {
            break libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0;
        }
        if Instant::now() >= deadline {
            // SAFETY: `process` is the child this test made and has not been waited for.
            unsafe {
                libc::kill(process, libc::SIGKILL);
                libc::waitpid(process, &mut status, 0);
            }
            break false;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let mut text = String::new();
    // SAFETY: both descriptors are open and owned here; `read_end` goes to the `File`. The
    // copy has left, so the read ends.
    unsafe {
        libc::close(write_end);
        File::from_raw_fd(read_end)
            .read_to_string(&mut text)
            .expect("the value of the copy");
    }
    drop(interpreter);
    Outcome {
        original,
        copy: if text.is_empty() {
            "(no value)".to_owned()
        } else {
            text
        },
        clean,
    }
}
