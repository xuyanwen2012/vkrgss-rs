//! The fuzz targets of Ruby 1.9.2: `Marshal.load`, the parser and the regular expression
//! engine fed with arbitrary bytes under libFuzzer, AddressSanitizer and
//! UndefinedBehaviorSanitizer.
//!
//! A target is one C file under `fuzz/` with its seeds under `fuzz/seeds/<file>/`. The
//! ignored test below builds the interpreter as the sanitized run does, with libFuzzer's
//! coverage instrumentation added, links the target and runs it. It is run by
//! `cargo xtask fuzz <target> [--seconds N]` with the targets `marshal192`, `parse192` and
//! `regex192`, which sets the variables it reads; it is not part of the merge gate. The other
//! tests run in the normal test step.
#![cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]

mod common;

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Instant;

use common::{eval_inspect, instrumented};
use vkrgss_ruby192_sys::{Interpreter, Options};

/// One target: its name in `cargo xtask fuzz`, which is also the name of its directory below
/// cargo's directory for test files, the name of its C file and of its seed directory, the
/// largest input in bytes, and the seconds one input may take.
struct Target {
    name: &'static str,
    file: &'static str,
    max_len: u32,
    timeout: u32,
}

/// The targets. An input is limited in size because nesting recurses in `Marshal.load` and
/// in the regular expression engine, and a deep enough input exhausts the machine stack,
/// which this run does not count as a finding; see `docs/development.md`.
const TARGETS: [Target; 3] = [
    Target {
        name: "marshal192",
        file: "marshal",
        max_len: 4096,
        timeout: 10,
    },
    Target {
        name: "parse192",
        file: "parse",
        max_len: 4096,
        timeout: 10,
    },
    Target {
        name: "regex192",
        file: "regex",
        max_len: 512,
        timeout: 3,
    },
];

/// The variable that names the target to run.
const TARGET_VARIABLE: &str = "VKRGSS_FUZZ_TARGET";

/// The variable that gives the seconds to run for.
const SECONDS_VARIABLE: &str = "VKRGSS_FUZZ_SECONDS";

/// The seconds a target runs for when the variable does not say.
const DEFAULT_SECONDS: u64 = 60;

/// What the fuzz build adds to the flags of the build script: the sanitizers as in the
/// sanitized run (see `tests/sanitized.rs` for `-fno-sanitize=function`) and the coverage
/// instrumentation of libFuzzer. A target cannot filter reports through an allow-list, so
/// undefined behaviour ends the run, and the check whose reports the allow-list of the
/// sanitized run accepts for the interpreter is off: a null pointer given to `memcpy` with
/// a length of zero.
const FUZZ_FLAGS: &[&str] = &[
    "-fsanitize=address,undefined,fuzzer-no-link",
    "-fno-sanitize=function,nonnull-attribute",
    "-fno-sanitize-recover=undefined",
    "-fno-omit-frame-pointer",
    "-g",
];

/// Added for the files of the engine: it reads the operands of a compiled pattern at
/// whatever offset they have, on purpose (see `tests/sanitized.allow`).
const FUZZ_ENGINE_FLAGS: &[&str] = &["-fno-sanitize=alignment"];

fn fuzz_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz")
}

fn fresh_dir(dir: &Path) {
    if dir.exists() {
        fs::remove_dir_all(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
    }
    fs::create_dir_all(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
}

/// Builds the executable of `target` in `scratch`.
fn build_target(scratch: &Path, target: &Target) -> PathBuf {
    let libraries =
        instrumented::build_instrumented_libraries(scratch, FUZZ_FLAGS, FUZZ_ENGINE_FLAGS);
    let source = fuzz_dir().join(target.file).with_extension("c");
    let objects = instrumented::compile_jobs(vec![instrumented::job(
        scratch, scratch, &source, FUZZ_FLAGS,
    )]);
    let executable = scratch.join(format!("fuzz-{}", target.name));
    common::run(
        instrumented::instrumented_compiler(scratch, FUZZ_FLAGS)
            .arg("-fsanitize=fuzzer")
            .args(&objects)
            .args(libraries.link_arguments())
            .arg("-o")
            .arg(&executable),
    );
    executable
}

/// The status with which libFuzzer ends when an input ran into the time limit, and when
/// the process ran into the memory limit. Both are its defaults.
const TIMEOUT_STATUS: i32 = 70;
const OUT_OF_MEMORY_STATUS: i32 = 71;

/// The arguments of libFuzzer for `target`: the corpus that grows, then the seeds, which
/// are only read.
fn fuzzer_arguments(target: &Target, dir: &Path, seconds: u64) -> Vec<String> {
    vec![
        dir.join("corpus").display().to_string(),
        fuzz_dir()
            .join("seeds")
            .join(target.file)
            .display()
            .to_string(),
        format!("-max_total_time={seconds}"),
        format!("-max_len={}", target.max_len),
        format!("-timeout={}", target.timeout),
        format!("-timeout_exitcode={TIMEOUT_STATUS}"),
        "-print_final_stats=1".to_owned(),
        format!("-artifact_prefix={}/", dir.join("artifacts").display()),
    ]
}

/// How one process of a target ended.
#[derive(Debug, PartialEq, Eq)]
enum Ended {
    /// The time was used up.
    Clean,
    /// An input ran into the time limit, which backtracking does by its nature, or the
    /// process into the memory limit, which the symbol table does because it only grows.
    /// Neither is a finding: the input is put aside and the run goes on in a new process.
    Limit(&'static str),
    /// A sanitizer report or a crash.
    Finding,
}

fn ended(status: Option<i32>) -> Ended {
    match status {
        Some(0) => Ended::Clean,
        Some(TIMEOUT_STATUS) => Ended::Limit("time"),
        Some(OUT_OF_MEMORY_STATUS) => Ended::Limit("memory"),
        _ => Ended::Finding,
    }
}

/// The number of inputs a process says it ran, from the statistics it prints at its end.
fn executed_units(line: &str) -> Option<u64> {
    line.strip_prefix("stat::number_of_executed_units:")?
        .trim()
        .parse()
        .ok()
}

/// Runs one process of the target for at most `seconds`, passes its output through and
/// returns its status and the number of inputs it ran.
fn run_once(executable: &Path, target: &Target, dir: &Path, seconds: u64) -> (Option<i32>, u64) {
    let mut child = Command::new(executable)
        .args(fuzzer_arguments(target, dir, seconds))
        .current_dir(dir)
        .env("TMPDIR", dir.join("tmp"))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("could not start {}: {error}", executable.display()));
    let mut executed = 0;
    let output = BufReader::new(child.stderr.take().expect("the piped error stream"));
    for line in output.split(b'\n') {
        let line = String::from_utf8_lossy(&line.expect("output of the target")).into_owned();
        eprintln!("{line}");
        executed += executed_units(&line).unwrap_or(0);
    }
    (child.wait().expect("the target ends").code(), executed)
}

#[test]
#[ignore = "opt-in: run by `cargo xtask fuzz <target>`, which sets VKRGSS_FUZZ_TARGET"]
fn the_chosen_target_runs_without_a_finding() {
    let name = std::env::var(TARGET_VARIABLE).unwrap_or_default();
    let target = TARGETS
        .iter()
        .find(|target| target.name == name)
        .unwrap_or_else(|| {
            panic!("{TARGET_VARIABLE} is `{name}`; `cargo xtask fuzz <target>` sets it")
        });
    let seconds: u64 = match std::env::var(SECONDS_VARIABLE) {
        Ok(text) => text
            .parse()
            .unwrap_or_else(|_| panic!("{SECONDS_VARIABLE} is `{text}`, not a number")),
        Err(_) => DEFAULT_SECONDS,
    };

    // The corpus and the inputs put aside stay from run to run; the build does not.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("fuzz")
        .join(target.name);
    for kept in ["corpus", "artifacts"] {
        fs::create_dir_all(dir.join(kept)).expect("directory of the target");
    }
    fresh_dir(&dir.join("build"));
    fresh_dir(&dir.join("tmp"));
    let executable = build_target(&dir.join("build"), target);

    let started = Instant::now();
    let (mut executed, mut time_limits, mut memory_limits) = (0, 0, 0);
    loop {
        let left = seconds.saturating_sub(started.elapsed().as_secs());
        if left == 0 {
            break;
        }
        let (status, units) = run_once(&executable, target, &dir, left);
        executed += units;
        match ended(status) {
            Ended::Clean => break,
            Ended::Limit("time") => time_limits += 1,
            Ended::Limit(_) => memory_limits += 1,
            Ended::Finding => panic!(
                "the target `{}` ended with {status:?} after {executed} inputs: the report \
                 is above and the input is in {}",
                target.name,
                dir.join("artifacts").display()
            ),
        }
    }
    eprintln!(
        "fuzz: `{}` ran {executed} inputs in {} s without a finding; {time_limits} input(s) \
         ran into the time limit of {} s and {memory_limits} process(es) into the memory \
         limit, which are put aside in {}",
        target.name,
        started.elapsed().as_secs(),
        target.timeout,
        dir.join("artifacts").display()
    );
}

fn eval(source: String) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |ruby| eval_inspect(ruby, &source))
}

/// What a target does with one input, as Ruby source that reads the file named by
/// `$vkrgss_input`. A script is compiled as UTF-8 and not run: its first statement leaves
/// it, so the input begins on its second line.
fn ruby_of(file: &str) -> &'static str {
    match file {
        "marshal" => "Marshal.load(File.open($vkrgss_input, 'rb') { |f| f.read }); :done",
        "parse" => {
            "catch(:compiled) { eval(\"throw :compiled, :done\\n\" + \
             File.open($vkrgss_input, 'rb') { |f| f.read }.force_encoding('UTF-8')) }"
        }
        "regex" => {
            "pattern, subject = File.open($vkrgss_input, 'rb') { |f| f.read }.split(\"\\n\", 2); \
             Regexp.new(pattern.to_s.force_encoding('UTF-8'))\
             .match(subject.to_s.force_encoding('UTF-8')); :done"
        }
        other => panic!("no target file `{other}`"),
    }
}

/// Runs the files of `dir` through what the target of `file` does and returns the result of
/// each.
fn run_inputs(file: &str, dir: &Path) -> Vec<(String, Result<String, String>)> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.file_name().is_some_and(|name| name != ".gitkeep"))
        .collect();
    files.sort();
    files
        .iter()
        .map(|input| {
            let path = input.to_str().expect("UTF-8 path");
            let result = eval(format!("$vkrgss_input = {path:?}; {}", ruby_of(file)));
            let name = input.file_name().expect("a file name").to_string_lossy();
            (format!("{file}/{name}"), result)
        })
        .collect()
}

#[test]
fn every_target_has_its_file_and_well_formed_seeds() {
    for target in &TARGETS {
        let source = fuzz_dir().join(target.file).with_extension("c");
        let text = fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("{}: {error}", source.display()));
        assert!(text.contains("LLVMFuzzerTestOneInput"), "{}", target.name);

        // A seed is an input the interpreter accepts: none raises.
        let results = run_inputs(target.file, &fuzz_dir().join("seeds").join(target.file));
        assert!(results.len() >= 5, "{} has too few seeds", target.name);
        for (seed, result) in results {
            assert_eq!(result, Ok(":done".to_owned()), "{seed}");
        }
    }
}

#[test]
fn the_fuzzer_reads_the_seeds_and_writes_only_below_the_target_directory() {
    let target = &TARGETS[0];
    let arguments = fuzzer_arguments(target, Path::new("/t/fuzz/marshal192"), 30);
    assert_eq!(arguments[0], "/t/fuzz/marshal192/corpus");
    assert!(
        arguments[1].ends_with("fuzz/seeds/marshal"),
        "{arguments:?}"
    );
    for expected in [
        "-max_total_time=30",
        "-max_len=4096",
        "-timeout=10",
        "-artifact_prefix=/t/fuzz/marshal192/artifacts/",
    ] {
        assert!(arguments.iter().any(|arg| arg == expected), "{expected}");
    }
}

#[test]
fn only_the_two_limits_are_not_findings() {
    assert_eq!(ended(Some(0)), Ended::Clean);
    assert_eq!(ended(Some(70)), Ended::Limit("time"));
    assert_eq!(ended(Some(71)), Ended::Limit("memory"));
    // The sanitizers end with 1, libFuzzer's own errors with 77, a signal with no status.
    for status in [Some(1), Some(77), None] {
        assert_eq!(ended(status), Ended::Finding);
    }
    assert_eq!(
        executed_units("stat::number_of_executed_units: 306"),
        Some(306)
    );
    assert_eq!(executed_units("stat::peak_rss_mb:              45"), None);
}
