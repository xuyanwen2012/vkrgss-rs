//! The report of an uncaught exception, in the form of RGSS1 and RGSS2 and in that of RGSS3.

use vkrgss_ruby_sys::{Ruby, RubyError};
use vkrgss_vfs::Runtime;

use crate::guest::Guest;
use crate::rb;

use super::prelude::PRELUDE_NAME;

/// The file and line at the start of a backtrace entry such as ``Section001:3:in `update'``.
fn position(entry: &str) -> Option<(&str, u32)> {
    let (file, rest) = entry.split_once(':')?;
    let digits = rest.split(|c: char| !c.is_ascii_digit()).next()?;
    Some((file, digits.parse().ok()?))
}

/// The index of a script in the archive from the file name it is evaluated under.
fn section_index(file: &str) -> Option<usize> {
    let digits = file.strip_prefix("Section")?;
    (digits.len() >= 3 && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse().ok())?
}

/// The index of a script in the archive from the file name it is evaluated under in RGSS3:
/// `{` and the index as four digits and `}`, found anywhere in the name.
fn script_index_rgss3(file: &str) -> Option<usize> {
    let (_, rest) = file.split_once('{')?;
    let (digits, _) = rest.split_once('}')?;
    (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse().ok())?
}

/// The name the archive stores for the script at `index`.
fn script_name(ruby: &Ruby, index: usize) -> Option<String> {
    let name = ruby
        .eval(&format!("$RGSS_SCRIPTS[{index}][1]"), PRELUDE_NAME, 1)
        .ok()?;
    rb::string_bytes(ruby, name).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// The report of an uncaught exception, in the form of the original runtime: the script by
/// the name the archive stores for it, found through the first position of the backtrace
/// (or, for a script that does not parse, of the message) that lies in a script.
fn report(ruby: &Ruby, error: &RubyError) -> String {
    let positions = || {
        error
            .backtrace
            .iter()
            .map(String::as_str)
            .chain(error.message.lines())
            .filter_map(position)
    };
    let in_a_script = positions().find_map(|(file, line)| {
        let name = script_name(ruby, section_index(file)?)?;
        Some((name, line))
    });
    let (name, line) = in_a_script
        .or_else(|| {
            positions()
                .next()
                .map(|(file, line)| (file.to_owned(), line))
        })
        .unwrap_or_default();
    format!(
        "Script '{name}' line {line}: {} occurred.\n\n{}",
        error.class_name, error.message
    )
}

/// The position at which the report of RGSS3 divides a line: the last `:<digits>:` that has
/// something before it, as `(.+):(\d+):(.*)` of Ruby matches. Returns the file, the line and
/// what follows.
fn split_position(line: &str) -> Option<(&str, u32, &str)> {
    let mut search = line.len();
    while let Some(end) = line[..search].rfind(':') {
        let digits_start = line[..end]
            .bytes()
            .rposition(|byte| !byte.is_ascii_digit())
            .map_or(0, |index| index + 1);
        if digits_start < end && digits_start >= 2 && line.as_bytes()[digits_start - 1] == b':' {
            let line_number = line[digits_start..end].parse().unwrap_or(u32::MAX);
            return Some((&line[..digits_start - 1], line_number, &line[end + 1..]));
        }
        search = end;
    }
    None
}

/// The file and line of a backtrace entry as the report of RGSS3 reads them: an entry
/// without a position is the file, and its line is the number it starts with, 0 for most.
fn entry_position(entry: &str) -> (&str, u32) {
    match split_position(entry) {
        Some((file, line, _)) => (file, line),
        None => {
            let digits = entry.bytes().take_while(u8::is_ascii_digit).count();
            (entry, entry[..digits].parse().unwrap_or(0))
        }
    }
}

/// Whether a line starts with the position of a script of RGSS3, `{<digits>}:<digits>:`.
fn starts_with_script_position(line: &str) -> bool {
    let Some((index, rest)) = line
        .strip_prefix('{')
        .and_then(|rest| rest.split_once("}:"))
    else {
        return false;
    };
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    digits(index) && rest.split_once(':').is_some_and(|(line, _)| digits(line))
}

/// The message of a `SyntaxError` as the report of RGSS3 cuts it: the position taken out of
/// the first line that has one, then the first line and the second unless that starts with
/// the position of a script, without the words ` syntax error, `, a leading space and a
/// line of nothing but `^`.
fn syntax_message(message: &str) -> String {
    let mut lines: Vec<String> = message.split('\n').map(str::to_owned).collect();
    if let Some(line) = lines.iter_mut().find(|line| split_position(line).is_some()) {
        *line = split_position(line).map_or_else(String::new, |(_, _, rest)| rest.to_owned());
    }
    let mut text = lines.first().cloned().unwrap_or_default();
    if let Some(second) = lines
        .get(1)
        .filter(|line| !starts_with_script_position(line))
    {
        text.push('\n');
        text.push_str(second);
    }
    for prefix in [" syntax error, ", " "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest.to_owned();
        }
    }
    match text.split_once('\n') {
        Some((first, second)) if second.trim() == "^" => first.to_owned(),
        _ => text,
    }
}

/// The report of an uncaught exception in a game of RGSS3, in the form measured on the
/// original runtime (`ace-v1-api-REPORT.md` section 1.2): the position is that of the first
/// backtrace entry that does not start with `(eval)`, or the last entry; a `SyntaxError`
/// without a line there takes it from its message; a script is named by
/// `$RGSS_SCRIPTS[i][1]` as it is now. Not followed: the empty box for a text longer than
/// 1023 UTF-16 units.
fn report_rgss3(ruby: &Ruby, error: &RubyError) -> String {
    let class = error.class_name.as_str();
    let entry = error
        .backtrace
        .iter()
        .position(|entry| !entry.starts_with("(eval)"))
        .or_else(|| error.backtrace.len().checked_sub(1))
        .map_or("", |index| error.backtrace[index].as_str());
    let (mut name, mut line) = entry_position(entry);
    let mut message = error.message.clone();
    if class == "SyntaxError" {
        // The frame of the call of `eval` itself, `ruby:in `eval'` as on the original,
        // has no line.
        if name.contains(":in `") || line == 0 {
            let first = error.message.lines().find_map(split_position);
            (name, line) = first.map_or((name, line), |(file, number, _)| (file, number));
        }
        message = syntax_message(&error.message);
    } else if class == "Errno::ENOENT" {
        let file = message
            .strip_prefix("No such file or directory - ")
            .unwrap_or(&message);
        return format!("Unable to find file:\n\n{file}");
    }
    let name = match script_index_rgss3(name) {
        Some(index) => script_name(ruby, index).unwrap_or_else(|| name.to_owned()),
        None => name.to_owned(),
    };
    let mut text = if line != 0 {
        format!("Script '{name}' line {line}: {class} occurred.")
    } else {
        format!("{class} occurred.")
    };
    if !message.is_empty() {
        text.push_str("\n\n");
        text.push_str(&message);
    }
    text
}

/// The report of an uncaught exception in a game of the guest's runtime.
pub(super) fn report_of_runtime(ruby: &Ruby, error: &RubyError) -> String {
    if Guest::with(ruby, |guest| guest.runtime.run.runtime) == Runtime::Rgss3 {
        report_rgss3(ruby, error)
    } else {
        report(ruby, error)
    }
}
