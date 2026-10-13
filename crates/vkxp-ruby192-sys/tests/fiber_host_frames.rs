//! Frames of the host above and below the interpreter, across Fiber switches, under
//! `GC.stress = true`.
//!
//! A host function keeps Ruby values in locals only (no root), forces a collection, yields
//! to a Ruby block that suspends the Fiber it runs on, and uses the values after the Fiber
//! is resumed. A Fiber switch of Ruby 1.9.2 copies the machine stack to the heap and back.
//! If the conservative collector missed a value held by a frame of the host, on the
//! thread's stack or on the saved stack of a suspended Fiber, the result is wrong or the
//! process crashes.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use vkxp_ruby192_sys::ffi::{self, VALUE};
use vkxp_ruby192_sys::{Arity, HostResult, Interpreter, Options, Ruby};

/// `host_join(n) { ... }`: makes the strings `<0>` to `<n-1>` and two more, collects, yields,
/// collects, and joins them. With `n` up to 9 it holds eleven new strings in its frame.
fn host_join(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let count = ffi::fix2long(args[0]) as usize;
    let mut parts = [ffi::Qnil; 16];
    for (index, slot) in parts.iter_mut().enumerate().take(count) {
        *slot = ruby.str_new_utf8(&format!("<{index}>"))?;
    }
    // Two more that live in registers or spill slots, as the optimiser likes.
    let head = ruby.str_new_utf8("[")?;
    let tail = ruby.str_new_utf8("]")?;
    // SAFETY: on the interpreter thread, called by Ruby with a block. No value with a
    // destructor is alive in this frame, so a raise out of the block may jump over it. The
    // strings are live: they are in locals of this frame, which is what is tested.
    unsafe {
        ffi::rb_gc_start();
        // The block suspends this Fiber; other Fibers run and collect meanwhile.
        ffi::rb_yield(ffi::Qnil);
        ffi::rb_gc_start();
        let out = ffi::rb_str_buf_new(0);
        ffi::rb_str_append(out, head);
        for part in parts.iter().take(count) {
            ffi::rb_str_append(out, *part);
        }
        ffi::rb_str_append(out, tail);
        Ok(out)
    }
}

/// Four Fibers with six calls each. The block of every call suspends its Fiber.
const SCRIPT: &str = "
GC.stress = true
fibers = (1..4).map { |k|
  Fiber.new {
    acc = []
    6.times { |i| acc << host_join(k + i) { Fiber.yield(:waiting); Array.new(50) { |j| 'g' * j } } }
    acc
  }
}
results = {}
alive = fibers.each_with_index.to_a
until alive.empty?
  alive.reject! { |f, i| r = f.resume; r == :waiting ? false : (results[i] = r; true) }
  Array.new(100) { |j| 'x' * j }
end
GC.stress = false
want = (1..4).map { |k| (0...6).map { |i| '[' + (0...(k + i)).map { |m| '<' + m.to_s + '>' }.join + ']' } }
got = (0...4).map { |i| results[i] }
(got == want ? 'OK ' : 'MISMATCH ') + got.flatten.size.to_s + ' strings, last ' + got.last.last
";

#[test]
fn values_in_host_frames_survive_fiber_switches_under_collector_stress() {
    let interpreter = Interpreter::start(Options::default()).expect("the interpreter starts");
    let result = interpreter.run(|ruby| {
        ruby.define_global_function_with_arity(c"host_join", Arity::Fixed(1), host_join)
            .expect("the host function is defined");
        common::eval_inspect(ruby, SCRIPT)
    });
    assert_eq!(
        result,
        Ok("\"OK 24 strings, last [<0><1><2><3><4><5><6><7><8>]\"".to_owned())
    );
}
