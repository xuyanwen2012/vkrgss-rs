//! Start-up state, the work queue, the single-interpreter guard and bounded recursion, on an
//! interpreter with the default options. The start-up values are those measured on the
//! original runtime.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use common::eval_inspect;
use vkxp_ruby192_sys::{Interpreter, Options, StartError};

/// The one interpreter of this test process, with the result of the very first evaluation.
struct Shared {
    interpreter: Interpreter,
    first_evaluation: Result<String, String>,
}

fn shared() -> &'static Shared {
    static SHARED: OnceLock<Shared> = OnceLock::new();
    SHARED.get_or_init(|| {
        let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
        let first_evaluation = interpreter.run(|ruby| eval_inspect(ruby, "[$:, $LOADED_FEATURES]"));
        Shared {
            interpreter,
            first_evaluation,
        }
    })
}

fn eval(source: &'static str) -> Result<String, String> {
    shared()
        .interpreter
        .run(move |ruby| eval_inspect(ruby, source))
}

fn ok(text: &str) -> Result<String, String> {
    Ok(text.to_owned())
}

#[test]
fn load_path_and_features_are_those_of_the_original() {
    assert_eq!(
        shared().first_evaluation,
        ok("[[\".\"], [\"enumerator.so\"]]")
    );
}

#[test]
fn default_encodings_are_those_of_the_original() {
    assert_eq!(
        eval(
            "[Encoding.default_external, Encoding.default_internal, __ENCODING__, 'abc'.encoding]"
        ),
        ok("[#<Encoding:UTF-8>, nil, #<Encoding:UTF-8>, #<Encoding:UTF-8>]")
    );
    // The source is read as UTF-8: a literal of three bytes is one character.
    assert_eq!(
        eval("['あ'.size, 'あ'.bytesize, 'あ'.valid_encoding?]"),
        ok("[1, 3, true]")
    );
}

#[test]
fn zlib_is_defined_without_being_a_feature() {
    assert_eq!(
        eval("Zlib::Inflate.inflate(Zlib::Deflate.deflate('x' * 1000)) == 'x' * 1000"),
        ok("true")
    );
    assert_eq!(eval("Zlib::VERSION"), ok("\"0.6.0\""));
    assert_eq!(
        eval("begin; require 'zlib'; rescue LoadError => e; e.class; end"),
        ok("LoadError")
    );
    assert_eq!(eval("$LOADED_FEATURES"), ok("[\"enumerator.so\"]"));
}

#[test]
fn the_prelude_has_not_run() {
    assert_eq!(
        eval("[Mutex.method_defined?(:synchronize), Thread.respond_to?(:exclusive)]"),
        ok("[false, false]")
    );
}

#[test]
fn fibers_are_those_of_the_core_only() {
    assert_eq!(
        eval("f = Fiber.new { Fiber.yield 1; 2 }; [f.resume, f.resume]"),
        ok("[1, 2]")
    );
    assert_eq!(
        eval(
            "[Fiber.method_defined?(:alive?), Fiber.respond_to?(:current), \
              Fiber.method_defined?(:transfer), respond_to?(:callcc, true)]"
        ),
        ok("[false, false, false, false]")
    );
}

#[test]
fn standard_output_and_error_are_synchronous() {
    assert_eq!(eval("[$stdout.sync, $stderr.sync]"), ok("[true, true]"));
}

#[test]
fn script_name_is_set() {
    assert_eq!(eval("$0"), ok("\"vkxp\""));
}

#[test]
fn work_from_another_thread_runs_in_order_on_the_interpreter_thread() {
    let interpreter = &shared().interpreter;
    let order = Arc::new(Mutex::new(Vec::new()));
    let (first_order, second_order) = (Arc::clone(&order), Arc::clone(&order));

    let caller = thread::current().id();
    let first = interpreter.run(move |ruby| {
        first_order.lock().unwrap().push("first");
        (thread::current().id(), eval_inspect(ruby, "6 * 7"))
    });
    let second = interpreter.run(move |ruby| {
        second_order.lock().unwrap().push("second");
        (thread::current().id(), eval_inspect(ruby, "'a' + 'b'"))
    });

    assert_eq!(*order.lock().unwrap(), ["first", "second"]);
    assert_eq!(first.0, interpreter.thread_id());
    assert_eq!(second.0, interpreter.thread_id());
    assert_ne!(first.0, caller);
    assert_eq!(first.1, ok("42"));
    assert_eq!(second.1, ok("\"ab\""));
}

#[test]
fn work_submitted_by_several_threads_all_runs_on_the_interpreter_thread() {
    let interpreter = &shared().interpreter;
    let workers: Vec<_> = (0..8)
        .map(|index| {
            thread::spawn(move || {
                (0..20)
                    .map(|step| {
                        interpreter.run(move |ruby| {
                            let source = format!("{index} * 100 + {step}");
                            (thread::current().id(), eval_inspect(ruby, &source))
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
    assert_eq!(eval("1 + 1"), ok("2"));
}

#[test]
fn second_interpreter_is_refused_and_the_first_keeps_working() {
    let _ = shared();
    let second = Interpreter::start(Options::default());
    assert!(matches!(second, Err(StartError::AlreadyStarted)));
    assert_eq!(eval("1 + 1"), ok("2"));
}

#[test]
fn unbounded_recursion_raises_and_leaves_the_interpreter_usable() {
    common::assert_recursion_is_bounded(&shared().interpreter);
}
