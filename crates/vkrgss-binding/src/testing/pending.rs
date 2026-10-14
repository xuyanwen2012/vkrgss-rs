//! The allow-list of tests that cannot pass over Ruby 1.9.2 yet.
//!
//! The tests of the binding run over both interpreters from the same files. A test or a
//! snippet case that expects behaviour of RGSS1 or RGSS2, or wording of Ruby 1.8, is named
//! in `tests/ruby192.allow` of the package that is built over Ruby 1.9.2, with its reason.
//! Over Ruby 1.8 the list is not read and nothing here has an effect.
//!
//! The file holds entries separated by blank lines. An entry is a reason, written as comment
//! lines, followed by one or more lines of the forms
//!
//! ```text
//! test <binary> <name of the test as vkrgss-binding prints it>
//! case <name of the snippet case>
//! ```
//!
//! where `<binary>` is the name of the test file, or `lib` for a unit test.
//!
//! A listed test has [`pending_over_ruby_1_9_2!`](crate::pending_over_ruby_1_9_2) as its
//! first statement. The test is then run alone, in a process of its own, and counts as passed
//! while it fails there; when it passes, the run fails and says that its entry is to be
//! removed. A test whose entry was removed runs in place, like a test without the marker.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use crate::rb;

/// The list file, below the directory of the package under test.
pub const ALLOW_LIST: &str = "tests/ruby192.allow";

/// The environment variable that marks the process a listed test is run alone in. Its value
/// is the name of that test.
const CHILD_VARIABLE: &str = "VKRGSS_PENDING_TEST";

/// The tests and cases of the list, each with the reason of its entry.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct AllowList {
    /// Binary, name and reason.
    tests: Vec<(String, String, String)>,
    /// Name and reason.
    cases: Vec<(String, String)>,
}

impl AllowList {
    /// Parses the text of a list. The error names the line that is wrong.
    pub(crate) fn parse(text: &str) -> Result<AllowList, String> {
        let mut list = AllowList::default();
        let mut reason = String::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            let wrong = |what: &str| format!("{ALLOW_LIST}, line {}: {what}: `{line}`", index + 1);
            if line.is_empty() {
                reason.clear();
            } else if let Some(comment) = line.strip_prefix('#') {
                if !reason.is_empty() {
                    reason.push(' ');
                }
                reason.push_str(comment.trim());
            } else {
                if reason.trim().is_empty() {
                    return Err(wrong("an entry without a reason"));
                }
                let (kind, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
                let mut words = rest.split_whitespace();
                match (kind, words.next(), words.next(), words.next()) {
                    ("test", Some(binary), Some(name), None) => {
                        list.tests
                            .push((binary.to_owned(), name.to_owned(), reason.clone()));
                    }
                    // The name of a case is the rest of the line: it may hold spaces.
                    ("case", Some(_), _, _) => {
                        list.cases.push((rest.trim().to_owned(), reason.clone()));
                    }
                    _ => {
                        return Err(wrong("neither `test <binary> <name>` nor `case <name>`"));
                    }
                }
            }
        }
        Ok(list)
    }

    /// The reason of the entry of a test, if it is listed.
    pub(crate) fn test(&self, binary: &str, name: &str) -> Option<&str> {
        self.tests
            .iter()
            .find(|(listed_binary, listed, _)| listed_binary == binary && listed == name)
            .map(|(_, _, reason)| reason.as_str())
    }

    /// Whether a snippet case is listed.
    pub(crate) fn case(&self, name: &str) -> bool {
        self.cases.iter().any(|(listed, _)| listed == name)
    }
}

/// A test as the marker finds it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Marked {
    /// The name of the test file, or `lib`.
    binary: String,
    /// The name as `vkrgss-binding` prints it, which is how the list names it.
    name: String,
    /// The name as this executable knows it.
    exact: String,
}

impl Marked {
    /// Takes apart the type name of a function nested in a test,
    /// `<crate>::<modules>::<test>::<local>`: the first segment is the binary, where the
    /// name of `library` stands for `lib`; the last is dropped; in a test file a leading `t`
    /// is the module that wraps the file of `vkrgss-binding`.
    pub(crate) fn from_type_name(type_name: &str, library: &str) -> Option<Marked> {
        let mut segments: Vec<&str> = type_name.split("::").collect();
        segments.pop()?;
        if segments.len() < 2 {
            return None;
        }
        let first = segments.remove(0);
        let exact = segments.join("::");
        let (binary, name) = if first == library {
            ("lib", exact.clone())
        } else {
            let name = match segments.split_first() {
                Some((&"t", rest)) if !rest.is_empty() => rest.join("::"),
                _ => exact.clone(),
            };
            (first, name)
        };
        Some(Marked {
            binary: binary.to_owned(),
            name,
            exact,
        })
    }
}

/// How a listed test ended when it was run alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// One test ran and failed.
    Failed,
    /// One test ran and passed.
    Passed,
    /// The process ended without reporting one test as passed or failed.
    NoTest,
    /// The process was ended by this signal.
    Signal(i32),
}

impl Outcome {
    /// Reads the outcome from what the process of a test harness printed and how it ended.
    pub(crate) fn of(stdout: &str, signal: Option<i32>) -> Outcome {
        if let Some(signal) = signal {
            return Outcome::Signal(signal);
        }
        let count = |line: &str, what: &str| -> Option<u32> {
            let before = line.split(what).next()?;
            before.split_whitespace().last()?.parse().ok()
        };
        let counts = stdout
            .lines()
            .filter(|line| line.starts_with("test result:"))
            .find_map(|line| Some((count(line, " passed")?, count(line, " failed")?)));
        match counts {
            Some((0, 1)) => Outcome::Failed,
            Some((1, 0)) => Outcome::Passed,
            _ => Outcome::NoTest,
        }
    }
}

/// The part that decides: what a listed test that ended alone with `outcome` means. `Ok` is
/// the reason of its entry, to be printed, and the test counts as passed; `Err` is the
/// message the test fails with.
pub(crate) fn judge(list: &AllowList, marked: &Marked, outcome: Outcome) -> Result<String, String> {
    let Marked { binary, name, .. } = marked;
    let Some(reason) = list.test(binary, name) else {
        return Err(format!("`{binary} {name}` is not in {ALLOW_LIST}"));
    };
    match outcome {
        Outcome::Failed => Ok(reason.to_owned()),
        Outcome::Passed => Err(format!(
            "`{name}` passes over Ruby 1.9.2: remove its entry `test {binary} {name}` from \
             {ALLOW_LIST} and its marker"
        )),
        Outcome::NoTest => Err(format!(
            "`{name}` is listed in {ALLOW_LIST}, and no test ran when it was run alone: the \
             process did not report one test as passed or failed"
        )),
        Outcome::Signal(signal) => Err(format!(
            "`{name}` is listed in {ALLOW_LIST}, and the process that ran it alone was ended \
             by signal {signal}: that is a fault, not a reason to list a test"
        )),
    }
}

/// The name of the listed test that this process runs alone, in a form for a file name, or
/// `None` in any other process. Such a process is started beside the one that started it,
/// from the same executable, and must not share its files.
pub(crate) fn alone() -> Option<String> {
    let name = std::env::var(CHILD_VARIABLE).ok()?;
    Some(name.replace("::", "."))
}

/// The list of the package under test.
fn list() -> &'static AllowList {
    static LIST: OnceLock<AllowList> = OnceLock::new();
    LIST.get_or_init(|| {
        let dir = std::env::var_os("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test run");
        let path = PathBuf::from(dir).join(ALLOW_LIST);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        AllowList::parse(&text).unwrap_or_else(|error| panic!("{error}"))
    })
}

/// Runs the test alone in a process of its own and returns how it ended.
fn run_alone(marked: &Marked) -> Outcome {
    let executable = std::env::current_exe().expect("the path of the test executable");
    let output = Command::new(executable)
        .args([&marked.exact, "--exact", "--test-threads=1"])
        .env(CHILD_VARIABLE, &marked.exact)
        .output()
        .expect("the test executable starts again");
    #[cfg(unix)]
    let signal = std::os::unix::process::ExitStatusExt::signal(&output.status);
    #[cfg(not(unix))]
    let signal = None;
    Outcome::of(&String::from_utf8_lossy(&output.stdout), signal)
}

/// The function behind [`pending_over_ruby_1_9_2!`](crate::pending_over_ruby_1_9_2), which
/// passes the type name of a function nested in the test. Returns whether the test is done:
/// `false` lets its body run.
///
/// # Panics
///
/// Panics when a listed test passes alone, or ends otherwise than as a failed test.
pub fn pending(type_name: &str) -> bool {
    if !rb::RUBY_19 {
        return false;
    }
    let Some(marked) = Marked::from_type_name(type_name, env!("CARGO_CRATE_NAME")) else {
        panic!("the marker of the allow-list stands in `{type_name}`, which is not a test");
    };
    let list = list();
    if list.test(&marked.binary, &marked.name).is_none() {
        return false;
    }
    if std::env::var_os(CHILD_VARIABLE).is_some_and(|name| name == marked.exact.as_str()) {
        return false;
    }
    match judge(list, &marked, run_alone(&marked)) {
        Ok(reason) => {
            println!(
                "`{}` fails over Ruby 1.9.2 as {ALLOW_LIST} says: {reason}",
                marked.name
            );
            true
        }
        Err(message) => panic!("{message}"),
    }
}

/// Whether a snippet case is on the allow-list. Always `false` over Ruby 1.8.
pub fn pending_case(name: &str) -> bool {
    rb::RUBY_19 && list().case(name)
}

/// The first statement of a test that is on the allow-list of Ruby 1.9.2: see
/// [`testing::pending`](crate::testing::pending). Over Ruby 1.8, and for a test that is not
/// listed, it does nothing.
#[macro_export]
macro_rules! pending_over_ruby_1_9_2 {
    () => {{
        fn marker() {}
        if $crate::testing::pending(::std::any::type_name_of_val(&marker)) {
            return;
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "\
# The header of the file.

# Symbol#to_i is gone
# in Ruby 1.9.
test run_game the_first_script
test lib foundation_tests::a_test
case a case of two colours

# Another reason.
case another_case
";

    fn marked(binary: &str, name: &str) -> Marked {
        Marked {
            binary: binary.to_owned(),
            name: name.to_owned(),
            exact: format!("t::{name}"),
        }
    }

    #[test]
    fn a_list_gives_every_line_the_reason_of_its_entry() {
        let list = AllowList::parse(LIST).unwrap();
        let reason = "Symbol#to_i is gone in Ruby 1.9.";
        assert_eq!(list.test("run_game", "the_first_script"), Some(reason));
        assert_eq!(list.test("lib", "foundation_tests::a_test"), Some(reason));
        assert_eq!(list.test("lib", "the_first_script"), None);
        assert!(list.case("a case of two colours") && list.case("another_case"));
        assert!(!list.case("the_first_script"));
        assert_eq!(list.cases[1].1, "Another reason.");
    }

    #[test]
    fn an_entry_without_a_reason_and_a_line_of_no_form_are_errors_that_name_the_line() {
        let error = AllowList::parse("# Reason.\ncase a\n\ntest run_game b\n").unwrap_err();
        assert!(
            error.contains("line 4") && error.contains("without a reason"),
            "{error}"
        );
        let error = AllowList::parse("#\ncase a\n").unwrap_err();
        assert!(
            error.contains("line 2") && error.contains("without a reason"),
            "{error}"
        );
        for line in ["skip run_game b", "test run_game", "case", "test a b c"] {
            let error = AllowList::parse(&format!("# Reason.\ncase a\n{line}\n")).unwrap_err();
            assert!(error.contains("line 3") && error.contains(line), "{error}");
        }
    }

    #[test]
    fn the_marker_finds_binary_and_name() {
        let found = |type_name| Marked::from_type_name(type_name, "vkrgss_binding_rgss3");
        assert_eq!(
            found("run_game::t::the_first_script::marker"),
            Some(Marked {
                binary: "run_game".to_owned(),
                name: "the_first_script".to_owned(),
                exact: "t::the_first_script".to_owned(),
            })
        );
        assert_eq!(
            found("vkrgss_binding_rgss3::foundation_tests::a_test::marker"),
            Some(Marked {
                binary: "lib".to_owned(),
                name: "foundation_tests::a_test".to_owned(),
                exact: "foundation_tests::a_test".to_owned(),
            })
        );
        // A test file of the package itself has no wrapping module.
        assert_eq!(
            found("strings::a_test::marker").map(|marked| (marked.name, marked.exact)),
            Some(("a_test".to_owned(), "a_test".to_owned()))
        );
        assert_eq!(found("marker"), None);
        assert_eq!(found("strings::marker"), None);
    }

    #[test]
    fn the_outcome_is_read_from_the_result_line_and_the_signal() {
        let line = |passed, failed| {
            let word = if failed == 0 { "ok" } else { "FAILED" };
            format!(
                "\nrunning 1 test\ntest t::a ... x\n\ntest result: {word}. {passed} passed; \
                 {failed} failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.01s\n"
            )
        };
        assert_eq!(Outcome::of(&line(0, 1), None), Outcome::Failed);
        assert_eq!(Outcome::of(&line(1, 0), None), Outcome::Passed);
        assert_eq!(Outcome::of(&line(0, 0), None), Outcome::NoTest);
        assert_eq!(Outcome::of(&line(1, 1), None), Outcome::NoTest);
        assert_eq!(Outcome::of("", None), Outcome::NoTest);
        assert_eq!(Outcome::of(&line(0, 1), Some(11)), Outcome::Signal(11));
    }

    #[test]
    fn a_listed_test_counts_as_passed_only_while_it_fails_alone() {
        let list = AllowList::parse(LIST).unwrap();
        let test = marked("run_game", "the_first_script");
        assert_eq!(
            judge(&list, &test, Outcome::Failed).as_deref(),
            Ok("Symbol#to_i is gone in Ruby 1.9.")
        );

        let passed = judge(&list, &test, Outcome::Passed).unwrap_err();
        assert!(
            passed.contains("`the_first_script` passes over Ruby 1.9.2"),
            "{passed}"
        );
        assert!(passed.contains("tests/ruby192.allow"), "{passed}");

        let no_test = judge(&list, &test, Outcome::NoTest).unwrap_err();
        assert!(no_test.contains("no test ran"), "{no_test}");
        let signal = judge(&list, &test, Outcome::Signal(11)).unwrap_err();
        assert!(signal.contains("ended by signal 11"), "{signal}");
        assert!(!signal.contains("no test ran") && !no_test.contains("signal"));
    }

    #[test]
    fn a_test_taken_off_the_list_is_not_found_there() {
        let list = AllowList::parse(LIST).unwrap();
        let without =
            AllowList::parse(&LIST.replace("test run_game the_first_script\n", "")).unwrap();
        assert!(list.test("run_game", "the_first_script").is_some());
        assert_eq!(without.test("run_game", "the_first_script"), None);
        // The decision is never asked for it; if it were, it would say so.
        let test = marked("run_game", "the_first_script");
        assert!(judge(&without, &test, Outcome::Failed).is_err());
    }

    /// The marker in a test that is not listed lets the body run, over either interpreter;
    /// over Ruby 1.8 the list is not even read.
    #[test]
    fn the_marker_is_inert_in_a_test_that_is_not_listed() {
        let mut ran = false;
        let mut body = || {
            crate::pending_over_ruby_1_9_2!();
            ran = true;
        };
        body();
        assert!(ran);
        assert!(rb::RUBY_19 || !pending_case("a case that no list is read for"));
    }
}
