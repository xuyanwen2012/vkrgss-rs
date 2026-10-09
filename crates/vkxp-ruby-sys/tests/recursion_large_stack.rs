//! Bounded recursion on an interpreter thread with a 32 MiB stack.

mod common;

use vkxp_ruby_sys::{Interpreter, Options};

#[test]
fn unbounded_recursion_raises_on_a_32_mib_stack() {
    let interpreter = Interpreter::start(Options {
        stack_size: 32 << 20,
        ..Options::default()
    })
    .expect("the interpreter starts");
    common::assert_recursion_is_bounded(&interpreter);

    // The larger stack really is in use: a depth that the default stack cannot hold.
    let deep = interpreter.run(|_| {
        common::eval_inspect(
            "def vkxp_depth(n); n == 0 ? :bottom : vkxp_depth(n - 1); end; vkxp_depth(6000)",
        )
    });
    assert_eq!(deep, Ok(":bottom".to_owned()));
}
