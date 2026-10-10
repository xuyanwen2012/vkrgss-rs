//! Regular expressions behave as in the original runtime, whose Ruby is built with Oniguruma.
//!
//! `data/regex.original.txt` is the output of a probe script run in the original runtime: the
//! value of `$KCODE`, then one line per pattern and subject, then the results of a few
//! expressions. The probe's own source stays out of the repository; `PROBE` below evaluates
//! the same patterns, subjects and expressions, taking the first two from the measured lines.

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkxp_ruby_sys::{Interpreter, Options};

fn eval(source: String) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |_| eval_inspect(&source))
}

/// Rebuilds the measured file line by line and returns the lines that differ, each as its
/// number (from 1), the measured text and the text this interpreter gives. The pattern and
/// the subject are read from the measured line, which shows them with `inspect`; a `#` in
/// them is text, because the `inspect` of Ruby 1.8.1 does not escape it.
///
/// Two measured lines are adjusted before the comparison, and only as said here:
///
/// - Line 17: the capture is the two bytes `\346\265`, an incomplete character. The `inspect`
///   of Ruby 1.8.1 prints them raw together with the byte that follows the string; 1.8.7
///   prints them as escapes. The captured bytes are the same.
/// - Line 40: Oniguruma 5.9.6 rejects `[\d-z]` as the original's older Oniguruma does, with
///   another message. See "Follow-ups" in the design of the change `embed-oniguruma`.
const PROBE: &str = r##"
$KCODE = "UTF8"
vkxp_measured = File.open(VKXP_MEASURED, "rb") { |f| f.read }.split(/\r?\n/)
vkxp_measured[16] = vkxp_measured[16].sub("\346\265\0", "\\\\346\\\\265")
vkxp_measured[39] = vkxp_measured[39].sub(
  "mismatch multibyte code length in char-class range", "empty range in char class")
vkxp_result = lambda do |pat, subj|
  begin
    r = eval(pat)
    m = r.match(subj)
    "#{m ? [m.begin(0), m.to_a].inspect : "nil"} | gsub #{subj.gsub(r, "<\\0>").inspect} | scan #{subj.scan(r).length}"
  rescue Exception => e
    "#{e.class}: #{e.message.split("\n").last}"
  end
end
vkxp_actual = [$KCODE]
vkxp_measured[1..-1].each do |line|
  quoted = line.scan(/\G"(?:[^"\\]|\\.)*" /m)
  break if quoted.length != 2
  pat, subj = quoted.map { |text| eval(text.gsub("#", "\\\\#")) }
  vkxp_actual << "#{quoted.join}=> #{vkxp_result.call(pat, subj)}"
end
vkxp_actual.concat([
  "abc".sub(/b/){ $~[0].upcase }, ("a,b;c".split(/[,;]/)).inspect,
  ("x1y22".scan(/\d+/)).inspect, "aaa".index(/a/, 1).inspect, "caf\xc3\xa9".length.to_s,
  ("caf\xc3\xa9" =~ /\xc3\xa9/).inspect, "caf\xc3\xa9".scan(/./).length.to_s,
  "\xe6\xb5\x8b\xe8\xaf\x95".split(//).length.to_s
])
vkxp_actual.concat([
  Regexp.escape("a.b*c[d]"), Regexp.union("a", /b/).inspect, /a/i.inspect, /a/.source,
  (/(a)(b)?/.match("a").to_a).inspect, $~.pre_match.inspect
])
vkxp_differences = []
[vkxp_measured.length, vkxp_actual.length].max.times do |i|
  if vkxp_measured[i] != vkxp_actual[i]
    vkxp_differences << [i + 1, vkxp_measured[i], vkxp_actual[i]]
  end
end
[vkxp_actual.length, vkxp_differences]
"##;

#[test]
fn results_equal_those_measured_on_the_original() {
    let data = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/regex.original.txt");
    let differences = eval(format!("VKXP_MEASURED = {data:?}\n{PROBE}"));
    assert_eq!(differences, Ok("[66, []]".to_owned()));
}

#[test]
fn the_pattern_that_stopped_a_game_is_accepted() {
    assert_eq!(
        eval(r#"eval('/\[Vv][([0-9]+)]/').class"#.to_owned()),
        Ok("Regexp".to_owned())
    );
}

#[test]
fn multi_byte_characters_are_single_characters() {
    assert_eq!(
        eval(r#"$KCODE = "UTF8"; ["测试".scan(/./).length, "测试".split(//).length]"#.to_owned()),
        Ok("[2, 2]".to_owned())
    );
}
