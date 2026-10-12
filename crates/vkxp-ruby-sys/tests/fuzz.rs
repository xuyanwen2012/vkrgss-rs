//! The fuzz targets: `Marshal.load`, the parser and the regular expression engine fed with
//! arbitrary bytes under libFuzzer, AddressSanitizer and UndefinedBehaviorSanitizer.
//!
//! A target is one C file under `fuzz/` with its seeds under `fuzz/seeds/<target>/`. The
//! ignored test below builds the interpreter and the engine as the sanitized run does, with
//! libFuzzer's coverage instrumentation added, links the target and runs it. It is run by
//! `cargo xtask fuzz <target> [--seconds N]`, which sets the variables it reads; it is not
//! part of the merge gate. The other tests run in the normal test step.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use common::eval_inspect;
use vkxp_ruby_sys::{Interpreter, Options};

/// One target: its name, which is the name of its C file and of its seed directory, the
/// largest input in bytes, and the seconds one input may take.
struct Target {
    name: &'static str,
    max_len: u32,
    timeout: u32,
}

/// The targets. An input is limited in size because nesting recurses in `Marshal.load` and
/// in the parser, and a deep enough input exhausts the machine stack, which this run does
/// not count as a finding; see `docs/development.md` for the depths.
const TARGETS: [Target; 3] = [
    Target {
        name: "marshal",
        max_len: 4096,
        timeout: 10,
    },
    Target {
        name: "parse",
        max_len: 4096,
        timeout: 10,
    },
    Target {
        name: "regex",
        max_len: 512,
        timeout: 10,
    },
];

/// The variable that names the target to run.
const TARGET_VARIABLE: &str = "VKXP_FUZZ_TARGET";

/// The variable that gives the seconds to run for.
const SECONDS_VARIABLE: &str = "VKXP_FUZZ_SECONDS";

/// The seconds a target runs for when the variable does not say.
const DEFAULT_SECONDS: u64 = 60;

/// What the fuzz build adds to the flags of the build script: the sanitizers as in the
/// sanitized run (see `tests/sanitized.rs` for `-fno-sanitize=function`) and the coverage
/// instrumentation of libFuzzer. A target cannot filter reports through an allow-list, so
/// undefined behaviour ends the run, and the one check whose reports the allow-list accepts
/// for the interpreter is off: a null pointer given to `memcpy` with a length of zero.
const FUZZ_FLAGS: &[&str] = &[
    "-fsanitize=address,undefined,fuzzer-no-link",
    "-fno-sanitize=function,nonnull-attribute",
    "-fno-sanitize-recover=undefined",
    "-fno-omit-frame-pointer",
    "-g",
];

/// Added for the files of the engine: it reads the operands of a compiled pattern at
/// whatever offset they have, on purpose (see `tests/sanitized.allow`).
const FUZZ_ONIG_FLAGS: &[&str] = &["-fno-sanitize=alignment"];

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
    let libraries = common::build_instrumented_libraries(scratch, FUZZ_FLAGS, FUZZ_ONIG_FLAGS);
    let fuzz = fuzz_dir();
    let mut includes: Vec<&Path> = libraries.includes.iter().map(PathBuf::as_path).collect();
    includes.push(&fuzz);
    let source = fuzz.join(target.name).with_extension("c");
    let objects = common::compile_jobs(vec![common::job(
        scratch, scratch, &source, &includes, FUZZ_FLAGS,
    )]);
    let executable = scratch.join(format!("fuzz-{}", target.name));
    common::run(
        common::instrumented_compiler(scratch, &[], FUZZ_FLAGS)
            .arg("-fsanitize=fuzzer")
            .args(&objects)
            .args(libraries.link_arguments())
            .arg("-o")
            .arg(&executable),
    );
    executable
}

/// The arguments of libFuzzer for `target`: the corpus that grows, then the seeds, which
/// are only read. The target runs in a child process that is replaced when it ends
/// (`-fork=1`): an input that runs into the time limit, which backtracking does by its
/// nature, or into the memory limit, which a length field that asks for a huge allocation
/// does, is put aside under `artifacts` and the run goes on, while a sanitizer report or a
/// crash ends the run with a failure. A new process also starts with an empty symbol table,
/// which in one process only grows.
fn fuzzer_arguments(target: &Target, dir: &Path, seconds: u64) -> Vec<String> {
    vec![
        dir.join("corpus").display().to_string(),
        fuzz_dir()
            .join("seeds")
            .join(target.name)
            .display()
            .to_string(),
        format!("-max_total_time={seconds}"),
        format!("-max_len={}", target.max_len),
        format!("-timeout={}", target.timeout),
        "-fork=1".to_owned(),
        "-ignore_timeouts=1".to_owned(),
        "-ignore_ooms=1".to_owned(),
        "-ignore_crashes=0".to_owned(),
        format!("-artifact_prefix={}/", dir.join("artifacts").display()),
    ]
}

#[test]
#[ignore = "opt-in: run by `cargo xtask fuzz <target>`, which sets VKXP_FUZZ_TARGET"]
fn the_chosen_target_runs_without_a_finding() {
    let name = std::env::var(TARGET_VARIABLE).unwrap_or_default();
    let target = TARGETS
        .iter()
        .find(|target| target.name == name)
        .unwrap_or_else(|| {
            panic!("{TARGET_VARIABLE} is `{name}`; `cargo xtask fuzz <target>` sets it")
        });
    let seconds = match std::env::var(SECONDS_VARIABLE) {
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
    let (build, tmp) = (dir.join("build"), dir.join("tmp"));
    fresh_dir(&build);
    fresh_dir(&tmp);
    let executable = build_target(&build, target);

    let arguments = fuzzer_arguments(target, &dir, seconds);
    eprintln!("fuzz: {} {}", executable.display(), arguments.join(" "));
    let status = Command::new(&executable)
        .args(&arguments)
        .current_dir(&dir)
        .env("TMPDIR", &tmp)
        .status()
        .unwrap_or_else(|error| panic!("could not start {}: {error}", executable.display()));
    assert!(
        status.success(),
        "the target `{}` ended with {status}: the report is above and the input is in {}",
        target.name,
        dir.join("artifacts").display()
    );
}

fn eval(source: String) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |_| eval_inspect(&source))
}

/// What a target does with one input, as Ruby source that reads the file `VKXP_INPUT`.
/// The script is compiled and not run: its first statement leaves it.
fn ruby_of(target: &str) -> &'static str {
    match target {
        "marshal" => "Marshal.load(File.open(VKXP_INPUT, 'rb') { |f| f.read }); :done",
        "parse" => {
            "catch(:compiled) { eval(\"throw :compiled, :done\\n\" + \
             File.open(VKXP_INPUT, 'rb') { |f| f.read }) }"
        }
        "regex" => {
            "$KCODE = 'U'; pattern, subject = File.open(VKXP_INPUT, 'rb') { |f| f.read }\
             .split(\"\\n\", 2); Regexp.new(pattern.to_s).match(subject.to_s); :done"
        }
        other => panic!("no target `{other}`"),
    }
}

/// Runs the files of `dir` through what `target` does and returns the result of each.
fn run_inputs(target: &str, dir: &Path) -> Vec<(String, Result<String, String>)> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    files.sort();
    files
        .iter()
        .map(|file| {
            let path = file.to_str().expect("UTF-8 path");
            let result = eval(format!("VKXP_INPUT = {path:?}; {}", ruby_of(target)));
            let name = file.file_name().expect("a file name").to_string_lossy();
            (format!("{target}/{name}"), result)
        })
        .collect()
}

#[test]
fn every_target_has_its_file_and_well_formed_seeds() {
    for target in &TARGETS {
        let source = fuzz_dir().join(target.name).with_extension("c");
        let text = fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("{}: {error}", source.display()));
        assert!(text.contains("LLVMFuzzerTestOneInput"), "{}", target.name);

        // A seed is an input the interpreter accepts: none raises.
        let results = run_inputs(target.name, &fuzz_dir().join("seeds").join(target.name));
        assert!(results.len() >= 5, "{} has too few seeds", target.name);
        for (seed, result) in results {
            assert_eq!(result, Ok(":done".to_owned()), "{seed}");
        }
    }
}

#[test]
fn the_fuzzer_reads_the_seeds_and_writes_only_below_the_target_directory() {
    let target = &TARGETS[0];
    let arguments = fuzzer_arguments(target, Path::new("/t/fuzz/marshal"), 30);
    assert_eq!(arguments[0], "/t/fuzz/marshal/corpus");
    assert!(
        arguments[1].ends_with("fuzz/seeds/marshal"),
        "{arguments:?}"
    );
    for expected in [
        "-max_total_time=30",
        "-max_len=4096",
        "-timeout=10",
        "-artifact_prefix=/t/fuzz/marshal/artifacts/",
    ] {
        assert!(arguments.iter().any(|arg| arg == expected), "{expected}");
    }
}
