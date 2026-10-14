//! Classes, modules and methods backed by host functions: what Ruby passes in, and how a
//! value, a raise and a panic leave a host function.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::sync::OnceLock;

use vkxp_ruby192_sys::ffi::{self, VALUE};
use vkxp_ruby192_sys::{
    Arity, GC_STRESS_VARIABLE, HostResult, Interpreter, MIN_STACK_SIZE, Options, Raise, Raised,
    Root, Ruby, RubyError, SIGNALS_TAKEN_BY_RUBY, STACK_HEADROOM, StartError, protect,
};

fn interpreter() -> &'static Interpreter {
    static SHARED: OnceLock<Interpreter> = OnceLock::new();
    SHARED.get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
}

/// Evaluates source and returns `inspect` of its value, or the described exception.
fn eval(source: &'static str) -> Result<String, RubyError> {
    interpreter().run(move |ruby| inspect_eval(ruby, source, "test", 1))
}

fn inspect_eval(ruby: &Ruby, source: &str, file: &str, line: u32) -> Result<String, RubyError> {
    let value = ruby.eval_protected(source, file, line)?;
    // SAFETY: `value` was just returned by the interpreter and is held in a local.
    let text = unsafe { ruby.inspect(value) };
    Ok(text.expect("inspect of a test value does not raise"))
}

fn ok(text: &str) -> Result<String, RubyError> {
    Ok(text.to_owned())
}

/// The public items of `vkxp-ruby-sys` exist here under the same names. This test is the
/// list; that it compiles is the check.
#[test]
fn the_public_items_of_vkxp_ruby192_sys_are_offered() {
    fn named<T: ?Sized>() {}
    named::<Interpreter>();
    named::<Options>();
    named::<StartError>();
    named::<Ruby>();
    named::<Raised>();
    named::<RubyError>();
    named::<HostResult>();
    named::<Raise>();
    named::<Arity>();
    named::<Root>();
    let _: usize = MIN_STACK_SIZE;
    let _: usize = STACK_HEADROOM;
    let _: &str = GC_STRESS_VARIABLE;
    let _: &[libc::c_int] = &SIGNALS_TAKEN_BY_RUBY;
    let _: ffi::VALUE = ffi::Qnil;
    let _ = protect::<()>;
    let _: fn() = vkxp_ruby192_sys::restart_thread_timer;
    let _: fn() -> bool = vkxp_ruby192_sys::threads_lost_in_a_copy;

    let _ = Ruby::eval;
    let _ = Ruby::eval_protected;
    let _ = Ruby::funcall;
    let _ = Ruby::describe;
    let _ = Ruby::set_current_error;
    let _ = Ruby::str_new;
    let _ = Ruby::str_new_text;
    let _ = Ruby::str_new_utf8;
    let _ = Ruby::str_bytes;
    let _ = Ruby::top_self;
    let _ = Ruby::mark_game_scripts;
    let _: vkxp_ruby192_sys::RubyApi = vkxp_ruby192_sys::RUBY_API;
    let _: fn(u8) -> Options = Options::for_rgss;
    let _ = Ruby::inspect;
    let _ = Ruby::root;
    let _ = Ruby::check_arity;
    let _ = Ruby::define_class;
    let _ = Ruby::define_module;
    let _ = Ruby::define_class_under;
    let _ = Ruby::define_module_under;
    let host: fn(&Ruby, VALUE, &[VALUE]) -> HostResult = twice;
    interpreter().run(move |ruby| {
        // SAFETY: the class global and the freshly defined class and module are live.
        unsafe {
            let class = ruby
                .define_class(c"VkxpNamedItems", ffi::rb_cObject)
                .expect("the class is defined");
            let module = ruby.define_module(c"VkxpNamedModule").expect("the module");
            ruby.define_method(class, c"a", twice).unwrap();
            ruby.define_method_with_arity(class, c"b", Arity::Fixed(1), twice)
                .unwrap();
            ruby.define_private_method(class, c"c", twice).unwrap();
            ruby.define_private_method_with_arity(class, c"d", Arity::Any, twice)
                .unwrap();
            ruby.define_singleton_method(class, c"e", twice).unwrap();
            ruby.define_singleton_method_with_arity(class, c"f", Arity::Any, twice)
                .unwrap();
            ruby.define_module_function(module, c"g", twice).unwrap();
            ruby.define_module_function_with_arity(module, c"h", Arity::Any, twice)
                .unwrap();
        }
        ruby.define_global_function(c"vkxp_named_i", twice).unwrap();
        ruby.define_global_function_with_arity(c"vkxp_named_j", Arity::Any, twice)
            .unwrap();
        let _ = host;
    });
    assert_eq!(eval("VkxpNamedItems.new.b(4)"), ok("8"));
}

// --- Host functions ---------------------------------------------------------------------------

fn twice(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    ruby.check_arity(args, 1)?;
    // SAFETY: the argument is a live value for the duration of the call; a conversion error
    // raises from C with no Rust value alive in this frame that needs dropping.
    let number = unsafe { ffi::num2long(args[0]) };
    Ok(ffi::long2fix(number * 2))
}

fn reject(_ruby: &Ruby, _recv: VALUE, _args: &[VALUE]) -> HostResult {
    let _held = String::from("a value with a destructor, dropped before the raise");
    // SAFETY: the class globals are set while the interpreter runs.
    Err(Raise::new(
        unsafe { ffi::rb_eArgError },
        "rejected by the host",
    ))
}

fn explode(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let _held = Box::new([1_u8; 64]);
    if args.is_empty() {
        panic!("host function exploded");
    }
    Ok(ffi::Qnil)
}

/// Calls back into Ruby and lets whatever leaves the call continue through the host frame.
fn call_block_value(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    ruby.check_arity(args, 1)?;
    // SAFETY: the argument is live for the duration of the call.
    let value = unsafe { ruby.funcall(args[0], c"call", &[]) }?;
    Ok(value)
}

/// Returns its arguments, in order, as an array.
fn pair(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    // SAFETY: the arguments are live for the duration of the call; the array is created with
    // no Rust value alive in this frame that needs dropping.
    Ok(unsafe { ffi::rb_ary_new4(args.len() as std::ffi::c_long, args.as_ptr()) })
}

fn define_host_api(ruby: &Ruby) {
    // SAFETY: the class globals and the freshly defined classes and modules are live.
    unsafe {
        let class = ruby
            .define_class(c"VkxpHostClass", ffi::rb_cObject)
            .expect("the class is defined");
        ruby.define_method(class, c"twice", twice).unwrap();
        ruby.define_method(class, c"reject", reject).unwrap();
        ruby.define_method(class, c"explode", explode).unwrap();
        ruby.define_private_method(class, c"hidden", twice).unwrap();
        ruby.define_singleton_method(class, c"twice", twice)
            .unwrap();
        ruby.define_method(
            class,
            c"sum",
            |_ruby: &Ruby, _recv: VALUE, args: &[VALUE]| {
                Ok(ffi::long2fix(
                    args.iter().map(|&arg| ffi::fix2long(arg)).sum(),
                ))
            },
        )
        .unwrap();

        let module = ruby
            .define_module(c"VkxpHostModule")
            .expect("the module is defined");
        ruby.define_module_function(module, c"twice", twice)
            .unwrap();
        let inner = ruby
            .define_class_under(module, c"Inner", class)
            .expect("the nested class is defined");
        ruby.define_method(inner, c"inner?", |_: &Ruby, _: VALUE, _: &[VALUE]| {
            Ok(ffi::Qtrue)
        })
        .unwrap();
        ruby.define_module_under(module, c"Nested")
            .expect("the nested module is defined");
    }
    // SAFETY: the freshly defined class and module are live.
    unsafe {
        let class = ruby
            .define_class(c"VkxpFixedClass", ffi::rb_cObject)
            .expect("the class is defined");
        ruby.define_method_with_arity(class, c"pair", Arity::Fixed(2), pair)
            .unwrap();
        ruby.define_method_with_arity(class, c"none", Arity::Fixed(0), pair)
            .unwrap();
        ruby.define_method_with_arity(class, c"fifteen", Arity::Fixed(15), pair)
            .unwrap();
        ruby.define_method_with_arity(class, c"any", Arity::Any, pair)
            .unwrap();
        ruby.define_method_with_arity(class, c"reject", Arity::Fixed(1), reject)
            .unwrap();
        ruby.define_method_with_arity(class, c"explode", Arity::Fixed(0), explode)
            .unwrap();
        ruby.define_private_method_with_arity(class, c"hidden", Arity::Fixed(2), pair)
            .unwrap();
        ruby.define_singleton_method_with_arity(class, c"pair", Arity::Fixed(2), pair)
            .unwrap();
        let module = ruby
            .define_module(c"VkxpFixedModule")
            .expect("the module is defined");
        ruby.define_module_function_with_arity(module, c"pair", Arity::Fixed(2), pair)
            .unwrap();
    }
    ruby.define_global_function_with_arity(c"vkxp_pair", Arity::Fixed(2), pair)
        .unwrap();
    ruby.define_global_function(c"vkxp_twice", twice).unwrap();
    ruby.define_global_function(c"vkxp_call", call_block_value)
        .unwrap();
}

fn host_eval(source: &'static str) -> Result<String, RubyError> {
    static DEFINED: OnceLock<()> = OnceLock::new();
    DEFINED.get_or_init(|| interpreter().run(define_host_api));
    eval(source)
}

#[test]
fn host_method_is_called_with_its_argument() {
    assert_eq!(host_eval("VkxpHostClass.new.twice(21)"), ok("42"));
}

#[test]
fn host_functions_work_in_every_place_they_can_be_defined() {
    assert_eq!(host_eval("VkxpHostClass.twice(4)"), ok("8"));
    assert_eq!(host_eval("VkxpHostModule.twice(5)"), ok("10"));
    assert_eq!(host_eval("vkxp_twice(6)"), ok("12"));
    assert_eq!(host_eval("VkxpHostClass.new.sum(1, 2, 3, 4)"), ok("10"));
    assert_eq!(host_eval("VkxpHostClass.new.send(:hidden, 7)"), ok("14"));
    assert_eq!(
        host_eval("VkxpHostClass.new.hidden(7) rescue $!.class"),
        ok("NoMethodError")
    );
    assert_eq!(
        host_eval(
            "[VkxpHostModule::Inner.superclass, VkxpHostModule::Inner.new.inner?, VkxpHostModule::Nested.class]"
        ),
        ok("[VkxpHostClass, true, Module]")
    );
}

#[test]
fn redefining_a_class_with_another_superclass_is_an_error_not_a_jump() {
    host_eval("nil").unwrap();
    let error = interpreter().run(|ruby| {
        // SAFETY: the class global is live; the raised exception stays in a local.
        unsafe {
            let raised = ruby
                .define_class(c"VkxpHostClass", ffi::rb_cString)
                .expect_err("the superclass does not match");
            ruby.describe(raised)
        }
    });
    assert_eq!(error.class_name, "TypeError");
    assert_eq!(error.message, "superclass mismatch for class VkxpHostClass");
}

#[test]
fn wrong_argument_count_and_type_raise_in_ruby() {
    assert_eq!(
        host_eval("begin; VkxpHostClass.new.twice; rescue ArgumentError => e; e.message; end"),
        ok("\"wrong number of arguments (0 for 1)\"")
    );
    assert_eq!(
        host_eval("begin; VkxpHostClass.new.twice('x'); rescue TypeError => e; e.class; end"),
        ok("TypeError")
    );
}

#[test]
fn exception_raised_by_a_host_function_is_rescuable() {
    assert_eq!(
        host_eval(
            "begin; VkxpHostClass.new.reject; :not_raised; \
             rescue ArgumentError => e; [:rescued, e.message]; end"
        ),
        ok("[:rescued, \"rejected by the host\"]")
    );
    // Unrescued, it reaches the host as a described exception with the script's position.
    let error = host_eval("\nVkxpHostClass.new.reject").expect_err("nothing rescues it");
    assert_eq!(error.class_name, "ArgumentError");
    assert_eq!(error.message, "rejected by the host");
    assert_eq!(
        error.backtrace[..2],
        ["test:2:in `reject'", "test:2:in `<main>'"]
    );
}

#[test]
fn panic_in_a_host_function_becomes_a_ruby_exception() {
    assert_eq!(
        host_eval(
            "begin; VkxpHostClass.new.explode; :not_raised; \
             rescue Exception => e; [e.class, e.message]; end"
        ),
        ok("[Exception, \"panic in a host function: host function exploded\"]")
    );
    // A bare `rescue` does not swallow it, `ensure` still runs, and the process goes on.
    assert_eq!(
        host_eval(
            "$vkxp_ensured = false; \
             begin; begin; VkxpHostClass.new.explode; rescue; :swallowed; ensure; $vkxp_ensured = true; end; \
             rescue Exception; [:outer, $vkxp_ensured]; end"
        ),
        ok("[:outer, true]")
    );
    assert_eq!(host_eval("VkxpHostClass.new.explode(1)"), ok("nil"));
    assert_eq!(host_eval("VkxpHostClass.new.twice(21)"), ok("42"));
}

#[test]
fn jumps_out_of_a_nested_ruby_call_continue_through_the_host_function() {
    assert_eq!(host_eval("vkxp_call(proc { 5 })"), ok("5"));
    assert_eq!(
        host_eval(
            "begin; vkxp_call(proc { raise IndexError, 'inner' }); rescue IndexError => e; e.message; end"
        ),
        ok("\"inner\"")
    );
    assert_eq!(
        host_eval("catch(:out) { vkxp_call(proc { throw :out, :thrown }); :not_thrown }"),
        ok(":thrown")
    );
}

#[test]
fn many_raises_and_panics_do_not_disturb_the_interpreter() {
    assert_eq!(
        host_eval(
            "h = VkxpHostClass.new; n = 0; \
             2000.times { begin; h.reject; rescue ArgumentError; n += 1; end; \
                          begin; h.explode; rescue Exception; n += 1; end }; n"
        ),
        ok("4000")
    );
}

/// Counts the drops of the guard that `inspect_with_guard` holds across its call into Ruby.
static GUARD_DROPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

struct DropGuard;

impl Drop for DropGuard {
    fn drop(&mut self) {
        GUARD_DROPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Inspects its argument while a value with a destructor is alive in the frame.
fn inspect_with_guard(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    ruby.check_arity(args, 1)?;
    let _guard = DropGuard;
    // SAFETY: the argument is live for the duration of the call.
    let text = unsafe { ruby.inspect(args[0]) }?;
    Ok(ruby.str_new(text.as_bytes())?)
}

#[test]
fn inspect_whose_result_cannot_become_a_string_is_an_error_not_a_jump() {
    use std::sync::atomic::Ordering;

    let (outcome, drops, after) = interpreter().run(|ruby| {
        ruby.define_global_function(c"vkxp_inspect_guarded", inspect_with_guard)
            .unwrap();
        let before = GUARD_DROPS.load(Ordering::SeqCst);
        // `inspect` itself succeeds; the `to_s` of what it returns raises.
        let outcome = ruby
            .eval_protected(
                "class VkxpBadInspect\n\
                 \x20 def inspect; x = Object.new; def x.to_s; raise IOError, 'conversion failed'; end; x; end\n\
                 end\n\
                 vkxp_inspect_guarded(VkxpBadInspect.new)",
                "test",
                1,
            )
            .map(|_| ());
        let drops = GUARD_DROPS.load(Ordering::SeqCst) - before;
        let after = inspect_eval(ruby, "1 + 1", "test", 1);
        (outcome, drops, after)
    });
    let error = outcome.expect_err("the conversion raises");
    assert_eq!(error.class_name, "IOError");
    assert_eq!(error.message, "conversion failed");
    assert_eq!(
        drops, 1,
        "the host frame was unwound normally, not jumped over"
    );
    assert_eq!(after, ok("2"));

    // The same through the helper directly, and when `inspect` itself raises.
    let errors = interpreter().run(|ruby| {
        let bad_result = ruby
            .eval_protected("VkxpBadInspect.new", "test", 1)
            .expect("no exception");
        let bad_method = ruby
            .eval_protected(
                "o = Object.new; def o.inspect; raise IndexError, 'no inspect'; end; o",
                "test",
                1,
            )
            .expect("no exception");
        // SAFETY: both values were just returned and are held in locals; so are the
        // raised exceptions until they are described.
        unsafe {
            let first = ruby.inspect(bad_result).expect_err("the conversion raises");
            let second = ruby.inspect(bad_method).expect_err("inspect raises");
            (ruby.describe(first), ruby.describe(second))
        }
    });
    assert_eq!(errors.0.class_name, "IOError");
    assert_eq!(errors.1.class_name, "IndexError");
    assert_eq!(
        host_eval("vkxp_inspect_guarded([1, :two, 'three'])"),
        ok("\"[1, :two, \\\"three\\\"]\"")
    );
}

// --- Fixed-arity host functions ---------------------------------------------------------------

#[test]
fn fixed_arity_method_reports_its_arity() {
    assert_eq!(
        host_eval(
            "c = VkxpFixedClass; [:pair, :none, :fifteen, :any, :reject, :hidden].map { |m| c.instance_method(m).arity }"
        ),
        ok("[2, 0, 15, -1, 1, 2]")
    );
    assert_eq!(
        host_eval(
            "[VkxpFixedClass.method(:pair).arity, VkxpFixedModule.method(:pair).arity, method(:vkxp_pair).arity]"
        ),
        ok("[2, 2, 2]")
    );
}

#[test]
fn fixed_arity_method_receives_its_arguments_in_order() {
    assert_eq!(
        host_eval("VkxpFixedClass.new.pair(:first, 'second')"),
        ok("[:first, \"second\"]")
    );
    assert_eq!(host_eval("VkxpFixedClass.new.none"), ok("[]"));
    assert_eq!(
        host_eval("VkxpFixedClass.new.fifteen(*(1..15).to_a)"),
        ok("[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]")
    );
    assert_eq!(
        host_eval("VkxpFixedClass.new.any(1, 2, 3)"),
        ok("[1, 2, 3]")
    );
    assert_eq!(
        host_eval(
            "[VkxpFixedClass.pair(1, 2), VkxpFixedModule.pair(3, 4), vkxp_pair(5, 6), VkxpFixedClass.new.send(:hidden, 7, 8)]"
        ),
        ok("[[1, 2], [3, 4], [5, 6], [7, 8]]")
    );
}

#[test]
fn fixed_arity_method_rejects_a_wrong_count_in_ruby() {
    let error = host_eval("VkxpFixedClass.new.pair(1)").expect_err("one argument for two");
    assert_eq!(error.class_name, "ArgumentError");
    // The wording of the interpreter itself for a C function, which has no space.
    assert_eq!(error.message, "wrong number of arguments(1 for 2)");
    let error = host_eval("VkxpFixedClass.new.none(1)").expect_err("one argument for none");
    assert_eq!(error.message, "wrong number of arguments(1 for 0)");
    // The body never ran: the count is checked by Ruby before it is called.
    assert_eq!(
        host_eval("begin; VkxpFixedClass.new.explode(1); rescue ArgumentError => e; e.class; end"),
        ok("ArgumentError")
    );
    assert_eq!(host_eval("VkxpFixedClass.new.pair(1, 2)"), ok("[1, 2]"));
}

#[test]
fn raise_and_panic_in_a_fixed_arity_body_behave_as_in_the_any_arity_case() {
    assert_eq!(
        host_eval(
            "begin; VkxpFixedClass.new.reject(1); :not_raised; \
             rescue ArgumentError => e; [:rescued, e.message]; end"
        ),
        ok("[:rescued, \"rejected by the host\"]")
    );
    assert_eq!(
        host_eval(
            "[VkxpFixedClass.new, VkxpHostClass.new].map { |o| \
             begin; o.explode; rescue Exception => e; [e.class, e.message]; end }"
        ),
        ok(
            "[[Exception, \"panic in a host function: host function exploded\"], \
             [Exception, \"panic in a host function: host function exploded\"]]"
        )
    );
    assert_eq!(host_eval("VkxpFixedClass.new.pair(1, 2)"), ok("[1, 2]"));
}
