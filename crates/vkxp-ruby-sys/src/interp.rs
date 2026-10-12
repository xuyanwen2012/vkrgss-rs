//! The interpreter thread: start-up, the queue of work, shutdown.

use std::ffi::{CString, c_ulong};
use std::marker::PhantomData;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread::{self, JoinHandle, ThreadId};

use crate::ffi::{self, VALUE};
use crate::protect;
use crate::signals::Dispositions;

/// The smallest stack the interpreter thread is given.
pub const MIN_STACK_SIZE: usize = 16 << 20;

/// How much of the thread's stack is kept free below Ruby's own limit.
///
/// Ruby checks its stack depth only on every 256th method call and was measured to run up to
/// 462 KiB past the limit before raising `SystemStackError`; host frames need room as well.
pub const STACK_HEADROOM: usize = 1 << 20;

/// Set when an interpreter has been started in this process. Never cleared once `ruby_init`
/// has run: Ruby 1.8 cannot be initialised twice, not even after it was finalised.
static STARTED: AtomicBool = AtomicBool::new(false);

/// The environment variable of the collector stress: a number `n` of 1 or more makes the
/// collector run at every n-th allocation of an object or of memory. Unset, empty or 0
/// changes nothing. It is for finding values that the conservative collector loses and
/// makes everything very slow.
pub const GC_STRESS_VARIABLE: &str = "VKXP_GC_STRESS";

/// Reads [`GC_STRESS_VARIABLE`]: the `n` of the collector stress, 0 for none.
fn gc_stress_from_environment() -> Result<c_ulong, StartError> {
    match std::env::var_os(GC_STRESS_VARIABLE) {
        None => Ok(0),
        Some(value) if value.is_empty() => Ok(0),
        Some(value) => value
            .to_str()
            .and_then(|text| text.parse().ok())
            .ok_or_else(|| StartError::InvalidGcStress(value.to_string_lossy().into_owned())),
    }
}

/// How to start the interpreter.
#[derive(Debug, Clone)]
pub struct Options {
    /// The stack size of the interpreter thread in bytes. Values below [`MIN_STACK_SIZE`]
    /// are raised to it.
    pub stack_size: usize,
    /// What Ruby reports as `$0`.
    pub script_name: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            stack_size: MIN_STACK_SIZE,
            script_name: "vkxp".to_owned(),
        }
    }
}

/// Why the interpreter did not start.
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// An interpreter was already started in this process. There can be one per process, and
    /// it cannot be started again after it was dropped.
    #[error("a Ruby interpreter was already started in this process")]
    AlreadyStarted,
    /// The script name contains a NUL byte.
    #[error("the script name contains a NUL byte")]
    InvalidScriptName,
    /// The environment variable `VKXP_GC_STRESS` holds something other than a number.
    #[error("VKXP_GC_STRESS is `{0}`; it must be unset, empty or a number")]
    InvalidGcStress(String),
    /// The operating system did not create the interpreter thread.
    #[error("the interpreter thread could not be created")]
    Spawn(#[source] std::io::Error),
    /// The interpreter thread ended before it reported the end of its start-up.
    #[error("the interpreter thread ended during start-up")]
    ThreadEnded,
    /// A start-up step raised a Ruby exception. No interpreter can be started in this process
    /// any more.
    #[error("interpreter start-up failed while {step}")]
    Init {
        /// What was being done, as a phrase.
        step: &'static str,
    },
}

/// Proof of being on the interpreter thread while the interpreter is alive.
///
/// The closures given to [`Interpreter::run`] and the host functions that Ruby calls receive
/// a `&Ruby`; the operations of this crate that touch the interpreter are its methods. It is
/// neither `Send` nor `Sync`, so it cannot leave the thread.
pub struct Ruby {
    not_send: PhantomData<*mut ()>,
}

impl Ruby {
    /// # Safety
    ///
    /// Only on the interpreter thread, between the end of start-up and finalisation.
    pub(crate) unsafe fn new() -> Self {
        Self {
            not_send: PhantomData,
        }
    }

    /// Tells the interpreter that the game's scripts begin here.
    ///
    /// Symbol numbers follow the original runtime: a name it has before its first game
    /// script has the number measured there, whenever it is interned. Any other name
    /// interned before this call gets a number from a separate high range; from this call
    /// on, a new name gets the next number of the original's sequence, and the first use of
    /// a name of the high range consumes one number of that sequence. Calling it again
    /// changes nothing.
    pub fn mark_game_scripts(&self) {
        // SAFETY: on the interpreter thread; the function sets a flag and cannot raise.
        unsafe { ffi::ruby_vkxp_mark_game_scripts() };
    }
}

type Job = Box<dyn FnOnce(&Ruby) + Send>;

/// The handle of the embedded Ruby interpreter and of the thread it lives on.
///
/// Every call into Ruby happens on that thread; [`run`](Self::run) is the way onto it.
/// Dropping the handle lets queued work finish, finalises Ruby, stops the timer its threads
/// use, puts the process's signal dispositions back once more and joins the thread.
pub struct Interpreter {
    jobs: Option<Sender<Job>>,
    thread: Option<JoinHandle<()>>,
    thread_id: ThreadId,
}

impl Interpreter {
    /// Starts the interpreter on a new thread and waits until it is ready.
    ///
    /// On that thread, in this order: the dispositions of
    /// [`SIGNALS_TAKEN_BY_RUBY`](crate::SIGNALS_TAKEN_BY_RUBY) are recorded, the stack is
    /// anchored for the collector, `ruby_init` runs, the stack limit is set to the stack size
    /// minus [`STACK_HEADROOM`], `Zlib` is registered as if `zlib` had been required, `$0` is
    /// set, `$stdout` and `$stderr` are made synchronous, and the recorded dispositions are
    /// put back. The load path stays empty.
    ///
    /// The environment variable [`GC_STRESS_VARIABLE`] is read once, here, and takes effect
    /// right after `ruby_init`.
    pub fn start(options: Options) -> Result<Self, StartError> {
        let script_name =
            CString::new(options.script_name).map_err(|_| StartError::InvalidScriptName)?;
        let gc_stress = gc_stress_from_environment()?;
        let stack_size = options.stack_size.max(MIN_STACK_SIZE);
        if STARTED.swap(true, Ordering::SeqCst) {
            return Err(StartError::AlreadyStarted);
        }

        let (jobs, queue) = mpsc::channel::<Job>();
        let (ready, started) = mpsc::sync_channel::<Result<(), StartError>>(1);
        let thread = thread::Builder::new()
            .name("ruby".to_owned())
            .stack_size(stack_size)
            .spawn(move || thread_main(stack_size, gc_stress, &script_name, queue, ready));
        let thread = match thread {
            Ok(thread) => thread,
            Err(error) => {
                // Nothing of Ruby was initialised, so another attempt is allowed.
                STARTED.store(false, Ordering::SeqCst);
                return Err(StartError::Spawn(error));
            }
        };
        match started.recv() {
            Ok(Ok(())) => Ok(Self {
                jobs: Some(jobs),
                thread_id: thread.thread().id(),
                thread: Some(thread),
            }),
            Ok(Err(error)) => {
                let _ = thread.join();
                Err(error)
            }
            Err(_) => {
                let _ = thread.join();
                Err(StartError::ThreadEnded)
            }
        }
    }

    /// Runs a closure on the interpreter thread and returns its result.
    ///
    /// Closures run one at a time, in the order they were submitted. A panic in the closure is
    /// carried back and resumed on the calling thread; the interpreter thread keeps serving.
    ///
    /// # Panics
    ///
    /// Panics when called on the interpreter thread itself, which would wait for itself, and
    /// when the interpreter thread has ended.
    pub fn run<F, R>(&self, work: F) -> R
    where
        F: FnOnce(&Ruby) -> R + Send + 'static,
        R: Send + 'static,
    {
        assert!(
            thread::current().id() != self.thread_id,
            "Interpreter::run called on the interpreter thread; use the `&Ruby` at hand"
        );
        let (reply, result) = mpsc::sync_channel(1);
        let job: Job = Box::new(move |ruby| {
            let outcome = panic::catch_unwind(AssertUnwindSafe(|| work(ruby)));
            let _ = reply.send(outcome);
        });
        self.jobs
            .as_ref()
            .expect("the queue exists until drop")
            .send(job)
            .expect("the interpreter thread has ended");
        match result.recv() {
            Ok(Ok(value)) => value,
            Ok(Err(payload)) => panic::resume_unwind(payload),
            Err(_) => panic!("the interpreter thread ended before finishing the work"),
        }
    }

    /// The identity of the interpreter thread.
    pub fn thread_id(&self) -> ThreadId {
        self.thread_id
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        // Closing the queue ends the serving loop after the work already submitted.
        drop(self.jobs.take());
        if let Some(thread) = self.thread.take()
            && thread::current().id() != self.thread_id
        {
            let _ = thread.join();
        }
    }
}

/// The body of the interpreter thread. Its frame holds the stack anchor, so it must enclose
/// every frame that ever holds a Ruby value: all of them are below [`serve`].
fn thread_main(
    stack_size: usize,
    gc_stress: c_ulong,
    script_name: &CString,
    queue: Receiver<Job>,
    ready: SyncSender<Result<(), StartError>>,
) {
    let saved = Dispositions::capture();
    let mut anchor: VALUE = 0;
    // SAFETY: `anchor` lives until this function returns, which is after `serve` has
    // finalised the interpreter, and no interpreter exists yet in this process (`STARTED`).
    unsafe { ffi::ruby_init_stack(&mut anchor) };
    serve(stack_size, gc_stress, script_name, &saved, queue, ready);
    std::hint::black_box(&mut anchor);
}

/// Start-up steps that can raise, as one protected unit of plain C-API calls.
struct Setup {
    script_name: *const std::ffi::c_char,
    step: &'static str,
}

unsafe extern "C" fn setup_body(data: VALUE) -> VALUE {
    // SAFETY: `serve` passes a pointer to a `Setup` that outlives the call, on the
    // interpreter thread right after `ruby_init`. Everything called is the C API on live
    // values; `step` is a plain reference, so a raise jumps over nothing that needs dropping.
    unsafe {
        let setup = &mut *(data as *mut Setup);
        setup.step = "registering Zlib";
        ffi::ruby_init_ext(c"zlib.so".as_ptr(), ffi::Init_zlib);
        setup.step = "naming the script";
        ffi::ruby_script(setup.script_name);
        setup.step = "making the standard output and error synchronous";
        let sync = ffi::rb_intern(c"sync=".as_ptr());
        ffi::rb_funcall(ffi::rb_stdout, sync, 1, ffi::Qtrue);
        ffi::rb_funcall(ffi::rb_stderr, sync, 1, ffi::Qtrue);
    }
    ffi::Qnil
}

#[inline(never)]
fn serve(
    stack_size: usize,
    gc_stress: c_ulong,
    script_name: &CString,
    saved: &Dispositions,
    queue: Receiver<Job>,
    ready: SyncSender<Result<(), StartError>>,
) {
    let mut setup = Setup {
        script_name: script_name.as_ptr(),
        step: "initialising",
    };
    // SAFETY: this is the one interpreter thread of the process, its stack is anchored in
    // the caller's frame, and nothing of Ruby ran before. `ruby_set_stack_size` only stores
    // the limit, and `ruby_vkxp_gc_stress` the number. `setup_body` fits the contract of
    // `protect`.
    let started = unsafe {
        ffi::ruby_init();
        ffi::ruby_set_stack_size(stack_size - STACK_HEADROOM);
        ffi::ruby_vkxp_gc_stress(gc_stress);
        protect::protect(&mut setup, setup_body)
    };
    // Ruby took the signals during `ruby_init`; the process gets them back.
    saved.restore();

    match started {
        Ok(_) => {
            let _ = ready.send(Ok(()));
            // SAFETY: on the interpreter thread, after start-up and before finalisation.
            let ruby = unsafe { Ruby::new() };
            for job in queue {
                job(&ruby);
            }
        }
        Err(_) => {
            let _ = ready.send(Err(StartError::Init { step: setup.step }));
        }
    }

    // SAFETY: still on the interpreter thread; nothing uses Ruby after this.
    let _ = unsafe { protect::call_void(ffi::ruby_finalize) };
    saved.restore_after_finalize();
}
