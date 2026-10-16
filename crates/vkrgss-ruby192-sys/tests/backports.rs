//! The security fixes backported to the vendored tree (`docs/vendored-security.md`): what a
//! few lines can show of each defect, and that the results beside it are unchanged.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkrgss_ruby192_sys::{Interpreter, Options};

fn eval(source: &'static str) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |ruby| eval_inspect(ruby, source))
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
            "digits = '1.' + '5' * 300_000
             Marshal.load(\"\\x04\\x08f\\x03\" + [digits.size].pack('V')[0, 3] + digits).floor"
        ),
        Ok("1".to_owned())
    );
    // Short strings convert as before.
    assert_eq!(
        eval("['0.1'.to_f, '1e3'.to_f, '123456789.125'.to_f, '-2.5e-3'.to_f, '1_0.5'.to_f]"),
        Ok("[0.1, 1000.0, 123456789.125, -0.0025, 10.5]".to_owned())
    );
}

/// CVE-2014-4975: the line feed after a last group that filled the buffer of `encodes` was
/// written one byte past it, on the stack.
#[test]
fn base64_of_a_long_line_ends_with_its_line_feed() {
    assert_eq!(
        eval(
            "line = 'a' * 3071
             packed = [line].pack('m3072')
             [packed == [line].pack('m0') + \"\\n\", packed.unpack('m')[0] == line]"
        ),
        Ok("[true, true]".to_owned())
    );
    // Short lines pack as before.
    assert_eq!(
        eval("[['abc'].pack('m'), ['abcd'].pack('m0'), ['ab'].pack('u')]"),
        Ok(r#"["YWJj\n", "YWJjZA==", "\"86(`\n"]"#.to_owned())
    );
}

/// CVE-2018-8778: a count above `LONG_MAX` was negative, and `@` moved before the string.
#[test]
fn an_unpack_count_above_the_largest_long_is_refused() {
    assert_eq!(
        eval("'abc'.unpack('@18446744073709551615C')"),
        Err("RangeError: pack length too big".to_owned())
    );
    assert_eq!(
        eval("['abc'.unpack('@2C'), 'abc'.unpack('x1X1C')]"),
        Ok("[[99], [97]]".to_owned())
    );
}

/// CVE-2017-0898: with the largest width the buffer for an infinite or NaN value did not
/// grow, and the text was written that far past it.
#[test]
fn the_largest_width_of_a_non_finite_float_is_refused() {
    assert_eq!(
        eval("format('%2147483647f', 1.0 / 0)"),
        Err("ArgumentError: width too big".to_owned())
    );
    assert_eq!(
        eval("format('%*f', -2147483647, 0.0 / 0)"),
        Err("ArgumentError: width too big".to_owned())
    );
    assert_eq!(
        eval("[format('%6f|%-6f|%+f', 1.0 / 0, -1.0 / 0, 0.0 / 0), format('%8.3f', 2.5)]"),
        Ok(r#"["   Inf|-Inf  |+NaN", "   2.500"]"#.to_owned())
    );
}

/// CVE-2026-27820: `GzipReader#ungetc` of more bytes than the buffer had free wrote them
/// past its end.
#[test]
fn bytes_put_back_into_a_gzip_reader_are_read_again() {
    assert_eq!(
        eval(
            "sink = Object.new
             def sink.data; @data ||= ''.force_encoding('ASCII-8BIT'); end
             def sink.write(text); data << text; text.size; end
             writer = Zlib::GzipWriter.new(sink)
             writer.write('abcdefgh' * 4)
             writer.finish
             source = Object.new
             source.instance_variable_set(:@left, sink.data.dup)
             def source.read(size = nil, buffer = nil)
               return nil if @left.empty?
               part = size ? @left.slice!(0, size) : @left.slice!(0..-1)
               buffer ? buffer.replace(part) : part
             end
             reader = Zlib::GzipReader.new(source)
             first = reader.read(3)
             reader.ungetc('x' * 100_000)
             rest = reader.read
             [first, rest.size, rest.count('x'), rest[-29..-1]]"
        ),
        Ok(r#"["abc", 100029, 100000, "defghabcdefghabcdefghabcdefgh"]"#.to_owned())
    );
}

/// Oniguruma 9574bc1: the distance of nested repeats indexed a table as a negative number,
/// and the compilation of the pattern ended the process.
#[test]
fn nested_repeats_compile() {
    assert_eq!(
        eval("('x' * 10 + 'b') =~ /(?:(?:(?:a{0,100000}){0,100000}){0,100000})bc/"),
        Ok("nil".to_owned())
    );
}
