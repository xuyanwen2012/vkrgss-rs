//! The interpreter creates no thread of its own while the main thread is the only Ruby
//! thread; its timer thread starts with the first thread a script creates.
//!
//! One test per file: the threads of the whole process are counted.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

#[test]
fn no_thread_is_created_before_a_script_creates_one() {
    let before = common::thread_count();
    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    assert_eq!(
        common::thread_count(),
        before + 1,
        "after start: the interpreter thread and no other"
    );

    // A script with Fibers, collections and a sleep, and no Ruby thread.
    let value = interpreter.run(|ruby| {
        eval_inspect(
            ruby,
            "f = Fiber.new { 3.times {|i| Fiber.yield i }; :end }; \
             a = (1..4).map { f.resume }; GC.start; sleep 0.03; \
             [a, Thread.list.size, Thread.current == Thread.main]",
        )
    });
    assert_eq!(value, Ok("[[0, 1, 2, :end], 1, true]".to_owned()));
    assert_eq!(
        common::thread_count(),
        before + 1,
        "after a script without threads"
    );

    // The first script thread: Ruby threads work from then on.
    let value = interpreter.run(|ruby| eval_inspect(ruby, "Thread.new { 1 + 1 }.value"));
    assert_eq!(value, Ok("2".to_owned()));
    let value = interpreter.run(|ruby| {
        eval_inspect(
            ruby,
            "t = Thread.new { sleep 0.05; :done }; t.join; t.value",
        )
    });
    assert_eq!(value, Ok(":done".to_owned()));
    assert!(
        common::thread_count() >= before + 2,
        "the timer thread exists once a script has created a thread"
    );

    // A thread that blocks is woken with SIGVTALRM, whose disposition was the default before
    // the first thread: killing a sleeping thread must not end the process.
    let value = interpreter.run(|ruby| {
        eval_inspect(
            ruby,
            "t = Thread.new { sleep }; sleep 0.05; t.kill; t.join; t.alive?",
        )
    });
    assert_eq!(value, Ok("false".to_owned()));

    // Threads that take turns, which is what the timer thread is for.
    let value = interpreter.run(|ruby| {
        eval_inspect(
            ruby,
            "(1..4).map {|i| Thread.new(i) {|n| 500.times { Thread.pass }; n * 10 } }.map {|t| t.value }",
        )
    });
    assert_eq!(value, Ok("[10, 20, 30, 40]".to_owned()));

    drop(interpreter);
    // A thread that was joined has woken its joiner before the kernel has taken its task out
    // of the list this count reads, so the count may lag for a moment under load.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while common::thread_count() != before && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        common::thread_count(),
        before,
        "after shutdown no thread of the interpreter is left"
    );
}
