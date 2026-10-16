//! The functions of `Math` and the power of floats are correctly rounded: they come from the
//! vendored CORE-MATH (`third_party/core-math`). Special arguments and errors are as before.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::fmt::Write as _;
use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

fn eval(source: String) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |ruby| eval_inspect(ruby, &source))
}

/// The committed test vectors, which are kept with the tests of `vkrgss-ruby-sys`.
const VECTORS: &str = include_str!("../../vkrgss-ruby-sys/tests/data/core-math/vectors.txt");

/// How each function of the vectors is evaluated, with `x` and `y` its first and second
/// inputs (for `atan2` the first is `y`). Ruby 1.9.2 has every function of the vectors.
const EXPRESSIONS: &[(&str, &str)] = &[
    ("sin", "Math.sin(x)"),
    ("cos", "Math.cos(x)"),
    ("tan", "Math.tan(x)"),
    ("asin", "Math.asin(x)"),
    ("acos", "Math.acos(x)"),
    ("atan", "Math.atan(x)"),
    ("atan2", "Math.atan2(x, y)"),
    ("sinh", "Math.sinh(x)"),
    ("cosh", "Math.cosh(x)"),
    ("tanh", "Math.tanh(x)"),
    ("asinh", "Math.asinh(x)"),
    ("acosh", "Math.acosh(x)"),
    ("atanh", "Math.atanh(x)"),
    ("exp", "Math.exp(x)"),
    ("log", "Math.log(x)"),
    ("log2", "Math.log2(x)"),
    ("log10", "Math.log10(x)"),
    ("pow", "x ** y"),
    ("cbrt", "Math.cbrt(x)"),
    ("hypot", "Math.hypot(x, y)"),
    ("erf", "Math.erf(x)"),
    ("erfc", "Math.erfc(x)"),
    ("tgamma", "Math.gamma(x)"),
    ("lgamma", "Math.lgamma(x)[0]"),
];

/// The bits of a C99 hexadecimal float of the vectors, which are all normal numbers.
fn hex_float_bits(text: &str) -> u64 {
    let (negative, rest) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let rest = rest
        .strip_prefix("0x")
        .unwrap_or_else(|| panic!("not a hexadecimal float: {text}"));
    let (digits, exponent) = rest
        .split_once('p')
        .unwrap_or_else(|| panic!("no exponent: {text}"));
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    assert_eq!(whole, "1", "not normalised: {text}");
    assert!(fraction.len() <= 13, "too many digits: {text}");
    let fraction = format!("{fraction:0<13}");
    let mantissa = u64::from_str_radix(&fraction, 16).expect("hexadecimal digits");
    let exponent: i64 = exponent.parse().expect("a decimal exponent");
    let biased = u64::try_from(exponent + 1023).expect("a normal number");
    assert!((1..2047).contains(&biased), "not a normal number: {text}");
    (u64::from(negative) << 63) | (biased << 52) | mantissa
}

/// A Ruby expression for the float with the bits `bits`.
fn ruby_float(bits: u64) -> String {
    format!("[{bits}].pack('Q').unpack('D')[0]")
}

#[test]
fn the_functions_give_the_bits_of_the_test_vectors() {
    let mut source = String::from("failed = []\n");
    let mut evaluated = 0;
    for line in VECTORS.lines() {
        let fields: Vec<&str> = line.split(',').collect();
        let name = fields[0];
        let expression = EXPRESSIONS
            .iter()
            .find(|(function, _)| *function == name)
            .unwrap_or_else(|| panic!("no expression for {name}"))
            .1;
        let (result, inputs) = fields[1..].split_last().expect("inputs and a result");
        let x = ruby_float(hex_float_bits(inputs[0]));
        let y = inputs
            .get(1)
            .map_or("nil".to_owned(), |input| ruby_float(hex_float_bits(input)));
        writeln!(
            source,
            "x = {x}; y = {y}; r = [{expression}].pack('D').unpack('Q')[0]; \
             failed << {line:?} + ' gave ' + r.to_s(16) if r != {}",
            hex_float_bits(result)
        )
        .unwrap();
        evaluated += 1;
    }
    source.push_str("failed");
    assert_eq!(evaluated, 5851, "every line of the vectors");
    assert_eq!(eval(source), Ok("[]".to_owned()));
}

/// The values, and the classes of the errors, that these arguments gave before the functions
/// came from CORE-MATH. Ruby 1.9.2 checks the domain before it calls a function.
#[test]
fn special_arguments_behave_as_before() {
    for (expression, expected) in [
        ("Math.log(-1)", "Math::DomainError"),
        ("Math.sqrt(-1)", "Math::DomainError"),
        ("Math.acos(2)", "Math::DomainError"),
        ("Math.atanh(1)", "Infinity"),
        ("Math.log(0)", "-Infinity"),
        ("Math.exp(1000)", "Infinity"),
        ("0.0 ** -1", "Infinity"),
    ] {
        let source = format!("begin; ({expression}).to_s; rescue => e; e.class.to_s; end");
        assert_eq!(eval(source), Ok(format!("{expected:?}")), "{expression}");
    }
}

/// `Math.lgamma` gives the sign of the gamma function with the logarithm of its absolute
/// value, and that sign is what CORE-MATH's `lgamma` leaves in `signgam`.
#[test]
fn lgamma_gives_the_sign_of_gamma() {
    assert_eq!(
        eval(
            "[-0.5, -1.5, -2.5, 0.5, 3.0, 1.0].map { |x| \
             [Math.lgamma(x)[1], Math.gamma(x) < 0 ? -1 : 1] }"
                .to_owned()
        ),
        Ok("[[-1, -1], [1, 1], [-1, -1], [1, 1], [1, 1], [1, 1]]".to_owned())
    );
}
