//! Ruby's own test suites under AddressSanitizer and UndefinedBehaviorSanitizer.
//!
//! The suites of `tests/upstream.rs` only look at results. Here the interpreter, the regular
//! expression engine and the test executable are compiled again with both sanitizers, in a
//! directory of their own, and the same suites run: an invalid memory access or undefined
//! behaviour is reported even where it does not change a result. Reports listed in
//! `sanitized.allow` next to this file, each with its reason, are accepted.
//!
//! The run is slow and needs a clang with the sanitizer runtimes, so it is opt-in and not
//! part of the merge gate:
//!
//! ```text
//! cargo test -p vkrgss-ruby-sys --test sanitized -- --ignored upstream_suites_pass_under_the_sanitizers
//! ```
//!
//! Two more ignored tests run the same build under collector stress, where the collector
//! runs at every n-th allocation (`VKRGSS_GC_STRESS`): a value that is kept alive only by a
//! pointer into it is then collected, and the use of a freed string buffer is a report of
//! AddressSanitizer. `cargo xtask stress` runs them and sets the variables they read.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use common::Suites;

/// What the sanitized build adds to the flags of the build script.
///
/// `-fno-sanitize=function` takes one check out of the undefined-behaviour group: clang
/// compares the prototype of a called function with the type of the pointer it is called
/// through, and the interpreter's functions are K&R definitions without a prototype, called
/// through prototyped pointers and the other way round. In `gnu89` those calls are valid
/// when the arguments agree, and the check reports every one of them.
const SANITIZER_FLAGS: &[&str] = &[
    "-fsanitize=address,undefined",
    "-fno-sanitize=function",
    "-fno-omit-frame-pointer",
    "-g",
];

/// Leak detection is off because the interpreter frees nothing at exit, by design.
/// Use-after-return detection is off because its "fake stack" would hide stack values from
/// the conservative collector.
const ASAN_OPTIONS: &str = "detect_stack_use_after_return=0:detect_leaks=0";

const UBSAN_OPTIONS: &str = "print_stacktrace=1:halt_on_error=0";

/// How long one test process may run. The sanitized interpreter is several times slower.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(900);

/// How long one test process may run under collector stress.
const STRESSED_PROCESS_TIMEOUT: Duration = Duration::from_secs(2700);

/// The variable of the collector stress, which the executable built here reads in its
/// `Init_ext`, as `vkrgss-ruby-sys` reads it when it starts an interpreter.
const GC_STRESS_VARIABLE: &str = "VKRGSS_GC_STRESS";

/// What the stressed run leaves out, separated by white space: `<suite>/<file>` for a file
/// of a suite, `<suite>/<file>#<method>` for one test method, `sample` for `sample/test.rb`.
/// `cargo xtask stress` sets it from its skip list.
const GC_STRESS_SKIP_VARIABLE: &str = "VKRGSS_GC_STRESS_SKIP";

/// C definitions for the `Init_ext` of the sanitized executable: the function that the test
/// of a lost value calls. It keeps only the buffer pointer of a fresh string across
/// allocations, which is the fault the stressed runs are there to find.
const INIT_EXT_DEFINITIONS: &str = r#"
#include <stdlib.h>
void ruby_vkxp_gc_stress _((unsigned long));
__attribute__((noinline)) static char *vkrgss_fresh_buffer()
{
    return RSTRING(rb_str_new2("only the buffer pointer of this string is kept"))->ptr;
}
static VALUE vkrgss_lost_buffer(self)
    VALUE self;
{
    char *buffer = vkrgss_fresh_buffer();
    rb_eval_string("[1, 2, 3].map { |number| number.to_s }");
    return rb_str_new2(buffer);
}
"#;

/// C statements for the end of that `Init_ext`.
const INIT_EXT_STATEMENTS: &str = r#"
    if (getenv("VKRGSS_GC_STRESS")) ruby_vkxp_gc_stress(strtoul(getenv("VKRGSS_GC_STRESS"), 0, 10));
    rb_define_global_function("vkrgss_lost_buffer", vkrgss_lost_buffer, 0);
"#;

const ALLOW_LIST: &str = include_str!("sanitized.allow");

/// One entry of the allow-list: why the reports are harmless here, and what they contain.
#[derive(Debug, PartialEq, Eq)]
struct Allowed {
    reason: String,
    patterns: Vec<String>,
}

/// Parses the allow-list: entries separated by blank lines, each made of `#` lines that
/// give the reason and of pattern lines. A block of comment lines alone is not an entry.
fn parse_allow_list(text: &str) -> Vec<Allowed> {
    let mut entries = Vec::new();
    for block in text.split("\n\n") {
        let mut reason = Vec::new();
        let mut patterns = Vec::new();
        for line in block.lines().map(str::trim) {
            if let Some(comment) = line.strip_prefix('#') {
                reason.push(comment.trim());
            } else if !line.is_empty() {
                patterns.push(line.to_owned());
            }
        }
        if !patterns.is_empty() {
            entries.push(Allowed {
                reason: reason.join(" ").trim().to_owned(),
                patterns,
            });
        }
    }
    entries
}

/// Whether `line` matches `pattern`: the parts of the pattern between its `*` occur in the
/// line in that order.
fn matches(line: &str, pattern: &str) -> bool {
    let mut rest = line;
    pattern.split('*').all(|part| match rest.find(part) {
        Some(at) => {
            rest = &rest[at + part.len()..];
            true
        }
        None => false,
    })
}

/// The lines of sanitizer output that name a finding, with the location made relative to
/// `third_party/`: the `runtime error` lines of UndefinedBehaviorSanitizer and the summary
/// line of AddressSanitizer.
fn report_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| {
            line.contains(": runtime error: ") || line.starts_with("SUMMARY: AddressSanitizer:")
        })
        .map(|line| match line.split_once("third_party/") {
            Some((before, after)) => {
                let kept = before.rfind(' ').map_or("", |end| &before[..=end]);
                format!("{kept}{after}")
            }
            None => line.to_owned(),
        })
        .collect()
}

/// The report lines of every file the sanitizers wrote into `dir`, with how often each was
/// reported.
fn collect_reports(dir: &Path) -> BTreeMap<String, usize> {
    let mut reports = BTreeMap::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    for entry in entries {
        let path = entry.expect("directory entry").path();
        let bytes = fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        for line in report_lines(&String::from_utf8_lossy(&bytes)) {
            *reports.entry(line).or_default() += 1;
        }
    }
    reports
}

/// Builds the sanitized `ruby` in `scratch`: the libraries of
/// `common::build_instrumented_libraries` and the test executable of
/// `common::build_ruby_executable`, all compiled here.
fn build_sanitized_executable(scratch: &Path) -> PathBuf {
    let source = common::source_dir();
    let libraries = common::build_instrumented_libraries(scratch, SANITIZER_FLAGS, &[]);
    let top_dir = scratch.join("top");
    fs::create_dir_all(&top_dir).unwrap_or_else(|error| panic!("{}: {error}", top_dir.display()));
    let init_ext = common::write_init_ext_with(scratch, INIT_EXT_DEFINITIONS, INIT_EXT_STATEMENTS);
    let includes: Vec<&Path> = libraries.includes.iter().map(PathBuf::as_path).collect();
    let top_objects = common::compile_jobs(
        [
            source.join("main.c"),
            init_ext,
            source.join("ext/fcntl/fcntl.c"),
            source.join("ext/stringio/stringio.c"),
        ]
        .iter()
        .map(|file| common::job(scratch, &top_dir, file, &includes, SANITIZER_FLAGS))
        .collect(),
    );

    // As in the unsanitized executable: the objects first, so that their `Init_ext` wins, and
    // the engine as a library, so that only what the interpreter refers to is linked.
    let executable = scratch.join("ruby");
    common::run(
        common::instrumented_compiler(scratch, &[], SANITIZER_FLAGS)
            .arg("-rdynamic")
            .args(&top_objects)
            .args(libraries.link_arguments())
            .arg("-o")
            .arg(&executable),
    );
    executable
}

#[test]
fn allow_list_entries_state_their_reasons() {
    for entry in parse_allow_list(ALLOW_LIST) {
        assert!(
            !entry.reason.is_empty(),
            "allow-list entry without a reason: {:?}",
            entry.patterns
        );
    }
}

#[test]
fn allow_list_format() {
    let entries = parse_allow_list(
        "# About the file.\n\n# First reason,\n# continued.\na.c:1:2: one\nb.c: two\n\nc.c: bare\n",
    );
    assert_eq!(
        entries,
        [
            Allowed {
                reason: "First reason, continued.".to_owned(),
                patterns: vec!["a.c:1:2: one".to_owned(), "b.c: two".to_owned()],
            },
            Allowed {
                reason: String::new(),
                patterns: vec!["c.c: bare".to_owned()],
            },
        ]
    );
}

#[test]
fn patterns_match_their_parts_in_order() {
    let line = "onig/regexec.c:15:7: runtime error: load of misaligned address 0x12 for type 'int'";
    assert!(matches(
        line,
        "regexec.c:15:7: runtime error: load of misaligned"
    ));
    assert!(matches(
        line,
        "onig/*: runtime error: load of misaligned address"
    ));
    assert!(!matches(line, "regexec.c:15:8:"));
    assert!(!matches(line, "runtime error*onig/"));
}

#[test]
fn report_lines_are_found_and_made_relative() {
    let output = "\
/home/x/vkrgss/third_party/ruby-1.8.7/st.c:10:5: runtime error: left shift of 1 by 40 places
    #0 0x1 in f /home/x/vkrgss/third_party/ruby-1.8.7/st.c:10:5
==7==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x1
SUMMARY: AddressSanitizer: heap-buffer-overflow /home/x/vkrgss/third_party/ruby-1.8.7/re.c:3:1 in g
SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior /home/x/third_party/ruby-1.8.7/st.c:10:5
";
    assert_eq!(
        report_lines(output),
        [
            "ruby-1.8.7/st.c:10:5: runtime error: left shift of 1 by 40 places",
            "SUMMARY: AddressSanitizer: heap-buffer-overflow ruby-1.8.7/re.c:3:1 in g",
        ]
    );
}

/// The scratch directory of this file and the sanitized executable in it, built once for
/// all tests of the process.
fn built() -> &'static (PathBuf, PathBuf) {
    static BUILT: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    BUILT.get_or_init(|| {
        let scratch = common::scratch_dir("sanitized");
        let ruby = build_sanitized_executable(&scratch);
        (scratch, ruby)
    })
}

/// The sanitized executable with a directory `dir` for its runs, and the directory in which
/// each of its processes writes its reports to a file of its own: the suites compare what
/// child interpreters print, which the reports must stay out of.
fn sanitized_suites(dir: PathBuf, timeout: Duration) -> (Suites, PathBuf) {
    let reports = dir.join("reports");
    fs::create_dir_all(&reports).expect("report directory");
    let log_path = format!("log_path={}", reports.join("report").display());
    let suites = Suites {
        ruby: built().1.clone(),
        scratch: dir,
        env: vec![
            ("ASAN_OPTIONS", format!("{ASAN_OPTIONS}:{log_path}")),
            ("UBSAN_OPTIONS", format!("{UBSAN_OPTIONS}:{log_path}")),
        ],
        timeout,
    };
    (suites, reports)
}

/// Runs the suites without what `skip` names (see [`GC_STRESS_SKIP_VARIABLE`]) and fails
/// on a suite that does not pass and on a sanitizer report outside the allow-list.
fn run_suites(suites: &Suites, reports: &Path, skip: &[String]) {
    let skips = |suite: &str| skip.iter().any(|entry| entry.starts_with(suite));
    // The reports are read even when a suite fails: a failure is then most often a report.
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let core = suites.run_unit_suite_without("ruby", skip);
        let zlib = suites.run_unit_suite_without("zlib", skip);
        if !skips("sample") {
            suites.run_sample();
        }
        eprintln!("test/ruby: {core:?}\ntest/zlib: {zlib:?}");
        assert_eq!((core.failures, core.errors), (0, 0));
        assert_eq!((zlib.failures, zlib.errors), (0, 0));
        // The number of tests is known for a suite of which nothing is left out.
        assert!(skips("ruby/") || core.tests == common::CORE_TESTS);
        assert!(skips("zlib/") || zlib.tests == common::ZLIB_TESTS);
    }));

    let allowed = parse_allow_list(ALLOW_LIST);
    let (accepted, refused): (Vec<_>, Vec<_>) =
        collect_reports(reports).into_iter().partition(|(line, _)| {
            allowed
                .iter()
                .flat_map(|entry| &entry.patterns)
                .any(|pattern| matches(line, pattern))
        });
    eprintln!(
        "{} distinct report(s) accepted by the allow-list",
        accepted.len()
    );
    let refused: Vec<String> = refused
        .iter()
        .map(|(line, count)| format!("{count:>7}x {line}"))
        .collect();
    assert!(
        refused.is_empty(),
        "{} sanitizer report(s) outside the allow-list (the full reports are in {}):\n{}",
        refused.len(),
        reports.display(),
        refused.join("\n")
    );
    assert!(
        outcome.is_ok(),
        "the suites did not pass under the sanitizers"
    );
}

#[test]
#[ignore = "opt-in: builds the interpreter with sanitizers and runs the upstream suites"]
fn upstream_suites_pass_under_the_sanitizers() {
    let (mut suites, reports) = sanitized_suites(built().0.clone(), PROCESS_TIMEOUT);
    // Whatever the caller has set: this run is the one without collector stress.
    suites.env.push((GC_STRESS_VARIABLE, String::new()));
    run_suites(&suites, &reports, &[]);
}

#[test]
#[ignore = "opt-in and slow: run by `cargo xtask stress`, which sets VKRGSS_GC_STRESS"]
fn upstream_suites_pass_under_collector_stress() {
    let every = std::env::var(GC_STRESS_VARIABLE).unwrap_or_default();
    assert!(
        every.parse::<u64>().is_ok_and(|every| every >= 1),
        "{GC_STRESS_VARIABLE} is `{every}`; this test needs a number of 1 or more, which \
         `cargo xtask stress` sets"
    );
    let skip: Vec<String> = std::env::var(GC_STRESS_SKIP_VARIABLE)
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    // The test processes and the interpreters they start inherit the variable.
    let (suites, reports) = sanitized_suites(built().0.join("stress"), STRESSED_PROCESS_TIMEOUT);
    run_suites(&suites, &reports, &skip);
}

#[test]
#[ignore = "opt-in: builds the interpreter with sanitizers"]
fn a_lost_value_is_reported_under_collector_stress() {
    let (mut suites, reports) = sanitized_suites(built().0.join("lost"), PROCESS_TIMEOUT);
    suites.env.push((GC_STRESS_VARIABLE, "1".to_owned()));
    let finished = suites.run_ruby("lost", &["-e", "p vkrgss_lost_buffer"]);
    let found = collect_reports(&reports);
    assert!(
        !finished.success
            && found
                .keys()
                .any(|line| line.contains("AddressSanitizer: heap-use-after-free")),
        "success {}, reports {found:?}\n{}",
        finished.success,
        finished.output
    );

    // And the same code passes where the collector runs only when it has to.
    let (mut suites, reports) = sanitized_suites(built().0.join("kept"), PROCESS_TIMEOUT);
    suites.env.push((GC_STRESS_VARIABLE, String::new()));
    let finished = suites.run_ruby("kept", &["-e", "p vkrgss_lost_buffer"]);
    assert!(
        finished.success && collect_reports(&reports).is_empty(),
        "{}",
        finished.output
    );
}
