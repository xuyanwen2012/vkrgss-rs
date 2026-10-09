//! Tests of the argument helpers and the error mapping, through the interpreter: a module of
//! host functions that exists only in this test binary exposes them to Ruby. Every snippet
//! is also evaluated with a garbage collection at every allocation.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use vkxp_core::{BitmapError, Color, CreateFailure, Table};
use vkxp_ruby_sys::ffi::VALUE;
use vkxp_ruby_sys::{HostResult, Ruby, RubyError};
use vkxp_vfs::{FileKind, GameFiles, VfsError};

use crate::error::{self, Error};
use crate::wrap::{self, Owned};
use crate::{rb, testing};

fn probe_int(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    Ok(rb::fix(rb::int(ruby, args[0])?))
}

fn probe_float(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::float_new(ruby, rb::float(ruby, args[0])?)
}

fn probe_text(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::string_new(ruby, rb::text(ruby, args[0])?.as_bytes())
}

fn probe_to_text(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::string_new(ruby, rb::to_text(ruby, args[0])?.as_bytes())
}

/// Takes one or two arguments and returns how many it got.
fn probe_count(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::check_count(ruby, args, 1, 2)?;
    Ok(rb::fix(args.len() as i32))
}

/// Fails with the error the symbol names.
fn probe_fail(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let which = rb::to_text(ruby, args[0])?;
    let files = GameFiles::new(testing::game_dir(), []);
    let error: Error = match which.as_str() {
        "disposed" => BitmapError::Disposed.into(),
        "create" => BitmapError::CreateFailed(CreateFailure::ZeroSide {
            width: 0,
            height: 1,
        })
        .into(),
        "image" => vkxp_image::decode(b"not an image").unwrap_err().into(),
        "missing" => files
            .read(FileKind::Image, "Graphics/Pictures/Nope")
            .unwrap_err()
            .into(),
        "unreadable" => VfsError::Io {
            path: testing::game_dir().to_owned(),
            source: std::io::ErrorKind::PermissionDenied.into(),
        }
        .into(),
        "decode" => Color::from_bytes(&[0; 31]).unwrap_err().into(),
        "table" => Table::new_3d(i32::MAX, i32::MAX, i32::MAX)
            .unwrap_err()
            .into(),
        other => panic!("no error is called {other}"),
    };
    Err(error::to_raise(ruby, error))
}

/// How many [`Held`] values were dropped.
static HELD_DROPS: AtomicUsize = AtomicUsize::new(0);

/// Held by the tests that call the probes with a [`Held`], so that one of them can count.
static HELD_PROBES: Mutex<()> = Mutex::new(());

/// A value with a destructor, alive in a host function while a conversion fails.
struct Held;

impl Drop for Held {
    fn drop(&mut self) {
        HELD_DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

/// Holds a value with a destructor while its argument fails to be a `String`.
fn probe_held_bytes(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let _held = Held;
    rb::bytes(ruby, args[0])?;
    Ok(args[0])
}

/// Holds a value with a destructor while its argument fails to be a `Color`.
fn probe_held_color(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let _held = Held;
    wrap::expect::<Owned<Color>>(ruby, args[0])?;
    Ok(args[0])
}

fn define(ruby: &Ruby) {
    let module = ruby.define_module(c"BindingProbe").expect("a new module");
    rb::class_method(ruby, module, c"int", 1, probe_int).unwrap();
    rb::class_method(ruby, module, c"float", 1, probe_float).unwrap();
    rb::class_method(ruby, module, c"text", 1, probe_text).unwrap();
    rb::class_method(ruby, module, c"to_text", 1, probe_to_text).unwrap();
    rb::class_method(ruby, module, c"count", -1, probe_count).unwrap();
    rb::class_method(ruby, module, c"fail", 1, probe_fail).unwrap();
    rb::class_method(ruby, module, c"held_bytes", 1, probe_held_bytes).unwrap();
    rb::class_method(ruby, module, c"held_color", 1, probe_held_color).unwrap();
}

/// Evaluates a snippet, and once more with a collection at every allocation, which must
/// not change the outcome.
fn eval(source: &str) -> Result<String, RubyError> {
    static DEFINED: OnceLock<()> = OnceLock::new();
    DEFINED.get_or_init(|| testing::run(define));
    let plain = testing::eval(source);
    let stressed = testing::eval(&format!(
        "GC.stress = true\nbegin\n{source}\nensure\nGC.stress = false\nend"
    ));
    let outcome = |result: &Result<String, RubyError>| match result {
        Ok(inspect) => Ok(inspect.clone()),
        Err(error) => Err((error.class_name.clone(), error.message.clone())),
    };
    assert_eq!(
        outcome(&plain),
        outcome(&stressed),
        "under stress: {source}"
    );
    plain
}

/// The class name and message of the exception `source` raises.
fn raised(source: &str) -> (String, String) {
    let error = eval(source).expect_err("the snippet raises");
    (error.class_name, error.message)
}

fn raises(class: &str, message: &str) -> (String, String) {
    (class.to_owned(), message.to_owned())
}

#[test]
fn integers_convert_as_num2int() {
    assert_eq!(eval("BindingProbe.int(42)").as_deref(), Ok("42"));
    assert_eq!(eval("BindingProbe.int(-7)").as_deref(), Ok("-7"));
    // A Float is truncated toward zero.
    assert_eq!(eval("BindingProbe.int(3.9)").as_deref(), Ok("3"));
    assert_eq!(eval("BindingProbe.int(-3.9)").as_deref(), Ok("-3"));
    // A Bignum-class value that fits is accepted; Ruby 1.8 on 64 bits has none within 32
    // bits, so the boundaries are Fixnums.
    assert_eq!(
        eval("BindingProbe.int(2**31 - 1)").as_deref(),
        Ok("2147483647")
    );
    assert_eq!(
        eval("BindingProbe.int(-2**31)").as_deref(),
        Ok("-2147483648")
    );
}

#[test]
fn integers_out_of_range_raise_range_error() {
    assert_eq!(
        raised("BindingProbe.int(2**31)"),
        raises(
            "RangeError",
            "integer 2147483648 too big to convert to `int'"
        )
    );
    assert_eq!(raised("BindingProbe.int(2**70)").0, "RangeError");
}

#[test]
fn a_wrong_type_for_an_integer_raises_type_error() {
    assert_eq!(
        raised("BindingProbe.int('12')"),
        raises("TypeError", "can't convert String into Integer")
    );
    assert_eq!(raised("BindingProbe.int(nil)").0, "TypeError");
    assert_eq!(raised("BindingProbe.int([1])").0, "TypeError");
}

#[test]
fn floats_convert_from_any_number() {
    assert_eq!(eval("BindingProbe.float(12.5)").as_deref(), Ok("12.5"));
    assert_eq!(eval("BindingProbe.float(3)").as_deref(), Ok("3.0"));
    assert_eq!(
        eval("BindingProbe.float(2**70)").as_deref(),
        Ok("1.18059162071741e+21")
    );
}

#[test]
fn a_wrong_type_for_a_float_raises_type_error() {
    assert_eq!(raised("BindingProbe.float('red')").0, "TypeError");
    assert_eq!(raised("BindingProbe.float(nil)").0, "TypeError");
}

#[test]
fn strings_convert_and_invalid_utf8_is_replaced() {
    assert_eq!(eval("BindingProbe.text('abc')").as_deref(), Ok("\"abc\""));
    assert_eq!(
        eval("BindingProbe.text(\"a\\377b\") == \"a\\357\\277\\275b\"").as_deref(),
        Ok("true")
    );
    assert_eq!(
        raised("BindingProbe.text(12)"),
        raises("TypeError", "can't convert Fixnum into String")
    );
    assert_eq!(eval("BindingProbe.to_text(123)").as_deref(), Ok("\"123\""));
    assert_eq!(eval("BindingProbe.to_text('x')").as_deref(), Ok("\"x\""));
    assert_eq!(eval("BindingProbe.to_text(nil)").as_deref(), Ok("\"\""));
}

#[test]
fn a_wrong_count_raises_argument_error() {
    assert_eq!(
        eval("[BindingProbe.count(1), BindingProbe.count(1, 2)]").as_deref(),
        Ok("[1, 2]")
    );
    assert_eq!(
        raised("BindingProbe.count"),
        raises("ArgumentError", "wrong number of arguments (0 for 1)")
    );
    assert_eq!(
        raised("BindingProbe.count(1, 2, 3)"),
        raises("ArgumentError", "wrong number of arguments (3 for 2)")
    );
    // A method registered with a fixed arity is checked by Ruby itself.
    assert_eq!(
        raised("BindingProbe.int(1, 2)"),
        raises("ArgumentError", "wrong number of arguments (2 for 1)")
    );
    assert_eq!(
        eval("[BindingProbe.method(:int).arity, BindingProbe.method(:count).arity]").as_deref(),
        Ok("[1, -1]")
    );
}

#[test]
fn rgss_error_is_a_standard_error() {
    assert_eq!(
        eval("[RGSSError.superclass, RGSSError.new('x').message]").as_deref(),
        Ok("[StandardError, \"x\"]")
    );
}

#[test]
fn each_error_maps_to_its_exception() {
    assert_eq!(
        raised("BindingProbe.fail(:disposed)"),
        raises("RGSSError", "disposed bitmap")
    );
    assert_eq!(
        raised("BindingProbe.fail(:create)"),
        raises("RGSSError", "failed to create bitmap")
    );
    assert_eq!(
        raised("BindingProbe.fail(:image)"),
        raises("RGSSError", "failed to create bitmap")
    );
    assert_eq!(
        raised("BindingProbe.fail(:missing)"),
        raises(
            "Errno::ENOENT",
            "No such file or directory - Graphics/Pictures/Nope"
        )
    );
    assert_eq!(raised("BindingProbe.fail(:unreadable)").0, "IOError");
    assert_eq!(
        raised("BindingProbe.fail(:decode)"),
        raises(
            "ArgumentError",
            "serialized colour must be 32 bytes long, found 31"
        )
    );
    assert_eq!(raised("BindingProbe.fail(:table)").0, "ArgumentError");
}

/// An instance of an anonymous class that is frozen: naming the class for a message has to
/// store the name in the class, which raises.
const UNNAMEABLE: &str = "c = Class.new; Object.instance_method(:freeze).bind(c).call; c.new";

#[test]
fn a_failure_to_name_the_class_of_a_wrong_argument_is_raised_after_destructors_ran() {
    let _alone = HELD_PROBES.lock().unwrap_or_else(PoisonError::into_inner);
    let before = HELD_DROPS.load(Ordering::SeqCst);
    for probe in ["held_bytes", "held_color"] {
        assert_eq!(
            raised(&format!("BindingProbe.{probe}(({UNNAMEABLE}))")),
            raises("TypeError", "can't modify frozen object"),
            "{probe}"
        );
    }
    // Each snippet ran twice, plain and under stress, and every call dropped its value.
    assert_eq!(HELD_DROPS.load(Ordering::SeqCst) - before, 4);
    // The interpreter goes on, and so does the class through its public surface.
    assert_eq!(eval("BindingProbe.int(1)").as_deref(), Ok("1"));
    assert_eq!(
        raised(&format!("Font.new(['A', ({UNNAMEABLE})])")),
        raises("TypeError", "can't modify frozen object")
    );
    assert_eq!(eval("Font.new(['A']).name").as_deref(), Ok("[\"A\"]"));
}

#[test]
fn wrong_type_messages_name_the_class() {
    let _alone = HELD_PROBES.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(
        raised("BindingProbe.held_bytes(12)"),
        raises("TypeError", "can't convert Fixnum into String")
    );
    assert_eq!(
        raised("BindingProbe.held_color(Tone.new(0, 0, 0))"),
        raises("TypeError", "wrong argument type Tone (expected Color)")
    );
    assert_eq!(
        raised("BindingProbe.held_color(nil)"),
        raises("TypeError", "wrong argument type NilClass (expected Color)")
    );
    // An anonymous class that can be named is named as Ruby names it.
    assert_eq!(
        eval("begin; BindingProbe.held_bytes(Class.new.new); rescue TypeError; $!.message =~ /\\Acan't convert #<Class:0x[0-9a-f]+> into String\\z/ ? :named : $!.message; end").as_deref(),
        Ok(":named")
    );
}
