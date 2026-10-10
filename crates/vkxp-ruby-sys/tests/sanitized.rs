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
//! cargo test -p vkxp-ruby-sys --test sanitized -- --ignored
//! ```

mod common;

use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use common::Suites;

/// The compiler of the sanitized build. The development machine has the sanitizer runtimes
/// of clang only, so this is not the compiler the build script used.
const COMPILER: &str = "clang";

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

/// One C file to compile: the command and the object file it writes.
struct Job {
    command: Command,
    object: PathBuf,
}

/// The sanitized compiler with the flags of the build script and the include paths given.
fn compiler(scratch: &Path, includes: &[&Path]) -> Command {
    let mut command = Command::new(COMPILER);
    command
        .args(
            env!("VKXP_RUBY_CFLAGS")
                .split(' ')
                .chain(env!("VKXP_RUBY_CLANG_CFLAGS").split(' '))
                .filter(|arg| !arg.is_empty()),
        )
        .args(SANITIZER_FLAGS)
        .env("TMPDIR", scratch);
    for include in includes {
        command.arg("-I").arg(include);
    }
    command
}

/// The job that compiles `source` into the directory `objects`.
fn job(scratch: &Path, objects: &Path, source: &Path, includes: &[&Path], more: &[&str]) -> Job {
    let stem = source.file_stem().expect("a file name");
    let object = objects.join(stem).with_extension("o");
    let mut command = compiler(scratch, includes);
    command
        .args(more)
        .arg("-c")
        .arg(source)
        .arg("-o")
        .arg(&object);
    Job { command, object }
}

/// Runs the jobs, several at a time, and returns the object files in the order of the jobs.
fn compile(jobs: Vec<Job>) -> Vec<PathBuf> {
    let objects = jobs.iter().map(|job| job.object.clone()).collect();
    let queue = Mutex::new(VecDeque::from(jobs));
    let workers = thread::available_parallelism().map_or(4, |count| count.get());
    thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let Some(mut job) = queue.lock().unwrap().pop_front() else {
                        break;
                    };
                    common::run(&mut job.command);
                }
            });
        }
    });
    objects
}

/// Puts object files into a static library in `scratch`.
fn archive(scratch: &Path, name: &str, objects: &[PathBuf]) -> PathBuf {
    let library = scratch.join(name);
    common::run(Command::new("ar").arg("rcs").arg(&library).args(objects));
    library
}

/// Builds the sanitized `ruby` in `scratch`: the files and include paths of the build
/// script, and the test executable of `common::build_ruby_executable`, all compiled here.
fn build_sanitized_executable(scratch: &Path) -> PathBuf {
    let source = common::source_dir();
    let config = Path::new(env!("VKXP_RUBY_CONFIG_DIR"));
    let onig = Path::new(env!("VKXP_RUBY_ONIG_DIR"));
    let onig_config = config.join("oniguruma");
    let directory = |name: &str| {
        let dir = scratch.join(name);
        fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
        dir
    };
    let (core_dir, onig_dir, top_dir) = (directory("core"), directory("onig"), directory("top"));

    let core_includes = [
        config,
        &source,
        Path::new(env!("VKXP_RUBY_GENERATED_DIR")),
        Path::new(env!("VKXP_RUBY_COMPAT_DIR")),
        onig,
    ];
    let zlib_includes = [
        config,
        &source,
        Path::new(env!("VKXP_RUBY_ZLIB_INCLUDE_DIR")),
    ];
    let init_ext = common::write_init_ext(scratch);

    let mut core_jobs: Vec<Job> = env!("VKXP_RUBY_CORE_SOURCES")
        .split(' ')
        .map(|name| source.join(format!("{name}.c")))
        .map(|file| job(scratch, &core_dir, &file, &core_includes, &[]))
        .collect();
    core_jobs.push(job(
        scratch,
        &core_dir,
        &source.join("ext/zlib/zlib.c"),
        &zlib_includes,
        &["-DHAVE_ZLIB_H", "-DOS_CODE=OS_UNIX"],
    ));
    let onig_jobs: Vec<Job> = env!("VKXP_RUBY_ONIG_SOURCES")
        .split(' ')
        .map(|name| onig.join(format!("{name}.c")))
        .map(|file| job(scratch, &onig_dir, &file, &[&onig_config, onig], &[]))
        .collect();
    let top_jobs: Vec<Job> = [
        source.join("main.c"),
        init_ext,
        source.join("ext/fcntl/fcntl.c"),
        source.join("ext/stringio/stringio.c"),
    ]
    .iter()
    .map(|file| job(scratch, &top_dir, file, &core_includes, &[]))
    .collect();

    let (core_count, onig_count) = (core_jobs.len(), onig_jobs.len());
    let mut objects = compile(
        core_jobs
            .into_iter()
            .chain(onig_jobs)
            .chain(top_jobs)
            .collect(),
    );
    let top_objects = objects.split_off(core_count + onig_count);
    let onig_objects = objects.split_off(core_count);

    // As in the unsanitized executable: the objects first, so that their `Init_ext` wins, and
    // the engine as a library, so that only what the interpreter refers to is linked.
    let executable = scratch.join("ruby");
    common::run(
        compiler(scratch, &[])
            .arg("-rdynamic")
            .args(&top_objects)
            .arg(archive(scratch, "libruby-sanitized.a", &objects))
            .arg(archive(scratch, "libonig-sanitized.a", &onig_objects))
            .arg(Path::new(env!("VKXP_RUBY_ZLIB_LIB_DIR")).join("libz.a"))
            .args(["-lcrypt", "-lm"])
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
/home/x/vkxp/third_party/ruby-1.8.7/st.c:10:5: runtime error: left shift of 1 by 40 places
    #0 0x1 in f /home/x/vkxp/third_party/ruby-1.8.7/st.c:10:5
==7==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x1
SUMMARY: AddressSanitizer: heap-buffer-overflow /home/x/vkxp/third_party/ruby-1.8.7/re.c:3:1 in g
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

#[test]
#[ignore = "opt-in: builds the interpreter with sanitizers and runs the upstream suites"]
fn upstream_suites_pass_under_the_sanitizers() {
    let scratch = common::scratch_dir("sanitized");
    let reports = scratch.join("reports");
    fs::create_dir_all(&reports).expect("report directory");
    // Each process writes its reports to a file of its own here; the suites compare what
    // child interpreters print, which the reports must stay out of.
    let log_path = format!("log_path={}", reports.join("report").display());
    let suites = Suites {
        ruby: build_sanitized_executable(&scratch),
        scratch,
        env: vec![
            ("ASAN_OPTIONS", format!("{ASAN_OPTIONS}:{log_path}")),
            ("UBSAN_OPTIONS", format!("{UBSAN_OPTIONS}:{log_path}")),
        ],
        timeout: PROCESS_TIMEOUT,
    };

    // The reports are read even when a suite fails: a failure is then most often a report.
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let core = suites.run_unit_suite("ruby");
        let zlib = suites.run_unit_suite("zlib");
        suites.run_sample();
        eprintln!("test/ruby: {core:?}\ntest/zlib: {zlib:?}");
        assert_eq!(
            (core.tests, core.failures, core.errors),
            (common::CORE_TESTS, 0, 0)
        );
        assert_eq!(
            (zlib.tests, zlib.failures, zlib.errors),
            (common::ZLIB_TESTS, 0, 0)
        );
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
    assert!(
        outcome.is_ok(),
        "the suites did not pass under the sanitizers"
    );
}
