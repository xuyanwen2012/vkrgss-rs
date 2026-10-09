//! Ruby's own test suites, run against the interpreter exactly as it was built for vkxp.
//!
//! A wrong compiler flag does not fail the build: without `-fwrapv`, for one, the interpreter
//! compiles and then computes `2**64` as 0. Only these suites show it, so they are neither
//! optional nor filtered. A real `ruby` executable is linked from the archives of the build
//! script, because the suites start child interpreters and expect a normal process.

mod common;

use std::collections::VecDeque;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// What `ruby -v` must print.
const VERSION_LINE: &str = "ruby 1.8.7 (2013-06-27 patchlevel 374) [x86_64-linux]";

/// The number of tests in `test/ruby` of the release.
const CORE_TESTS: u64 = 625;

/// The number of tests in `test/zlib` of the release.
const ZLIB_TESTS: u64 = 45;

/// The number of checks `sample/test.rb` of the release makes.
const SAMPLE_CHECKS: usize = 912;

/// How long one test process may run. The slowest file takes a few seconds.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(120);

/// The scratch directory of this test binary and the executable built in it.
struct Built {
    scratch: PathBuf,
    ruby: PathBuf,
}

fn built() -> &'static Built {
    static BUILT: OnceLock<Built> = OnceLock::new();
    BUILT.get_or_init(|| {
        let scratch = common::scratch_dir("upstream");
        let ruby = common::build_ruby_executable(&scratch);
        Built { scratch, ruby }
    })
}

/// The output and exit status of one finished test process.
struct Finished {
    success: bool,
    output: String,
}

/// Runs the executable in a working directory of its own, the way `make test-all` sets it
/// up: `./ruby` and `./miniruby` in the current directory, `RUBY` pointing at the former, a
/// scratch `HOME` and `TMPDIR`, and a load path that ends at the vendored `lib` and `ext`.
fn run_ruby(name: &str, args: &[&str]) -> Finished {
    let built = built();
    let source = common::source_dir();
    let dir = built.scratch.join("run").join(name);
    let home = dir.join("home");
    let tmp = dir.join("tmp");
    fs::create_dir_all(&home).expect("scratch home");
    fs::create_dir_all(&tmp).expect("scratch tmp");
    let ruby = dir.join("ruby");
    symlink(&built.ruby, &ruby).expect("link to the executable");
    symlink(&built.ruby, dir.join("miniruby")).expect("link to the executable");

    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut search = vec![dir.clone()];
    search.extend(std::env::split_paths(&path));
    let log = fs::File::create(dir.join("output.log")).expect("log file");
    let mut child = Command::new(&ruby)
        .args(args)
        .current_dir(&dir)
        .env("HOME", &home)
        .env("TMPDIR", &tmp)
        .env("RUBY", &ruby)
        .env(
            "PATH",
            std::env::join_paths(search).expect("a joinable PATH"),
        )
        .env(
            "RUBYLIB",
            format!(
                "{}:{}:-",
                source.join("lib").display(),
                source.join("ext").display()
            ),
        )
        .env_remove("RUBYOPT")
        .stdin(Stdio::null())
        .stdout(log.try_clone().expect("second handle to the log"))
        .stderr(log)
        .spawn()
        .unwrap_or_else(|error| panic!("could not start {}: {error}", ruby.display()));

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for the test process") {
            break Some(status);
        }
        if started.elapsed() > PROCESS_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(5));
    };
    let mut output =
        String::from_utf8_lossy(&fs::read(dir.join("output.log")).expect("the log")).into_owned();
    if status.is_none() {
        output.push_str("\n(killed: the process ran into the time limit)\n");
    }
    Finished {
        success: status.is_some_and(|status| status.success()),
        output,
    }
}

/// The totals `test/unit` prints at the end of a run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Totals {
    tests: u64,
    assertions: u64,
    failures: u64,
    errors: u64,
}

/// Finds the last `N tests, N assertions, N failures, N errors` line.
fn parse_totals(output: &str) -> Option<Totals> {
    output.lines().rev().find_map(|line| {
        let mut numbers = [0_u64; 4];
        let mut parts = line.split(", ");
        for (number, label) in numbers
            .iter_mut()
            .zip(["tests", "assertions", "failures", "errors"])
        {
            let (count, name) = parts.next()?.split_once(' ')?;
            if name != label {
                return None;
            }
            *number = count.parse().ok()?;
        }
        Some(Totals {
            tests: numbers[0],
            assertions: numbers[1],
            failures: numbers[2],
            errors: numbers[3],
        })
    })
}

/// Runs one file of a `test/unit` suite through `test/runner.rb`.
fn run_unit_file(suite: &str, file: &str) -> Result<Totals, String> {
    let source = common::source_dir();
    let runner = source.join("test/runner.rb");
    let basedir = format!("--basedir={}", source.join("test").display());
    let target = format!("{suite}/{file}");
    let name = format!("{suite}-{}", file.trim_end_matches(".rb"));
    let finished = run_ruby(
        &name,
        &[
            "-rpurelib.rb",
            runner.to_str().expect("UTF-8 path"),
            &basedir,
            "--runner=console",
            &target,
        ],
    );
    match parse_totals(&finished.output) {
        Some(totals) if finished.success && totals.failures == 0 && totals.errors == 0 => {
            Ok(totals)
        }
        _ => Err(format!("--- {target}\n{}", finished.output)),
    }
}

/// The `test_*.rb` files of a suite directory, sorted.
fn test_files(suite: &str) -> Vec<String> {
    let dir = common::source_dir().join("test").join(suite);
    let mut files: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.expect("directory entry").file_name())
        .map(|name| name.into_string().expect("UTF-8 file name"))
        .filter(|name| name.starts_with("test_") && name.ends_with(".rb"))
        .collect();
    files.sort();
    files
}

/// Runs every file of a suite as its own process, several at a time, and sums the totals.
fn run_unit_suite(suite: &str) -> Totals {
    let files = test_files(suite);
    assert!(!files.is_empty(), "no test files in test/{suite}");
    let queue = Mutex::new(VecDeque::from(files));
    let results: Mutex<Vec<Result<Totals, String>>> = Mutex::new(Vec::new());
    let workers = thread::available_parallelism().map_or(4, |count| count.get().min(8));
    thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let Some(file) = queue.lock().unwrap().pop_front() else {
                        break;
                    };
                    let result = run_unit_file(suite, &file);
                    results.lock().unwrap().push(result);
                }
            });
        }
    });

    let mut sum = Totals::default();
    let mut problems = Vec::new();
    for result in results.into_inner().unwrap() {
        match result {
            Ok(totals) => {
                sum.tests += totals.tests;
                sum.assertions += totals.assertions;
                sum.failures += totals.failures;
                sum.errors += totals.errors;
            }
            Err(problem) => problems.push(problem),
        }
    }
    assert!(
        problems.is_empty(),
        "{} file(s) of test/{suite} did not pass:\n{}",
        problems.len(),
        problems.join("\n")
    );
    sum
}

#[test]
fn upstream_executable_reports_its_version() {
    let output = common::run(Command::new(&built().ruby).arg("-v"));
    assert_eq!(output.trim_end(), VERSION_LINE);
}

#[test]
fn upstream_core_suite_passes() {
    let totals = run_unit_suite("ruby");
    eprintln!("test/ruby: {totals:?}");
    assert_eq!(
        (totals.tests, totals.failures, totals.errors),
        (CORE_TESTS, 0, 0),
        "test/ruby reported {totals:?}"
    );
}

#[test]
fn upstream_zlib_suite_passes() {
    let totals = run_unit_suite("zlib");
    eprintln!("test/zlib: {totals:?}");
    assert_eq!(
        (totals.tests, totals.failures, totals.errors),
        (ZLIB_TESTS, 0, 0),
        "test/zlib reported {totals:?}"
    );
}

#[test]
fn upstream_sample_test_passes() {
    let source = common::source_dir();
    let lib = format!("-I{}", source.join("lib").display());
    let script = source.join("sample/test.rb");
    let finished = run_ruby("sample", &[&lib, script.to_str().expect("UTF-8 path")]);
    let lines = |prefix: &str| -> usize {
        finished
            .output
            .lines()
            .filter(|line| line.starts_with(prefix))
            .count()
    };
    let (passed, failed, ended) = (lines("ok "), lines("not ok"), lines("end of test"));
    assert!(
        finished.success && failed == 0 && ended == 1 && passed == SAMPLE_CHECKS,
        "sample/test.rb: {passed} ok, {failed} not ok, {ended} `end of test`, success {}\n{}",
        finished.success,
        finished.output
    );
}

/// The suites must not write into the vendored tree: it is hashed by the manifest test.
#[test]
fn upstream_scratch_directory_is_outside_the_vendored_tree() {
    let scratch: &Path = &built().scratch;
    assert!(!scratch.starts_with(common::source_dir()));
}
