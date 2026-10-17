//! Values of the original runtime: identity, `Marshal`, random numbers, float text and
//! rounding.
//!
//! Every expected value here is the one the specification gives, measured on the original
//! runtime of RPG Maker VX Ace (`RGSS301.dll`), except the platform, which is that of this
//! build. No patch makes them so: the interpreter is the release the original embeds.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

fn interpreter() -> &'static Interpreter {
    static SHARED: OnceLock<Interpreter> = OnceLock::new();
    SHARED.get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
}

fn eval(source: &str) -> Result<String, String> {
    let source = source.to_owned();
    interpreter().run(move |ruby| eval_inspect(ruby, &source))
}

/// `RUBY_PLATFORM` of the build, which names the architecture and the system of the target.
const PLATFORM: &str = if cfg!(target_os = "android") {
    "aarch64-linux-android"
} else if cfg!(target_arch = "aarch64") {
    "aarch64-linux"
} else {
    "x86_64-linux"
};

fn ok(text: &str) -> Result<String, String> {
    Ok(text.to_owned())
}

/// `Marshal.dump` of each expression of the list, as hexadecimal.
fn dumps(expressions: &str) -> Vec<String> {
    let source = format!("[{expressions}].map {{|value| Marshal.dump(value).unpack('H*')[0] }}");
    let text = eval(&source).expect("the values are dumped");
    text.trim_matches(|character| character == '[' || character == ']')
        .split(", ")
        .map(|dump| dump.trim_matches('"').to_owned())
        .collect()
}

// --- Identity ---------------------------------------------------------------------------------

#[test]
fn version_constants_are_those_of_the_original() {
    assert_eq!(
        eval(
            "[RUBY_VERSION, RUBY_PATCHLEVEL, RUBY_RELEASE_DATE, RUBY_REVISION, RUBY_ENGINE, \
              RUBY_COPYRIGHT]"
        ),
        ok("[\"1.9.2\", 0, \"2010-08-18\", 29036, \"ruby\", \
             \"ruby - Copyright (C) 1993-2010 Yukihiro Matsumoto\"]")
    );
}

#[test]
fn description_and_platform_name_this_build() {
    assert_eq!(
        eval("[RUBY_DESCRIPTION, RUBY_PLATFORM]"),
        ok(&format!(
            "[\"ruby 1.9.2p0 (2010-08-18 revision 29036) [{PLATFORM}]\", \"{PLATFORM}\"]"
        ))
    );
}

// --- Marshal ----------------------------------------------------------------------------------

#[test]
fn an_object_is_dumped_as_the_original_dumps_it() {
    assert_eq!(
        eval(
            "class AceV0Obj\n\
             \x20 def initialize\n\
             \x20   @a = 1\n\
             \x20   @b = \"x\"\n\
             \x20   @c = 1.5\n\
             \x20   @d = :sym\n\
             \x20   @e = [nil, true]\n\
             \x20 end\n\
             end\n\
             Marshal.dump(AceV0Obj.new).unpack('H*')[0]"
        ),
        ok(
            "\"04086f3a0d41636556304f626a0a3a07406169063a07406249220678063a0645543a0740636608312e\
             353a0740643a0873796d3a0740655b073054\""
        )
    );
}

#[test]
fn strings_are_dumped_as_the_original_dumps_them() {
    assert_eq!(
        dumps(
            "\"abc\", \"abc\".force_encoding(\"ASCII-8BIT\"), \"abc\".force_encoding(\"US-ASCII\"), \
             \"abc\".force_encoding(\"Shift_JIS\"), \"abc\".encode(\"UTF-16LE\"), \"あ\""
        ),
        [
            "0408492208616263063a064554",
            "04082208616263",
            "0408492208616263063a064546",
            "0408492208616263063a0d656e636f64696e67220e53686966745f4a4953",
            "040849220b610062006300063a0d656e636f64696e67220d5554462d31364c45",
            "0408492208e38182063a064554",
        ]
    );
}

#[test]
fn floats_are_dumped_as_the_original_dumps_them() {
    assert_eq!(
        dumps("1.5, 0.1, 1e23, 1.0 / 3, -0.0, 1e100, 1.0 / 0, 0.0 / 0, 100.0"),
        [
            "04086608312e35",
            "0408661b302e313030303030303030303030303030303100999a",
            "0408661e392e39393939393939393939393939393932652b3232004af6",
            "0408661b302e3333333333333333333333333333333331005555",
            "040866072d30",
            "0408660e31652b31303000c37d",
            "04086608696e66",
            "040866086e616e",
            "04086608313030",
        ]
    );
}

#[test]
fn symbols_and_other_values_are_dumped_as_the_original_dumps_them() {
    assert_eq!(
        dumps(":sym, :\"あ\", [:a, :a], {1 => 2}, 2**40, 1..2, /a/i, nil, 300, -129"),
        [
            "04083a0873796d",
            "0408493a08e38182063a064554",
            "04085b073a06613b00",
            "04087b0669066907",
            "04086c2b08000000000001",
            "04086f3a0a52616e6765083a096578636c463a0a626567696e69063a08656e646907",
            "0408492f066101063a064546",
            "040830",
            "040869022c01",
            "040869ff7f",
        ]
    );
}

#[test]
fn a_string_dumped_without_encoding_is_loaded_as_binary() {
    assert_eq!(
        eval("Marshal.load(\"\\x04\\b\\\"\\babc\".force_encoding(\"ASCII-8BIT\")).encoding.name"),
        ok("\"ASCII-8BIT\"")
    );
    assert_eq!(
        eval("[Marshal::MAJOR_VERSION, Marshal::MINOR_VERSION]"),
        ok("[4, 8]")
    );
}

// --- Random numbers ---------------------------------------------------------------------------

#[test]
fn integers_and_a_float_after_srand_1_and_the_further_draws() {
    assert_eq!(
        eval(
            "srand(1); a = (1..5).map { rand(1000000) }; a << rand; \
             a + [rand(10), rand(2**40), rand(1.5).to_i, rand(-5).to_i]"
        ),
        ok(
            "[128037, 491755, 470924, 791624, 491263, 0.12812444792935673, \
             5, 340316410816, 0, 0]"
        )
    );
}

#[test]
fn floats_after_srand_1() {
    assert_eq!(
        eval("srand(1); [rand, rand, rand]"),
        ok("[0.417022004702574, 0.7203244934421581, 0.00011437481734488664]")
    );
}

#[test]
fn other_seeds() {
    assert_eq!(
        eval("srand(5); (1..6).map { rand(1000) }"),
        ok("[867, 206, 701, 998, 118, 400]")
    );
    assert_eq!(
        eval("srand(20261009); a = (1..5).map { rand(1000000) }; a << (\"%.17g\" % rand)"),
        ok("[452058, 88078, 612515, 880807, 883900, \"0.11596889136695721\"]")
    );
}

#[test]
fn random_objects() {
    assert_eq!(
        eval(
            "r = Random.new(1); a = (1..5).map { r.rand(1000000) }; a << r.rand; \
             a << r.bytes(4).unpack('H*')[0]"
        ),
        ok("[128037, 491755, 470924, 791624, 491263, 0.12812444792935673, \"851ec1ff\"]")
    );
}

#[test]
fn shuffle_and_sample() {
    assert_eq!(
        eval("srand(1); [(1..10).to_a.shuffle, (1..10).to_a.sample]"),
        ok("[[9, 4, 2, 10, 6, 8, 3, 1, 7, 5], 5]")
    );
}

// --- Float text and rounding ------------------------------------------------------------------

#[test]
fn float_text() {
    assert_eq!(
        eval(
            "[0.1 + 0.2, 1e23, 1.0 / 3, 1e22, 1e16, 1e15, 100.0, 1e-5, 1e-4, 1e300, -0.0, \
              2.0**70, 5e-324].map {|value| value.to_s }"
        ),
        ok(
            "[\"0.30000000000000004\", \"1.0e+23\", \"0.3333333333333333\", \"1.0e+22\", \
             \"1.0e+16\", \"1000000000000000.0\", \"100.0\", \"1.0e-05\", \"0.0001\", \
             \"1.0e+300\", \"-0.0\", \"1180591620717411300000.0\", \"5.0e-324\"]"
        )
    );
}

#[test]
fn rounding_and_division() {
    assert_eq!(
        eval(
            "[2.5.round, 3.5.round, (-2.5).round, 2.675.round(2), 10.divmod(3), -7 / 2, -7 % 3, \
              7.fdiv(2)]"
        ),
        ok("[3, 4, -3, 2.68, [3, 1], -4, 2, 3.5]")
    );
}
