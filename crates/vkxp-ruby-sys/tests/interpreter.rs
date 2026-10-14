//! Start-up state, the work queue, the single-interpreter guard and bounded recursion, on an
//! interpreter with the default options.

mod common;

use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use common::eval_inspect;
use vkxp_ruby_sys::{Interpreter, Options, StartError};

/// The one interpreter of this test process, with the result of the very first evaluation.
struct Shared {
    interpreter: Interpreter,
    first_evaluation: Result<String, String>,
}

fn shared() -> &'static Shared {
    static SHARED: OnceLock<Shared> = OnceLock::new();
    SHARED.get_or_init(|| {
        let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
        let first_evaluation = interpreter.run(|_| {
            eval_inspect("Zlib::Inflate.inflate(Zlib::Deflate.deflate('x' * 1000)) == 'x' * 1000")
        });
        Shared {
            interpreter,
            first_evaluation,
        }
    })
}

fn eval(source: &'static str) -> Result<String, String> {
    shared().interpreter.run(move |_| eval_inspect(source))
}

#[test]
fn zlib_works_as_the_first_thing_evaluated() {
    assert_eq!(shared().first_evaluation, Ok("true".to_owned()));
}

#[test]
fn requiring_zlib_later_returns_false() {
    assert_eq!(eval("require 'zlib'"), Ok("false".to_owned()));
    assert_eq!(eval("$\".include?('zlib.so')"), Ok("true".to_owned()));
}

#[test]
fn load_path_is_empty() {
    assert_eq!(eval("$:"), Ok("[]".to_owned()));
}

#[test]
fn standard_output_and_error_are_synchronous() {
    assert_eq!(
        eval("[$stdout.sync, $stderr.sync]"),
        Ok("[true, true]".to_owned())
    );
}

#[test]
fn script_name_is_set() {
    assert_eq!(eval("$0"), Ok("\"vkxp\"".to_owned()));
}

#[test]
fn work_from_another_thread_runs_in_order_on_the_interpreter_thread() {
    let interpreter = &shared().interpreter;
    let order = Arc::new(Mutex::new(Vec::new()));
    let (first_order, second_order) = (Arc::clone(&order), Arc::clone(&order));

    let caller = thread::current().id();
    let first = interpreter.run(move |_| {
        first_order.lock().unwrap().push("first");
        (thread::current().id(), eval_inspect("6 * 7"))
    });
    let second = interpreter.run(move |_| {
        second_order.lock().unwrap().push("second");
        (thread::current().id(), eval_inspect("'a' + 'b'"))
    });

    assert_eq!(*order.lock().unwrap(), ["first", "second"]);
    assert_eq!(first.0, interpreter.thread_id());
    assert_eq!(second.0, interpreter.thread_id());
    assert_ne!(first.0, caller);
    assert_eq!(first.1, Ok("42".to_owned()));
    assert_eq!(second.1, Ok("\"ab\"".to_owned()));
}

#[test]
fn work_submitted_by_several_threads_all_runs_on_the_interpreter_thread() {
    let interpreter = &shared().interpreter;
    let workers: Vec<_> = (0..8)
        .map(|index| {
            thread::spawn(move || {
                (0..20)
                    .map(|step| {
                        interpreter.run(move |_| {
                            let source = format!("{index} * 100 + {step}");
                            (thread::current().id(), eval_inspect(&source))
                        })
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    for (index, worker) in workers.into_iter().enumerate() {
        for (step, (thread_id, value)) in worker.join().unwrap().into_iter().enumerate() {
            assert_eq!(thread_id, interpreter.thread_id());
            assert_eq!(value, Ok((index * 100 + step).to_string()));
        }
    }
}

#[test]
fn a_panic_in_submitted_work_reaches_the_caller_and_the_thread_keeps_serving() {
    let interpreter = &shared().interpreter;
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        interpreter.run(|_| panic!("work failed"))
    }));
    let payload = caught.expect_err("the panic is carried back");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"work failed"));
    assert_eq!(eval("1 + 1"), Ok("2".to_owned()));
}

#[test]
fn second_interpreter_is_refused_and_the_first_keeps_working() {
    let _ = shared();
    let second = Interpreter::start(Options::default());
    assert!(matches!(second, Err(StartError::AlreadyStarted)));
    assert_eq!(eval("1 + 1"), Ok("2".to_owned()));
}

#[test]
fn unbounded_recursion_raises_and_leaves_the_interpreter_usable() {
    common::assert_recursion_is_bounded(&shared().interpreter);
}

#[test]
fn ruby_threads_and_continuations_work_on_the_interpreter_thread() {
    assert_eq!(
        eval(
            "(1..4).map {|i| Thread.new(i) {|n| 500.times { Thread.pass }; n * 10 } }.map {|t| t.value }"
        ),
        Ok("[10, 20, 30, 40]".to_owned())
    );
    assert_eq!(
        eval(
            "k = nil; c = 0; r = callcc {|cc| k = cc; 0 }; c += 1; k.call(r + 1) if r < 3; [r, c]"
        ),
        Ok("[3, 4]".to_owned())
    );
}

#[test]
fn a_copy_of_the_process_loses_no_script_thread() {
    // The threads of Ruby 1.8 are memory of the interpreter: the answer is `false` even
    // while a script thread is alive.
    let answers = shared().interpreter.run(|_| {
        let made = eval_inspect("$sleeper = Thread.new { sleep }; $sleeper.alive?");
        let lost = vkxp_ruby_sys::threads_lost_in_a_copy();
        let ended = eval_inspect("$sleeper.kill; $sleeper.join; $sleeper.alive?");
        (made, lost, ended)
    });
    assert_eq!(
        answers,
        (Ok("true".to_owned()), false, Ok("false".to_owned()))
    );
}
