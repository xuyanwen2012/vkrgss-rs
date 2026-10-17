//! Regular expressions behave as in the original runtime, whose Ruby is built with Oniguruma.
//!
//! `data/regex.original.txt` is the output of a probe script run in the original runtime: the
//! value of `$KCODE`, then one line per pattern and subject, then the results of a few
//! expressions. The probe's own source stays out of the repository; `PROBE` below evaluates
//! the same patterns, subjects and expressions, taking the first two from the measured lines.

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby_sys::{Interpreter, Options};

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
/// One measured line is adjusted before the comparison, and only as said here:
///
/// - Line 17: the capture is the two bytes `\346\265`, an incomplete character. The `inspect`
///   of Ruby 1.8.1 prints them raw together with the byte that follows the string; 1.8.7
///   prints them as escapes. The captured bytes are the same.
const PROBE: &str = r##"
$KCODE = "UTF8"
vkrgss_measured = File.open(VKRGSS_MEASURED, "rb") { |f| f.read }.split(/\r?\n/)
vkrgss_measured[16] = vkrgss_measured[16].sub("\346\265\0", "\\\\346\\\\265")
vkrgss_result = lambda do |pat, subj|
  begin
    r = eval(pat)
    m = r.match(subj)
    "#{m ? [m.begin(0), m.to_a].inspect : "nil"} | gsub #{subj.gsub(r, "<\\0>").inspect} | scan #{subj.scan(r).length}"
  rescue Exception => e
    "#{e.class}: #{e.message.split("\n").last}"
  end
end
vkrgss_actual = [$KCODE]
vkrgss_measured[1..-1].each do |line|
  quoted = line.scan(/\G"(?:[^"\\]|\\.)*" /m)
  break if quoted.length != 2
  pat, subj = quoted.map { |text| eval(text.gsub("#", "\\\\#")) }
  vkrgss_actual << "#{quoted.join}=> #{vkrgss_result.call(pat, subj)}"
end
vkrgss_actual.concat([
  "abc".sub(/b/){ $~[0].upcase }, ("a,b;c".split(/[,;]/)).inspect,
  ("x1y22".scan(/\d+/)).inspect, "aaa".index(/a/, 1).inspect, "caf\xc3\xa9".length.to_s,
  ("caf\xc3\xa9" =~ /\xc3\xa9/).inspect, "caf\xc3\xa9".scan(/./).length.to_s,
  "\xe6\xb5\x8b\xe8\xaf\x95".split(//).length.to_s
])
vkrgss_actual.concat([
  Regexp.escape("a.b*c[d]"), Regexp.union("a", /b/).inspect, /a/i.inspect, /a/.source,
  (/(a)(b)?/.match("a").to_a).inspect, $~.pre_match.inspect
])
vkrgss_differences = []
[vkrgss_measured.length, vkrgss_actual.length].max.times do |i|
  if vkrgss_measured[i] != vkrgss_actual[i]
    vkrgss_differences << [i + 1, vkrgss_measured[i], vkrgss_actual[i]]
  end
end
[vkrgss_actual.length, vkrgss_differences]
"##;

#[test]
fn results_equal_those_measured_on_the_original() {
    let data = common::manifest_dir().join("tests/data/regex.original.txt");
    let data = data.to_str().expect("UTF-8 path");
    let differences = eval(format!("VKRGSS_MEASURED = {data:?}\n{PROBE}"));
    assert_eq!(differences, Ok("[66, []]".to_owned()));
}

/// The start of such a range was read from an uninitialised variable, so the outcome depended
/// on the compiler; patch 1 of `third_party/oniguruma-5.9.6/PATCHES.md`.
#[test]
fn a_range_that_starts_with_a_class_is_the_error_of_the_original() {
    let source = r#"
['[\d-z]', '[\w-%]', '[\s-a]', '[a\d-z]', '[\d-z]+'].map do |pat|
  begin
    Regexp.new(pat)
    "accepted"
  rescue Exception => e
    [e.class, e.message.include?("mismatch multibyte code length in char-class range")]
  end
end
"#;
    let error = "[RegexpError, true]";
    assert_eq!(
        eval(source.to_owned()),
        Ok(format!("[{error}, {error}, {error}, {error}, {error}]"))
    );
    let literal = r#"
begin
  eval('/[\d-z]+/')
rescue SyntaxError => e
  e.message.include?("mismatch multibyte code length in char-class range")
end
"#;
    assert_eq!(eval(literal.to_owned()), Ok("true".to_owned()));
}

#[test]
fn a_hyphen_after_a_class_that_ends_the_class_stays_literal() {
    let source = r#"
[
  "1-" =~ /\A[\d-]+\z/, "a" =~ /[\d-]/, "x-7" =~ /[\d-&&\d]/, "x-7" =~ /[\d-&&-]/,
  "m5-" =~ /\A[a-z\d]+/, $~[0]
]
"#;
    assert_eq!(
        eval(source.to_owned()),
        Ok(r#"[0, nil, 2, 1, 0, "m5"]"#.to_owned())
    );
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
