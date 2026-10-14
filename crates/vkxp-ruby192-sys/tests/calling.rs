//! Calling in: protected evaluation, what an exception was, strings as bytes and as text,
//! the accessors of strings and arrays, rooting and `Zlib`, through the crate's own helpers.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::sync::OnceLock;

use vkxp_ruby192_sys::ffi::{self, VALUE};
use vkxp_ruby192_sys::{Interpreter, Options, RUBY_API, Root, Ruby, RubyApi, RubyError};

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
    let first = error.backtrace.first().expect("a backtrace");
    assert!(first.starts_with("Section001:2"), "{first}");
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
    assert_eq!(
        error.backtrace[..2],
        ["Main:101:in `vkxp_fail'", "Main:103:in `<main>'"]
    );
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
    assert_eq!(error.class_name, "ArgumentError");
    assert_eq!(error.message, "uncaught throw :nowhere");
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

#[test]
fn top_self_is_the_main_object() {
    let text = interpreter().run(|ruby| {
        // SAFETY: the main object lives as long as the interpreter.
        unsafe { ruby.inspect(ruby.top_self()) }.expect("inspect of main does not raise")
    });
    assert_eq!(text, "main");
    assert_eq!(
        eval("self.equal?(TOPLEVEL_BINDING.eval('self'))"),
        ok("true")
    );
}

#[test]
fn large_integers_promote_to_bignum() {
    assert_eq!(eval("2**64"), ok("18446744073709551616"));
    assert_eq!(eval("(2**64).class"), ok("Bignum"));
    assert_eq!(eval("4611686018427387903 + 1"), ok("4611686018427387904"));
    assert_eq!(eval("(4611686018427387903 + 1).class"), ok("Bignum"));
}

// --- Strings ----------------------------------------------------------------------------------

/// What Ruby says about a string the host made: the name of its encoding and its size.
fn describe_string(ruby: &Ruby, string: VALUE) -> (String, i64) {
    // SAFETY: `string` is a live string held in a local of the caller.
    unsafe {
        let encoding = ruby.funcall(string, c"encoding", &[]).expect("encoding");
        let name = ruby.funcall(encoding, c"name", &[]).expect("name");
        let name = String::from_utf8(ruby.str_bytes(name).expect("a string")).expect("a name");
        let size = ruby.funcall(string, c"size", &[]).expect("size");
        (name, ffi::fix2long(size))
    }
}

#[test]
fn bytes_are_binary_and_text_is_utf_8() {
    let bytes = [0xe3_u8, 0x81, 0x82];
    let (binary, text, binary_bytes, text_bytes) = interpreter().run(move |ruby| {
        let binary = ruby.str_new(&bytes).expect("a string of bytes");
        let text = ruby.str_new_utf8("あ").expect("a string of text");
        // SAFETY: both strings are live and held in locals.
        let (binary_bytes, text_bytes) = unsafe { (ruby.str_bytes(binary), ruby.str_bytes(text)) };
        (
            describe_string(ruby, binary),
            describe_string(ruby, text),
            binary_bytes,
            text_bytes,
        )
    });
    assert_eq!(binary, ("ASCII-8BIT".to_owned(), 3));
    assert_eq!(text, ("UTF-8".to_owned(), 1));
    assert_eq!(binary_bytes.as_deref(), Some(&bytes[..]));
    assert_eq!(text_bytes.as_deref(), Some(&bytes[..]));
    // Not a string: no bytes.
    let none = interpreter().run(|ruby| {
        // SAFETY: a fixnum is a special constant.
        unsafe { ruby.str_bytes(ffi::long2fix(1)) }
    });
    assert_eq!(none, None);
}

#[test]
fn accessors_read_embedded_and_heap_strings_and_arrays() {
    interpreter().run(|ruby| {
        // SAFETY: every value is created here, held in a local and read before anything
        // else runs in the interpreter.
        unsafe {
            for len in [0_usize, 3, 23, 24, 100] {
                let source = format!("'{}'", "x".repeat(len));
                let string = ruby.eval_protected(&source, "test", 1).expect("a string");
                assert_eq!(ffi::rb_type(string), ffi::T_STRING);
                assert_eq!(ffi::rstring_len(string), len as i64, "length {len}");
                let bytes = std::slice::from_raw_parts(ffi::rstring_ptr(string).cast::<u8>(), len);
                assert!(bytes.iter().all(|&byte| byte == b'x'), "length {len}");
                let embedded = (*(string as *const ffi::RBasic)).flags & ffi::RSTRING_NOEMBED == 0;
                assert_eq!(embedded, len <= ffi::RSTRING_EMBED_LEN_MAX, "length {len}");
                assert_eq!(ruby.str_bytes(string).map(|bytes| bytes.len()), Some(len));
            }
            for len in [0_usize, 2, 3, 4, 100] {
                let source = format!("(0...{len}).map {{|i| i * 3 }}");
                let array = ruby.eval_protected(&source, "test", 1).expect("an array");
                assert_eq!(ffi::rb_type(array), ffi::T_ARRAY);
                assert_eq!(ffi::rarray_len(array), len as i64, "length {len}");
                let elements = std::slice::from_raw_parts(ffi::rarray_ptr(array), len);
                for (index, &element) in elements.iter().enumerate() {
                    assert_eq!(ffi::fix2long(element), index as i64 * 3, "length {len}");
                    assert_eq!(ffi::rb_ary_entry(array, index as i64), element);
                }
            }
            // An array literal of two elements keeps them inside the object.
            let pair = ruby.eval_protected("[1, 2]", "test", 1).expect("an array");
            assert_ne!(
                (*(pair as *const ffi::RBasic)).flags & ffi::RARRAY_EMBED_FLAG,
                0
            );
        }
    });
}

#[test]
fn text_takes_bytes_and_labels_them_utf_8() {
    let bytes = [0xe3_u8, 0x81, 0x82];
    let (binary, text, text_bytes) = interpreter().run(move |ruby| {
        let binary = ruby.str_new(&bytes).expect("a string of bytes");
        let text = ruby.str_new_text(&bytes).expect("a string of text");
        // SAFETY: the string is live and held in a local.
        let text_bytes = unsafe { ruby.str_bytes(text) };
        (
            describe_string(ruby, binary),
            describe_string(ruby, text),
            text_bytes,
        )
    });
    assert_eq!(binary, ("ASCII-8BIT".to_owned(), 3));
    assert_eq!(text, ("UTF-8".to_owned(), 1));
    assert_eq!(text_bytes.as_deref(), Some(&bytes[..]));
}

#[test]
fn text_that_is_not_utf_8_keeps_its_bytes_and_its_label() {
    let (name, valid, bytes) = interpreter().run(|ruby| {
        let text = ruby.str_new_text(&[0xff]).expect("a string of text");
        // SAFETY: the string is live and held in a local.
        unsafe {
            let valid = ruby.funcall(text, c"valid_encoding?", &[]).expect("valid");
            (describe_string(ruby, text).0, valid, ruby.str_bytes(text))
        }
    });
    assert_eq!(name, "UTF-8");
    assert_eq!(valid, ffi::Qfalse);
    assert_eq!(bytes.as_deref(), Some(&[0xff_u8][..]));
}

// --- The surface shared with vkxp-ruby-sys ----------------------------------------------------

#[test]
fn the_interpreter_is_ruby_1_9() {
    assert_eq!(RUBY_API, RubyApi::Ruby19);
    assert_eq!(eval("RUBY_VERSION"), ok("\"1.9.2\""));
}

#[test]
fn options_for_a_runtime_are_the_defaults() {
    let default = Options::default();
    for generation in [1, 2, 3] {
        let Options {
            stack_size,
            script_name,
        } = Options::for_rgss(generation);
        assert_eq!(stack_size, default.stack_size, "generation {generation}");
        // RGSS3 gives its interpreter no script name (change
        // `match-the-endings-and-loading-of-rgss3`).
        let expected = if generation == 3 {
            ""
        } else {
            &default.script_name
        };
        assert_eq!(script_name, expected, "generation {generation}");
    }
}

#[test]
fn the_mark_of_the_game_scripts_does_nothing() {
    let value = interpreter().run(|ruby| {
        ruby.eval_protected("1", "test", 1).expect("no exception");
        ruby.mark_game_scripts();
        ruby.mark_game_scripts();
        ffi::fix2long(
            ruby.eval_protected("6 * 7", "test", 1)
                .expect("no exception"),
        )
    });
    assert_eq!(value, 42);
}

// --- Rooting ----------------------------------------------------------------------------------

/// Makes strings that only host heap memory refers to.
#[inline(never)]
fn make_rooted_strings(ruby: &Ruby, count: usize) -> Vec<Root> {
    (0..count)
        .map(|index| {
            let text = format!("rooted string number {index}, long enough to have a buffer");
            let string = ruby.str_new_utf8(&text).expect("a string");
            // SAFETY: `string` is live in a local until it is rooted.
            unsafe { ruby.root(string) }.expect("the root is registered")
        })
        .collect()
}

/// Overwrites the part of the stack that the calls above used, so that no stale copy of a
/// value keeps an object alive by chance.
#[inline(never)]
fn scrub_stack() {
    let mut junk = [0_usize; 4096];
    std::hint::black_box(&mut junk);
}

#[test]
fn rooted_strings_survive_many_collections() {
    interpreter().run(|ruby| {
        let roots = make_rooted_strings(ruby, 64);
        scrub_stack();
        for _ in 0..20 {
            ruby.eval_protected("100.times { 'garbage ' * 50 }; GC.start", "test", 1)
                .expect("collections run");
        }
        for (index, root) in roots.iter().enumerate() {
            // SAFETY: the value is rooted.
            let bytes = unsafe { ruby.str_bytes(root.get()) }.expect("still a string");
            assert_eq!(
                String::from_utf8(bytes).expect("text"),
                format!("rooted string number {index}, long enough to have a buffer")
            );
        }
    });
}

#[test]
fn a_root_can_be_replaced_and_keeps_its_new_value() {
    interpreter().run(|ruby| {
        let mut roots = make_rooted_strings(ruby, 1);
        let replacement = ruby.str_new_utf8("the replacement").expect("a string");
        // SAFETY: `replacement` is live in a local.
        unsafe { roots[0].set(replacement) };
        scrub_stack();
        ruby.eval_protected(
            "GC.start; 100.times { 'garbage ' * 50 }; GC.start",
            "test",
            1,
        )
        .expect("collections run");
        // SAFETY: the value is rooted.
        let bytes = unsafe { ruby.str_bytes(roots[0].get()) }.expect("still a string");
        assert_eq!(bytes, b"the replacement");
    });
}

// --- Zlib -------------------------------------------------------------------------------------

#[test]
fn crc_table_has_the_standard_values() {
    assert_eq!(eval("Zlib.crc_table.size"), ok("256"));
    assert_eq!(
        eval("Zlib.crc_table[0, 4].map {|value| '0x%08X' % value }"),
        ok("[\"0x00000000\", \"0x77073096\", \"0xEE0E612C\", \"0x990951BA\"]")
    );
    assert_eq!(eval("Zlib.crc_table.max < 2**32"), ok("true"));
}

#[test]
fn checksum_computed_from_the_table_matches_crc32() {
    assert_eq!(
        eval(
            "t = Zlib.crc_table; c = 0xFFFFFFFF; \
             '123456789'.each_byte {|b| c = t[(c ^ b) & 0xFF] ^ (c >> 8) }; \
             [c ^ 0xFFFFFFFF, Zlib.crc32('123456789')].map {|value| '0x%08X' % value }"
        ),
        ok("[\"0xCBF43926\", \"0xCBF43926\"]")
    );
}

#[test]
fn deflate_and_inflate_round_trip() {
    assert_eq!(
        eval(
            "s = (0..255).map {|i| i.chr }.join * 40; d = Zlib::Deflate.deflate(s); \
             [d.size < s.size, Zlib::Inflate.inflate(d) == s, d.encoding.name]"
        ),
        ok("[true, true, \"ASCII-8BIT\"]")
    );
}
