//! Calling in and out: protected evaluation, interpreter identity, host functions, rooting
//! and `Zlib`, through the crate's own helpers.

mod common;

use std::sync::OnceLock;

use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Interpreter, Options, Raise, Root, Ruby, RubyError};

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

// --- Protected evaluation ---------------------------------------------------------------------

#[test]
fn evaluation_returns_the_value() {
    let value = interpreter().run(|ruby| {
        let value = ruby
            .eval_protected("6 * 7", "test", 1)
            .expect("no exception");
        assert!(ffi::fixnum_p(value));
        ffi::fix2long(value)
    });
    assert_eq!(value, 42);
}

#[test]
fn exception_is_reported_with_class_message_and_backtrace() {
    let error = interpreter()
        .run(|ruby| {
            ruby.eval_protected("x = 1\nraise ArgumentError, 'boom'", "Section001", 1)
                .map(|_| ())
        })
        .expect_err("the source raises");
    assert_eq!(error.class_name, "ArgumentError");
    assert_eq!(error.message, "boom");
    assert_eq!(
        error.backtrace.first().map(String::as_str),
        Some("Section001:2")
    );
    assert_eq!(eval("1 + 1"), ok("2"));
}

#[test]
fn backtrace_follows_the_requested_first_line_and_names_methods() {
    let error = interpreter()
        .run(|ruby| {
            ruby.eval_protected(
                "def vkxp_fail\n  raise IOError, 'deep'\nend\nvkxp_fail",
                "Main",
                100,
            )
            .map(|_| ())
        })
        .expect_err("the source raises");
    assert_eq!(error.class_name, "IOError");
    assert_eq!(error.message, "deep");
    assert_eq!(error.backtrace, ["Main:101:in `vkxp_fail'", "Main:103"]);
    assert_eq!(error.to_string(), "Main:101:in `vkxp_fail': deep (IOError)");
}

#[test]
fn syntax_error_is_reported_and_the_next_evaluation_works() {
    let error = eval("def (").expect_err("the source does not parse");
    assert_eq!(error.class_name, "SyntaxError");
    assert!(error.message.contains("syntax error"), "{}", error.message);
    assert_eq!(eval("1 + 1"), ok("2"));
}

#[test]
fn exit_and_throw_do_not_leave_the_interpreter() {
    let error = eval("exit 3").expect_err("exit raises SystemExit");
    assert_eq!(error.class_name, "SystemExit");
    let error = eval("throw :nowhere").expect_err("an uncaught throw is an error");
    assert_eq!(error.class_name, "NameError");
    assert_eq!(eval("$!"), ok("nil"));
    assert_eq!(eval("1 + 1"), ok("2"));
}

#[test]
fn method_call_by_name_is_protected() {
    let (sum, error) = interpreter().run(|ruby| {
        // SAFETY: fixnums are special constants, and the results stay in locals.
        unsafe {
            let sum = ruby
                .funcall(ffi::long2fix(40), c"+", &[ffi::long2fix(2)])
                .expect("no exception");
            let raised = ruby
                .funcall(ffi::long2fix(1), c"/", &[ffi::long2fix(0)])
                .expect_err("division by zero raises");
            (ffi::fix2long(sum), ruby.describe(raised))
        }
    });
    assert_eq!(sum, 42);
    assert_eq!(error.class_name, "ZeroDivisionError");
    assert_eq!(error.message, "divided by 0");
}

// --- Interpreter identity ---------------------------------------------------------------------

#[test]
fn version_is_1_8_7_patch_level_374() {
    assert_eq!(
        eval("[RUBY_VERSION, RUBY_PATCHLEVEL, RUBY_PLATFORM, RUBY_RELEASE_DATE]"),
        ok("[\"1.8.7\", 374, \"x86_64-linux\", \"2013-06-27\"]")
    );
}

#[test]
fn ruby_1_8_language_points_hold() {
    assert_eq!(eval("x = 1; case x; when 1: :one; end"), ok(":one"));
    assert_eq!(eval("\"abc\"[0]"), ok("97"));
    assert_eq!(eval("[1, [2, 3]].to_s"), ok("\"123\""));
}

#[test]
fn large_integers_promote_to_bignum() {
    assert_eq!(eval("2**64"), ok("18446744073709551616"));
    assert_eq!(eval("(2**64).class"), ok("Bignum"));
    assert_eq!(eval("4611686018427387903 + 1"), ok("4611686018427387904"));
    assert_eq!(eval("(4611686018427387903 + 1).class"), ok("Bignum"));
    assert_eq!(eval("3037000500 * 3037000500"), ok("9223372037000250000"));
    assert_eq!(eval("-4611686018427387904 - 1"), ok("-4611686018427387905"));
}

#[test]
fn marshal_format_is_4_8() {
    assert_eq!(
        eval("[Marshal::MAJOR_VERSION, Marshal::MINOR_VERSION]"),
        ok("[4, 8]")
    );
    assert_eq!(eval("Marshal.dump([1, 'a']).unpack('C2')"), ok("[4, 8]"));
    assert_eq!(
        eval(
            "o = [1, 'a', {:b => 2.5}, 2**70, nil, true, :sym, 1..3]; Marshal.load(Marshal.dump(o)) == o"
        ),
        ok("true")
    );
    let round_trip = interpreter().run(|ruby| {
        let value = ruby
            .eval_protected("{'key' => [1, 2.5, :three]}", "test", 1)
            .expect("no exception");
        // SAFETY: the values come from the interpreter and stay in locals; dumping and
        // loading plain data does not raise.
        unsafe {
            let dumped = ffi::rb_marshal_dump(value, ffi::Qnil);
            let header = ruby.str_bytes(dumped).expect("a string")[..2].to_vec();
            let loaded = ffi::rb_marshal_load(dumped);
            (
                header,
                ruby.inspect(loaded).expect("inspect does not raise"),
            )
        }
    });
    assert_eq!(round_trip.0, [4, 8]);
    assert_eq!(round_trip.1, "{\"key\"=>[1, 2.5, :three]}");
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
    assert_eq!(error.class_name, "NameError");
    assert_eq!(error.message, "VkxpHostClass is already defined");
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
    assert_eq!(error.backtrace, ["test:2:in `reject'", "test:2"]);
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

// --- Rooting ----------------------------------------------------------------------------------

/// Creates the strings in a frame of its own, so that the caller's frame never holds them.
#[inline(never)]
fn make_rooted_strings(ruby: &Ruby, count: usize) -> Vec<Root> {
    (0..count)
        .map(|index| {
            let value = ruby.str_new(format!("rooted string number {index}").as_bytes());
            // SAFETY: the value was just created and is in a local.
            unsafe { ruby.root(value) }
        })
        .collect()
}

/// Overwrites the stack below the caller, where dead copies of values may linger.
#[inline(never)]
fn scrub_stack() {
    let mut scratch = [0_usize; 8192];
    std::hint::black_box(&mut scratch);
}

#[test]
fn rooted_strings_survive_many_collections() {
    interpreter().run(|ruby| {
        const COUNT: usize = 2000;
        let roots = make_rooted_strings(ruby, COUNT);
        scrub_stack();
        for round in 0..30 {
            // Garbage that would reuse the slots of the rooted strings had they been freed.
            ruby.eval_protected(
                "a = []; 20000.times {|i| a << ('garbage %d' % i) << [i] }; a = nil; GC.start",
                "test",
                1,
            )
            .expect("no exception");
            // SAFETY: on the interpreter thread; a collection is always allowed here.
            unsafe { ffi::rb_gc() };
            for (index, root) in roots.iter().enumerate() {
                // SAFETY: the value is rooted, so it is still a live object.
                let bytes = unsafe { ruby.str_bytes(root.get()) };
                assert_eq!(
                    bytes.as_deref(),
                    Some(format!("rooted string number {index}").as_bytes()),
                    "round {round}"
                );
            }
        }
        drop(roots);
        // SAFETY: as above.
        unsafe { ffi::rb_gc() };
    });
    assert_eq!(eval("1 + 1"), ok("2"));
}

#[test]
fn a_root_can_be_replaced_and_keeps_its_new_value() {
    interpreter().run(|ruby| {
        let first = ruby.str_new(b"first");
        // SAFETY: both values are fresh and in locals when they are rooted.
        let mut root = unsafe { ruby.root(first) };
        let second = ruby.str_new(b"second");
        // SAFETY: as above.
        unsafe { root.set(second) };
        // SAFETY: on the interpreter thread; the rooted value is live.
        unsafe {
            ffi::rb_gc();
            assert_eq!(ruby.str_bytes(root.get()).as_deref(), Some(&b"second"[..]));
        }
    });
}

// --- Zlib -------------------------------------------------------------------------------------

#[test]
fn crc_table_has_the_standard_values() {
    assert_eq!(eval("Zlib.crc_table.size"), ok("256"));
    assert_eq!(
        eval("Zlib.crc_table[0, 4].map {|v| '0x%08X' % v }"),
        ok("[\"0x00000000\", \"0x77073096\", \"0xEE0E612C\", \"0x990951BA\"]")
    );
    assert_eq!(
        eval("[Zlib.crc_table[255], Zlib.crc_table.all? {|v| v >= 0 && v <= 0xFFFFFFFF }]"),
        ok("[755167117, true]")
    );
}

#[test]
fn checksum_computed_from_the_table_matches_crc32() {
    assert_eq!(
        eval(
            "t = Zlib.crc_table; c = 0xFFFFFFFF; \
             '123456789'.each_byte {|b| c = t[(c ^ b) & 0xFF] ^ (c >> 8) }; \
             [c ^ 0xFFFFFFFF, Zlib.crc32('123456789')]"
        ),
        ok("[3421780262, 3421780262]")
    );
    assert_eq!(0xCBF4_3926_u32, 3_421_780_262);
}

#[test]
fn deflate_and_inflate_round_trip() {
    assert_eq!(
        eval(
            "s = (0...5000).map {|i| (i * 7 % 251).chr }.join + 'abc' * 1000; \
             d = Zlib::Deflate.deflate(s, 9); \
             [Zlib::Inflate.inflate(d) == s, d.size < s.size, Zlib.adler32('abc')]"
        ),
        ok("[true, true, 38600999]")
    );
    let error = eval("Zlib::Inflate.inflate('not a zlib stream')").expect_err("bad data raises");
    assert_eq!(error.class_name, "Zlib::DataError");
}
