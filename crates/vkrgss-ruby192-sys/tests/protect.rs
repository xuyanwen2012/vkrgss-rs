//! The exported protected call, used the way another crate uses it.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

use std::cell::Cell;
use std::ffi::c_long;
use std::sync::OnceLock;

use vkrgss_ruby192_sys::ffi::{self, VALUE};
use vkrgss_ruby192_sys::{Interpreter, Options, Raised, Ruby, protect};

fn interpreter() -> &'static Interpreter {
    static SHARED: OnceLock<Interpreter> = OnceLock::new();
    SHARED.get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
}

/// The plain data of the test operation.
struct Sum {
    left: c_long,
    right: c_long,
    /// Set by the body just before it returns.
    finished: bool,
}

unsafe extern "C" fn sum_body(data: VALUE) -> VALUE {
    // SAFETY: the tests pass a pointer to a `Sum` that outlives this call.
    unsafe {
        let sum = &mut *(data as *mut Sum);
        sum.finished = true;
        ffi::long2fix(sum.left + sum.right)
    }
}

unsafe extern "C" fn raise_body(data: VALUE) -> VALUE {
    // SAFETY: the tests pass a pointer to a `Sum` that outlives this call; the message is
    // NUL-terminated and nothing here has a destructor.
    unsafe {
        let sum = &mut *(data as *mut Sum);
        if sum.left == 0 {
            ffi::rb_exc_raise(ffi::rb_exc_new2(ffi::rb_eArgError, c"boom".as_ptr()));
        }
        sum.finished = true;
        ffi::Qnil
    }
}

/// Counts its drops, to show that a frame was left in the regular way exactly once.
struct Guard<'a>(&'a Cell<u32>);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn body_that_returns_gives_its_value() {
    let (value, finished) = interpreter().run(|_ruby| {
        let mut sum = Sum {
            left: 40,
            right: 2,
            finished: false,
        };
        // SAFETY: on the interpreter thread; `sum_body` fits the contract of `protect`.
        let value = unsafe { protect(&mut sum, sum_body) }.expect("the body does not raise");
        assert!(ffi::fixnum_p(value));
        (ffi::fix2long(value), sum.finished)
    });
    assert_eq!(value, 42);
    assert!(finished);
}

#[test]
fn body_that_raises_gives_the_exception_as_a_value() {
    interpreter().run(|ruby| {
        let drops = Cell::new(0);
        {
            let _guard = Guard(&drops);
            let mut sum = Sum {
                left: 0,
                right: 0,
                finished: false,
            };
            // SAFETY: on the interpreter thread; `raise_body` fits the contract of `protect`.
            let raised: Raised =
                unsafe { protect(&mut sum, raise_body) }.expect_err("the body raises");
            // The frame that called `protect` goes on after the raise, its locals intact.
            assert_eq!(drops.get(), 0);
            assert!(!sum.finished);
            assert_eq!(raised.state, ffi::TAG_RAISE);
            // SAFETY: `raised` is in a local on the interpreter thread.
            let error = unsafe { ruby.describe(raised) };
            assert_eq!(error.class_name, "ArgumentError");
            assert_eq!(error.message, "boom");
            // SAFETY: on the interpreter thread; reading the current error raises nothing.
            assert_eq!(unsafe { ffi::rb_errinfo() }, ffi::Qnil);
            let errinfo = ruby.eval_protected("$!.inspect", "test", 1);
            // SAFETY: the string was just returned and is held in a local.
            let text = unsafe { ruby.inspect(errinfo.expect("no exception")) };
            assert_eq!(text.as_deref(), Ok("\"nil\""));
        }
        assert_eq!(drops.get(), 1);
    });
}

#[test]
fn body_that_raises_leaves_the_error_it_found() {
    interpreter().run(|ruby| {
        let mut sum = Sum {
            left: 0,
            right: 0,
            finished: false,
        };
        // SAFETY: on the interpreter thread; the exception stays in the local `earlier` and,
        // as the current error, in a variable the collector marks. `raise_body` fits the
        // contract of `protect`.
        unsafe {
            let earlier = ffi::rb_exc_new2(ffi::rb_eArgError, c"earlier".as_ptr());
            ruby.set_current_error(earlier);
            let raised = protect(&mut sum, raise_body).expect_err("the body raises");
            assert_ne!(raised.exception, earlier);
            assert_eq!(ffi::rb_errinfo(), earlier);
            ruby.set_current_error(ffi::Qnil);
            assert_eq!(ffi::rb_errinfo(), ffi::Qnil);
            // A value that is no exception is refused without a jump through this frame.
            ruby.set_current_error(ffi::long2fix(1));
            assert_eq!(ffi::rb_errinfo(), ffi::Qnil);
        }
    });
}

unsafe extern "C" fn throw_body(_data: VALUE) -> VALUE {
    // SAFETY: plain C-API calls on fresh values; nothing here has a destructor.
    unsafe {
        let tag = ffi::id2sym(ffi::rb_intern(c"vkrgss_tag".as_ptr()));
        ffi::rb_funcall(
            ffi::rb_vm_top_self(),
            ffi::rb_intern(c"throw".as_ptr()),
            2,
            tag,
            ffi::long2fix(7),
        )
    }
}

#[test]
fn a_jump_that_is_no_raise_comes_back_as_its_state() {
    interpreter().run(|ruby| {
        ruby.eval_protected(
            "def vkrgss_catching; catch(:vkrgss_tag) { yield; :not_thrown }; end",
            "test",
            1,
        )
        .expect("the method is defined");
        ruby.define_global_function(c"vkrgss_throwing", |ruby: &Ruby, _, _: &[VALUE]| {
            let mut nothing = ();
            // SAFETY: on the interpreter thread; `throw_body` fits the contract of `protect`.
            let raised = unsafe { protect(&mut nothing, throw_body) }.expect_err("it throws");
            assert_ne!(raised.state, ffi::TAG_RAISE);
            assert_eq!(raised.exception, ffi::Qnil);
            // The jump is not continued: describing it says so and clears what it carried.
            // SAFETY: `raised` is in a local on the interpreter thread.
            let error = unsafe { ruby.describe(raised) };
            assert_eq!(error.class_name, "");
            // SAFETY: reading the current error raises nothing.
            assert_eq!(unsafe { ffi::rb_errinfo() }, ffi::Qnil);
            Ok(ffi::long2fix(1))
        })
        .expect("the host function is defined");
        let value = ruby
            .eval_protected("vkrgss_catching { vkrgss_throwing }", "test", 1)
            .expect("nothing is raised");
        // SAFETY: the value was just returned and is held in a local.
        let text = unsafe { ruby.inspect(value) };
        assert_eq!(text.as_deref(), Ok(":not_thrown"));
    });
}
