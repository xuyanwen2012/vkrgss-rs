//! Ruby's own test suites under AddressSanitizer and UndefinedBehaviorSanitizer.
//!
//! The suites of `tests/upstream.rs` only look at results. Here the interpreter, its
//! encodings, `Zlib` and the test executable are compiled again with both sanitizers, in a
//! directory of their own, and the same suites run with the same list of what is left out
//! (`upstream.allow`): an invalid memory access or undefined behaviour is reported even
//! where it does not change a result. Reports listed in `sanitized.allow` next to this file,
//! each with its reason, are accepted.
//!
//! The run is slow and needs a clang with the sanitizer runtimes, so it is opt-in and not
//! part of the merge gate:
//!
//! ```text
//! cargo test -p vkrgss-ruby192-sys --test sanitized -- --ignored
//! ```
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use common::instrumented;
use common::suites::{self, Suites};

/// What the sanitized build adds to the flags of the build script.
///
/// `-fno-sanitize=function` takes one check out of the undefined-behaviour group: clang
/// compares the prototype of a called function with the type of the pointer it is called
/// through, and the interpreter calls every method written in C through a pointer of the
/// type `VALUE (*)(ANYARGS)`, which has no prototype. Those calls are valid in `gnu89` when
/// the arguments agree, and the check reports every one of them.
const SANITIZER_FLAGS: &[&str] = &[
    "-fsanitize=address,undefined",
    "-fno-sanitize=function",
    "-fno-omit-frame-pointer",
    "-g",
];

/// The options of AddressSanitizer. Leak detection is off because the interpreter frees
/// little at exit, by design. Use-after-return detection is off because its "fake stack"
/// would hide stack values from the conservative collector. The sanitizer's own alternate
/// signal stacks are off because the interpreter gives every thread one of its own
/// (`rb_register_sigaltstack`, a buffer of `malloc`), and at the end of a thread the
/// sanitizer unmaps whichever is set, which fails for that buffer and ends the process.
/// The sanitizer leaves SIGSEGV and the limit of core files to the interpreter, whose
/// suites test both.
const ASAN_OPTIONS: &str = "detect_stack_use_after_return=0:detect_leaks=0:\
                            use_sigaltstack=0:handle_segv=0:disable_coredump=0";

/// No stack trace: printing one starts `llvm-symbolizer` as a child of the interpreter, and a
/// test that waits for any child (`Process.wait` in `test_process.rb`) waits for it forever.
/// The line of a report names its place without it.
const UBSAN_OPTIONS: &str = "print_stacktrace=0:halt_on_error=0";

/// C definitions for the `Init_ext` of the sanitized executable: the options above as the
/// defaults of the sanitizers, for the interpreters that the suites start with an emptied
/// environment. Without them leak detection would run at their exit, and its helper process
/// is a child that a test of `Process.wait` sees.
fn init_ext_definitions() -> String {
    format!(
        "const char *__asan_default_options(void) {{ return \"{ASAN_OPTIONS}\"; }}\n\
         const char *__ubsan_default_options(void) {{ return \"{UBSAN_OPTIONS}\"; }}\n"
    )
}

/// How long one test process may run. The sanitized interpreter is several times slower.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(1200);

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
    let line = "ruby-1.9.2/regexec.c:15:7: runtime error: load of misaligned address 0x12";
    assert!(matches(
        line,
        "regexec.c:15:7: runtime error: load of misaligned"
    ));
    assert!(matches(
        line,
        "ruby-1.9.2/reg*: runtime error: load of misaligned address"
    ));
    assert!(!matches(line, "regexec.c:15:8:"));
    assert!(!matches(line, "runtime error*ruby-1.9.2/"));
}

#[test]
fn report_lines_are_found_and_made_relative() {
    let output = "\
/home/x/vkrgss/third_party/ruby-1.9.2/st.c:10:5: runtime error: left shift of 1 by 40 places
    #0 0x1 in f /home/x/vkrgss/third_party/ruby-1.9.2/st.c:10:5
==7==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x1
SUMMARY: AddressSanitizer: heap-buffer-overflow /home/x/vkrgss/third_party/ruby-1.9.2/re.c:3:1 in g
SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior /home/x/third_party/ruby-1.9.2/st.c:10:5
";
    assert_eq!(
        report_lines(output),
        [
            "ruby-1.9.2/st.c:10:5: runtime error: left shift of 1 by 40 places",
            "SUMMARY: AddressSanitizer: heap-buffer-overflow ruby-1.9.2/re.c:3:1 in g",
        ]
    );
}

/// The scratch directory of this file and the sanitized executable in it, built once for
/// all tests of the process.
fn built() -> &'static (PathBuf, PathBuf) {
    static BUILT: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    BUILT.get_or_init(|| {
        let scratch = common::scratch_dir("sanitized");
        let ruby = instrumented::build_instrumented_executable(
            &scratch,
            SANITIZER_FLAGS,
            &[],
            &init_ext_definitions(),
            "",
        );
        (scratch, ruby)
    })
}

/// The suites of the sanitized executable, and the directory in which each of their
/// processes writes its reports to a file of its own: the suites compare what child
/// interpreters print, which the reports must stay out of.
fn sanitized_suites() -> (Suites, PathBuf) {
    let (scratch, ruby) = built();
    let reports = scratch.join("reports");
    fs::create_dir_all(&reports).expect("report directory");
    let log_path = format!("log_path={}", reports.join("report").display());
    let mut suites = Suites::with_executable(scratch.clone(), ruby.clone(), PROCESS_TIMEOUT, false);
    suites.env = vec![
        ("ASAN_OPTIONS", format!("{ASAN_OPTIONS}:{log_path}")),
        ("UBSAN_OPTIONS", format!("{UBSAN_OPTIONS}:{log_path}")),
    ];
    (suites, reports)
}

/// The suites of `tests/upstream.rs`, with its list of what is left out, and every sanitizer
/// report checked against the allow-list.
#[test]
#[ignore = "opt-in: builds the interpreter with sanitizers and runs the upstream suites"]
fn upstream_suites_pass_under_the_sanitizers() {
    let (suites, reports) = sanitized_suites();
    let skip = suites::allowed_tests();
    // The reports are read even when a suite fails: a failure is then most often a report.
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut problems = Vec::new();
        for file in suites::test_files("bootstraptest") {
            if !skip.contains(&format!("bootstraptest/{file}"))
                && let Err(problem) = suites.run_bootstrap_file(&file)
            {
                problems.push(problem);
            }
        }
        let files = suites::selected_core_files(&skip);
        let totals = catch_unwind(AssertUnwindSafe(|| suites.run_unit_files(&files, &skip)));
        eprintln!("test/ruby: {} files, {totals:?}", files.len());
        assert!(
            problems.is_empty(),
            "{} file(s) of bootstraptest did not pass:\n{}",
            problems.len(),
            problems.join("\n")
        );
        if let Err(payload) = totals {
            std::panic::resume_unwind(payload);
        }
    }));

    let allowed = parse_allow_list(ALLOW_LIST);
    let (accepted, refused): (Vec<_>, Vec<_>) =
        collect_reports(&reports)
            .into_iter()
            .partition(|(line, _)| {
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
    if let Err(payload) = outcome {
        std::panic::resume_unwind(payload);
    }
}
