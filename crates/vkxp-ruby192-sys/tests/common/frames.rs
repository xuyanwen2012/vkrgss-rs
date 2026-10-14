//! A script shaped like a game, and a host function that copies the process with `fork`
//! between two of its frames.
//!
//! The script runs three "event interpreters", each a Fiber that yields once per wait from
//! many calls deep (as `Game_Interpreter` of RPG Maker VX Ace does), and a main loop that
//! resumes them, calls the host once per loop and makes garbage. Everything that goes into
//! its digest is meant to be the same in every run and in every copy of the process.

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};

use vkxp_ruby192_sys::ffi::{self, VALUE};
use vkxp_ruby192_sys::{Arity, HostResult, Interpreter, Options, Ruby};

/// The script, for Fibers that each yield `steps` times; with `stress`, under
/// `GC.stress = true`.
fn script(steps: u32, stress: bool) -> String {
    format!(
        r#"
GC.stress = {stress}
srand(20261013)
log = []
state = {{:gold => 0, "steps" => 0, 3 => []}}

fibers = (1..3).map do |i|
  Fiber.new do
    local = "event#{{i}}"                       # a local on the fiber's machine stack
    depth = lambda {{ |n| n == 0 ? Fiber.yield(local) : depth.call(n - 1) }}
    {steps}.times do |step|
      state[:gold] += rand(100) * i
      state["steps"] += 1
      state[3] << (local + step.to_s).hash % 1000
      depth.call(i * 20)                      # yield from a deep stack
      GC.start if step % 4 == i
    end
    :done
  end
end

frames = 0
alive = fibers.dup
until alive.empty?
  alive.reject! do |f|
    r = f.resume
    log << r if r.is_a?(String) && frames % 5 == 0
    r == :done
  end
  frames += 1
  frame(frames)                               # the host may copy the process here
  junk = (1..200).map {{ |k| "x" * k }}         # garbage, so collections happen
end

sjis = "日本語".encode("Windows-31J")
digest = [frames, state[:gold], state["steps"], state[3].inject(:+), state.keys.inspect,
          log.join(","), sjis.bytes.to_a.inspect, "abc".hash, 1.5.hash, :gold.hash == :gold.hash,
          Marshal.dump(state).bytesize, rand(1000)]
GC.stress = false
digest.join(" ")
"#
    )
}

/// The frames at which the host function copies the process, first and last.
static FIRST_FORK: AtomicI64 = AtomicI64::new(0);
static LAST_FORK: AtomicI64 = AtomicI64::new(0);

/// Set in a copy, in which the host function copies nothing and the frame it was made at.
static IS_COPY: AtomicBool = AtomicBool::new(false);
static COPIED_AT: AtomicI64 = AtomicI64::new(0);

/// Whether a copy calls `restart_thread_timer()` before it goes on.
pub(crate) static RESTART_TIMER: AtomicBool = AtomicBool::new(false);

/// The end of the pipe a copy writes its digest to.
static DIGESTS: AtomicI32 = AtomicI32::new(-1);

/// The copies the original made: the frame and the process.
static COPIES: Mutex<Vec<(i64, libc::pid_t)>> = Mutex::new(Vec::new());

/// `frame(n)`, called by the script once per loop: at the chosen frames it copies the
/// process with `fork`, on the interpreter thread, and does nothing else about the
/// interpreter in the copy.
fn frame(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let number = ffi::fix2long(args[0]);
    let chosen = FIRST_FORK.load(Ordering::SeqCst)..=LAST_FORK.load(Ordering::SeqCst);
    if !IS_COPY.load(Ordering::SeqCst) && chosen.contains(&number) {
        // SAFETY: the interpreter thread holds no lock of this test here, and the copy goes
        // on with this one thread: it only runs the interpreter, writes to a pipe and
        // leaves with `_exit`.
        let process = unsafe { libc::fork() };
        match process {
            0 => {
                IS_COPY.store(true, Ordering::SeqCst);
                if RESTART_TIMER.load(Ordering::SeqCst) {
                    vkxp_ruby192_sys::restart_thread_timer();
                }
                COPIED_AT.store(number, Ordering::SeqCst);
            }
            process if process > 0 => COPIES.lock().unwrap().push((number, process)),
            _ => {
                // SAFETY: the class globals are set while the interpreter runs.
                let class = unsafe { ffi::rb_eRuntimeError };
                return Err(vkxp_ruby192_sys::Raise::new(class, "fork failed"));
            }
        }
    }
    Ok(ffi::Qnil)
}

/// What a run gave: the digest of the original process, and for every copy the frame it
/// was made at, its digest and whether it left with status 0.
pub(crate) struct Outcome {
    pub(crate) original: String,
    pub(crate) copies: Vec<(i64, String, bool)>,
}

/// Runs the script on a new interpreter, copying the process at each of the frames from
/// `first` to `last`. Every copy runs the script to its end, writes its digest to a pipe and
/// leaves; the original collects the digests and the exit statuses.
pub(crate) fn run_with_copies(steps: u32, stress: bool, first: i64, last: i64) -> Outcome {
    let mut ends = [0; 2];
    // SAFETY: `ends` is valid for the two descriptors.
    assert_eq!(unsafe { libc::pipe(ends.as_mut_ptr()) }, 0, "a pipe");
    let [read_end, write_end] = ends;
    DIGESTS.store(write_end, Ordering::SeqCst);
    FIRST_FORK.store(first, Ordering::SeqCst);
    LAST_FORK.store(last, Ordering::SeqCst);

    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let source = script(steps, stress);
    let original = interpreter.run(move |ruby| {
        ruby.define_global_function_with_arity(c"frame", Arity::Fixed(1), frame)
            .expect("the host function is defined");
        let result = ruby.eval_protected(&source, "frames", 1).map(|value| {
            // SAFETY: `value` was just returned and is held in a local.
            let bytes = unsafe { ruby.str_bytes(value) }.unwrap_or_default();
            String::from_utf8_lossy(&bytes).into_owned()
        });
        if IS_COPY.load(Ordering::SeqCst) {
            // A copy of the process has this thread only. It reports and leaves from here,
            // without returning into the test harness, which does not exist in it.
            let (line, status) = match &result {
                Ok(digest) => (
                    format!("{} {digest}\n", COPIED_AT.load(Ordering::SeqCst)),
                    0,
                ),
                Err(error) => (
                    format!("{} RAISED {error}\n", COPIED_AT.load(Ordering::SeqCst)),
                    1,
                ),
            };
            // SAFETY: the descriptor is the write end of the pipe, open in this copy. The
            // `File` is not dropped before `_exit`, which runs no destructor either.
            unsafe {
                let mut pipe = File::from_raw_fd(DIGESTS.load(Ordering::SeqCst));
                let written = pipe.write_all(line.as_bytes()).is_ok();
                libc::_exit(if written { status } else { 2 });
            }
        }
        result
    });
    let original = original.unwrap_or_else(|error| panic!("the script raised: {error}"));

    // The copies hold the write end too; the original closes its own and reads to the end.
    let mut text = String::new();
    // SAFETY: both descriptors are open and owned here; `read_end` goes to the `File`.
    unsafe {
        libc::close(write_end);
        File::from_raw_fd(read_end)
            .read_to_string(&mut text)
            .expect("the digests of the copies");
    }
    let copies = COPIES
        .lock()
        .unwrap()
        .iter()
        .map(|&(frame, process)| {
            let mut status = 0;
            // SAFETY: `process` is a child of this process that nobody waited for.
            let waited = unsafe { libc::waitpid(process, &mut status, 0) };
            let clean =
                waited == process && libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0;
            let prefix = format!("{frame} ");
            let digest = text
                .lines()
                .find_map(|line| line.strip_prefix(&prefix))
                .unwrap_or("(no digest)")
                .to_owned();
            (frame, digest, clean)
        })
        .collect();
    drop(interpreter);
    Outcome { original, copies }
}
