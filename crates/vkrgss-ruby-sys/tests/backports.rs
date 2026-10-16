//! The security fixes backported to the vendored trees (`docs/vendored-security.md`): what
//! a few lines can show of each defect, and that the results beside it are unchanged.

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby_sys::{Interpreter, Options};

/// Evaluates in the interpreter all tests of this file share. `$KCODE` is set before the
/// source is parsed, because a regular expression literal is compiled when it is parsed:
/// a test that ran before may have left another encoding behind (a pattern with the option
/// `n` that is refused does), and the tests run in any order.
fn eval(source: &str) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    let source = source.to_owned();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |_| {
            eval_inspect("$KCODE = 'U'")?;
            eval_inspect(&source)
        })
}

/// CVE-2013-4164: a digit string of this length overflowed the heap in `ruby_strtod`. The
/// same conversion reads a dumped Float in `Marshal.load`.
#[test]
fn a_very_long_digit_string_converts_to_a_float() {
    assert_eq!(
        eval("(('0.' + '1' * 300_000).to_f * 9 - 1.0).abs < 1e-12"),
        Ok("true".to_owned())
    );
    assert_eq!(
        eval(
            "digits = '1.' + '5' * 300_000; Marshal.load(\"\\004\\010f\" + Marshal.dump(digits)[3..-1]).floor"
        ),
        Ok("1".to_owned())
    );
    // Short strings convert as before.
    assert_eq!(
        eval("['0.1'.to_f, '1e3'.to_f, '123456789.125'.to_f, '-2.5e-3'.to_f, '1_0.5'.to_f]"),
        Ok("[0.1, 1000.0, 123456789.125, -0.0025, 10.5]".to_owned())
    );
}

/// CVE-2018-8778: a count that does not fit made `@` move the read position before the
/// string.
#[test]
fn an_unpack_count_that_does_not_fit_is_an_error() {
    let error = eval("'0123456789'.unpack('@18446744073709551516C10')").unwrap_err();
    assert!(
        error.starts_with("#<RangeError:") && error.contains("pack length too big"),
        "{error}"
    );
    assert_eq!(
        eval(
            "['0123456789'.unpack('@8C2'), 'abc'.unpack('a2@0a1'), [1, 2].pack('C2').unpack('C*')]"
        ),
        Ok("[[56, 57], [\"ab\", \"a\"], [1, 2]]".to_owned())
    );
}

/// Oniguruma `distance_value` (upstream 9574bc1): the distance of these repeats does not
/// fit a signed number, which indexed a table far before its start when the pattern was
/// compiled.
#[test]
fn a_repeat_with_a_huge_distance_compiles() {
    assert_eq!(
        eval("$KCODE = 'U'; Regexp.new('W.?{888}{888}{888}\\\\x00').class"),
        Ok("Regexp".to_owned())
    );
}

/// The Oniguruma fixes that check the end of the subject before they read it change no
/// result: a back-reference that matches nothing at the end, a multibyte class at the end,
/// and an alternative that peeks at the next byte.
#[test]
fn matches_at_the_end_of_the_subject_are_as_before() {
    assert_eq!(
        eval(
            "$KCODE = 'U'; ['aa' =~ /(a*)\\1x/, 'aax' =~ /(a*)\\1x/, 'aSs' =~ /[\\303\\237]/i, \
             \"a\\303\\237\" =~ /[\\303\\237]/, 'ab' =~ /a(?:b|c)?d?\\z/, 'xa' =~ /a*?b|a\\z/]"
        ),
        Ok("[nil, 0, 1, 1, 0, 1]".to_owned())
    );
}

/// Patterns the engine now refuses instead of reading past them: one that ends inside a
/// multibyte character, and an octal byte above 255.
#[test]
fn patterns_that_were_read_out_of_bounds_are_errors() {
    for (pattern, message) in [
        ("Regexp.new(\"a\\346\")", "too short multibyte code string"),
        ("Regexp.new('[\\\\777]', nil, 'n')", "too big number"),
        ("Regexp.new('\\\\777')", "too big number"),
    ] {
        let error = eval(&format!("$KCODE = 'U'; {pattern}")).unwrap_err();
        assert!(
            error.starts_with("#<RegexpError:") && error.contains(message),
            "{pattern}: {error}"
        );
    }
    // Complete characters and octal bytes up to 255 are accepted as before.
    assert_eq!(
        eval(
            "$KCODE = 'U'; [\"a\\346\\265\\213\" =~ Regexp.new(\"\\346\\265\\213\"), \"\\377\" =~ Regexp.new('\\\\377', nil, 'n'), 'A' =~ /\\101/]"
        ),
        Ok("[1, 0, 0]".to_owned())
    );
}

/// Oniguruma 33e5b47c of ruby/ruby (CVE-2024-27282): `backward_search_range` added the least
/// distance to the literal or class it looks for, `reg->dmin`, to the start of the range
/// before it compared that distance with the subject. A plain pattern longer than the
/// subject, such as `/abc/`, is turned away before by the threshold length, but the least
/// distance of nested repeats of 100000 does not fit the 32 bits of `OnigDistance` and is
/// the infinite distance, for which no threshold is set: the search reached the addition
/// with the literal (exact), the literal of three bytes (exact with Boyer-Moore, which a
/// backward search over so short a range takes as exact), the class (map) and the literal
/// before an anchor, and formed a position about 4 GiB past the subject. Nothing read
/// through it, and a sanitized build reported nothing there before the patch.
#[test]
fn backward_searches_longer_than_the_subject_find_nothing() {
    assert_eq!(
        eval(
            "r = '(?:(?:(?:.{100000}){100000}){100000}){100000}'
             ['ab'.rindex(/abc/), 'ab'.rindex(Regexp.new('ab' + r)),
              'ab'.rindex(Regexp.new(r + 'b')), 'ab'.rindex(Regexp.new(r + 'bcd')),
              'ab'.rindex(Regexp.new(r + '[bc]')), 'ab'.rindex(Regexp.new(r + 'b$'))]"
        ),
        Ok("[nil, nil, nil, nil, nil, nil]".to_owned())
    );
    // Backward searches that fit the subject find what they found before.
    assert_eq!(
        eval(
            "['abcab'.rindex(/ab/), 'abcab'.rindex(/b.{2}b/), 'abcab'.rindex(/[bc]/), 'ab'.rindex(/b$/)]"
        ),
        Ok("[3, 1, 4, 1]".to_owned())
    );
}
