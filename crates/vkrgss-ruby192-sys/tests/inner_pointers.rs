//! Values that only a pointer into them holds survive an allocation (patches 41 and 42 of
//! the vendored tree, change `guard-values-held-by-inner-pointers`). The scripts are the
//! reductions of the failures that the stressed suite showed on aarch64; on x86_64 they pass
//! without the patches too, so they guard the aarch64 runs of the suite.
#![cfg(not(vkrgss_interpreter_absent))]

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

/// Evaluates `source` on the interpreter of the tests.
fn eval(source: String) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |ruby| eval_inspect(ruby, &source))
}

/// Evaluates `source` with `GC.stress = true`, and turns stress off again afterwards.
fn eval_stressed(source: &str) -> Result<String, String> {
    eval(format!(
        "begin; GC.stress = true; {source}; ensure; GC.stress = false; end"
    ))
}

#[test]
fn sums_of_two_digit_bignums_are_right_under_stress() {
    assert_eq!(
        eval_stressed("[(2**63 - 1) + 1 == 2**63, (2**63 - 1) - (-1) == 2**63]"),
        Ok("[true, true]".to_owned())
    );
}

/// `File.utime` checks that the time fits `time_t` with the sum of `bigadd_int`, which gave
/// 1 and made every time after 1970 "out of system range".
#[test]
fn a_file_takes_a_time_of_2000_under_stress() {
    let scratch = common::scratch_dir("inner_pointers_utime");
    let file = scratch.join("touched");
    std::fs::write(&file, b"").expect("the file is written");
    let path = file.to_str().expect("a UTF-8 path");
    assert_eq!(
        eval_stressed(&format!(
            "time = Time.local(2000, 1, 1); File.utime(time, time, {path:?}); \
             File.mtime({path:?}) == time"
        )),
        Ok("true".to_owned())
    );
}

/// `Time#_dump` returns a string with instance variables for the fraction, which `w_object`
/// held only through its bytes and its table.
#[test]
fn times_with_a_fraction_marshal_under_stress() {
    for time in ["Time.at(0, Rational(1, 3))", "Time.at(0, 0.123)"] {
        assert_eq!(
            eval_stressed(&format!(
                "time = {time}; dumped = Marshal.dump(time); \
                 [dumped.class, Marshal.load(dumped) == time]"
            )),
            Ok("[String, true]".to_owned()),
            "{time}"
        );
    }
}

/// The search for the local time at a transition of daylight saving time adds Bignums; under
/// stress it picked the wrong offset.
#[test]
fn times_at_transitions_are_the_same_under_stress() {
    let script = "zone = ENV['TZ']; begin; \
         ENV['TZ'] = 'America/Managua'; a = Time.local(1993, 1, 1, 0, 0, 0).inspect; \
         ENV['TZ'] = 'Canada/Newfoundland'; b = Time.local(2007, 11, 4, 0, 1, 0).inspect; \
         [a, b]; ensure; ENV['TZ'] = zone; end";
    let calm = eval(script.to_owned());
    assert!(calm.is_ok(), "{calm:?}");
    assert_eq!(eval_stressed(script), calm);
}
