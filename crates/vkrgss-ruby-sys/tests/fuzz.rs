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
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Instant;

use common::eval_inspect;
use vkrgss_ruby_sys::{Interpreter, Options};

/// One target: its name, which is the name of its C file and of its seed directory, the
/// largest input in bytes, and the seconds one input may take.
struct Target {
    name: &'static str,
    max_len: u32,
    timeout: u32,
    /// The functions in which a stack overflow is set aside; see [`ended`].
    recursive: &'static [&'static [&'static str]],
}

/// The functions of Oniguruma 5.9.6 (`regparse.c`, `regcomp.c`) that recurse, directly
/// or through each other, once for each level of nesting of a pattern or each step of a
/// reference from one group to another.
const ENGINE_RECURSIVE: &[&str] = &[
    "parse_subexp",
    "parse_branch",
    "parse_exp",
    "parse_enclose",
    "parse_char_class",
    "fetch_escaped_value",
    "is_invalid_quantifier_target",
    "noname_disable_map",
    "renumber_by_map",
    "numbered_ref_check",
    "setup_tree",
    "setup_comb_exp_check",
    "setup_subexp_call",
    "subexp_recursive_check",
    "subexp_recursive_check_trav",
    "subexp_inf_recursive_check",
    "subexp_inf_recursive_check_trav",
    "quantifiers_memory_node_info",
    "get_min_match_length",
    "get_max_match_length",
    "get_char_length_tree1",
    "get_head_value_node",
    "check_type_tree",
    "optimize_node_left",
    "compile_length_tree",
    "compile_length_anchor_node",
    "compile_length_enclose_node",
    "compile_length_option_node",
    "compile_length_quantifier_node",
    "compile_tree",
    "compile_tree_n_times",
    "compile_tree_empty_check",
    "compile_anchor_node",
    "compile_enclose_node",
    "compile_option_node",
    "compile_quantifier_node",
    "compile_range_repeat_node",
    "onig_node_free",
];

/// The functions of the parser (`parse.y`) that recurse on the nesting of the nodes or of an
/// escape. Bison keeps its own stack, which is limited, on the heap.
const PARSER_RECURSIVE: &[&str] = &[
    "value_expr0",
    "cond0",
    "range_op",
    "assign_in_cond",
    "arg_prepend",
    "read_escape",
    "tokadd_escape",
];

/// The functions of `marshal.c` that recurse once for each level of nesting of the data.
const MARSHAL_RECURSIVE: &[&str] = &["r_object0", "r_object", "r_ivar"];

/// The targets. An input is limited in size because nesting recurses in `Marshal.load`, in
/// the parser and in the regular expression engine, and a deep enough input exhausts the
/// machine stack. This run sets that aside when the stack ends in a function listed for the
/// target; the parser lists those of the engine too, because it compiles a pattern literal
/// while it parses, and so does `Marshal.load`, because it compiles the pattern of a Regexp. See `docs/development.md` for the depths.
const TARGETS: [Target; 3] = [
    Target {
        name: "marshal",
        max_len: 4096,
        timeout: 10,
        recursive: &[MARSHAL_RECURSIVE, ENGINE_RECURSIVE],
    },
    Target {
        name: "parse",
        max_len: 4096,
        timeout: 10,
        recursive: &[PARSER_RECURSIVE, ENGINE_RECURSIVE],
    },
    Target {
        name: "regex",
        max_len: 512,
        timeout: 3,
        recursive: &[ENGINE_RECURSIVE],
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
    common::manifest_dir().join("fuzz")
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
            .join(target.name)
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
    /// An input ran into the time limit, which backtracking does by its nature, the
    /// process into the memory limit, which the symbol table does because it only grows,
    /// or an input exhausted the stack by nesting. None is a finding: the input is put
    /// aside and the run goes on in a new process.
    Limit(&'static str),
    /// A sanitizer report or a crash.
    Finding,
}

/// The frames of a report of a stack overflow that are looked at for a function of the
/// target's list: the function that ran into the end of the stack may be one that the
/// recursive function calls, such as an allocation, a helper or a callback of libFuzzer's
/// instrumentation.
const STACK_FRAMES: usize = 8;

/// How a process ended, from its status and the sanitizer's report on its error stream. A
/// stack overflow is a limit, named "stack", when one of the first frames of its report is
/// in one of the `recursive` functions; anywhere else it is a finding.
fn ended(status: Option<i32>, report: &str, recursive: &[&[&str]]) -> Ended {
    match status {
        Some(0) => Ended::Clean,
        Some(TIMEOUT_STATUS) => Ended::Limit("time"),
        Some(OUT_OF_MEMORY_STATUS) => Ended::Limit("memory"),
        _ if exhausted_by_nesting(report, recursive) => Ended::Limit("stack"),
        _ => Ended::Finding,
    }
}

/// Whether `report` is AddressSanitizer's report of a stack overflow with one of the
/// `recursive` functions among the first frames of its stack trace.
fn exhausted_by_nesting(report: &str, recursive: &[&[&str]]) -> bool {
    let mut lines = report
        .lines()
        .skip_while(|line| !line.contains("ERROR: AddressSanitizer: stack-overflow"));
    if lines.next().is_none() {
        return false;
    }
    lines
        .map(str::trim_start)
        .skip_while(|line| !line.starts_with('#'))
        .map_while(frame_function)
        .take(STACK_FRAMES)
        .any(|function| recursive.iter().any(|list| list.contains(&function)))
}

/// The function of a frame of a sanitizer's stack trace, as `parse_exp` of
/// `#3 0x55d0c1 in parse_exp /src/regparse.c:4512:9`.
fn frame_function(line: &str) -> Option<&str> {
    let (_, function) = line.strip_prefix('#')?.split_once(" in ")?;
    function.split_whitespace().next()
}

/// The file to which libFuzzer says it wrote the input that ended a process.
fn written_input(line: &str) -> Option<PathBuf> {
    let (_, path) = line.split_once("Test unit written to ")?;
    Some(PathBuf::from(path.trim()))
}

/// Renames the input of a process that exhausted the stack from libFuzzer's `crash-` to
/// `stack-`, as libFuzzer names those of the other limits `timeout-` and `oom-`, so that
/// it is not taken for the input of a finding.
fn set_aside_as_stack(input: &Path) {
    let Some(hash) = input
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("crash-"))
    else {
        return;
    };
    let renamed = input.with_file_name(format!("stack-{hash}"));
    fs::rename(input, &renamed)
        .unwrap_or_else(|error| panic!("{} -> {}: {error}", input.display(), renamed.display()));
}

/// The number of inputs a process says it ran, from the statistics it prints at its end.
fn executed_units(line: &str) -> Option<u64> {
    line.strip_prefix("stat::number_of_executed_units:")?
        .trim()
        .parse()
        .ok()
}

/// What one process of a target left.
struct Process {
    status: Option<i32>,
    /// The number of inputs it ran.
    executed: u64,
    /// The sanitizer's report on its error stream, from its first line on; empty without one.
    report: String,
    /// The file to which libFuzzer wrote the input that ended it.
    input: Option<PathBuf>,
}

/// Runs one process of the target for at most `seconds`, passes its output through and
/// returns what it left.
fn run_once(executable: &Path, target: &Target, dir: &Path, seconds: u64) -> Process {
    let mut child = Command::new(executable)
        .args(fuzzer_arguments(target, dir, seconds))
        .current_dir(dir)
        .env("TMPDIR", dir.join("tmp"))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("could not start {}: {error}", executable.display()));
    let (mut executed, mut report, mut input) = (0, String::new(), None);
    let output = BufReader::new(child.stderr.take().expect("the piped error stream"));
    for line in output.split(b'\n') {
        let line = String::from_utf8_lossy(&line.expect("output of the target")).into_owned();
        eprintln!("{line}");
        executed += executed_units(&line).unwrap_or(0);
        input = written_input(&line).or(input);
        if !report.is_empty() || line.contains("==ERROR: ") {
            report.push_str(&line);
            report.push('\n');
        }
    }
    Process {
        status: child.wait().expect("the target ends").code(),
        executed,
        report,
        input,
    }
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
    let dir = common::host_dir("CARGO_TARGET_TMPDIR", env!("CARGO_TARGET_TMPDIR"))
        .join("fuzz")
        .join(target.name);
    for kept in ["corpus", "artifacts"] {
        fs::create_dir_all(dir.join(kept)).expect("directory of the target");
    }
    fresh_dir(&dir.join("build"));
    fresh_dir(&dir.join("tmp"));
    let executable = build_target(&dir.join("build"), target);

    let started = Instant::now();
    let (mut executed, mut time_limits, mut memory_limits, mut stack_limits) = (0, 0, 0, 0);
    loop {
        let left = seconds.saturating_sub(started.elapsed().as_secs());
        if left == 0 {
            break;
        }
        let process = run_once(&executable, target, &dir, left);
        executed += process.executed;
        match ended(process.status, &process.report, target.recursive) {
            Ended::Clean => break,
            Ended::Limit("time") => time_limits += 1,
            Ended::Limit("memory") => memory_limits += 1,
            Ended::Limit(_) => {
                stack_limits += 1;
                if let Some(input) = &process.input {
                    set_aside_as_stack(input);
                }
            }
            Ended::Finding => panic!(
                "the target `{}` ended with {:?} after {executed} inputs: the report is \
                 above and the input is in {}",
                target.name,
                process.status,
                dir.join("artifacts").display()
            ),
        }
    }
    eprintln!(
        "fuzz: `{}` ran {executed} inputs in {} s without a finding; {time_limits} input(s) \
         ran into the time limit of {} s, {stack_limits} exhausted the stack by nesting and \
         {memory_limits} process(es) ran into the memory limit, which are put aside in {}",
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
        .run(move |_| eval_inspect(&source))
}

/// What a target does with one input, as Ruby source that reads the file `VKRGSS_INPUT`.
/// The script is compiled and not run: its first statement leaves it.
fn ruby_of(target: &str) -> &'static str {
    match target {
        "marshal" => "Marshal.load(File.open(VKRGSS_INPUT, 'rb') { |f| f.read }); :done",
        "parse" => {
            "catch(:compiled) { eval(\"throw :compiled, :done\\n\" + \
             File.open(VKRGSS_INPUT, 'rb') { |f| f.read }) }"
        }
        "regex" => {
            "$KCODE = 'U'; pattern, subject = File.open(VKRGSS_INPUT, 'rb') { |f| f.read }\
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
            let result = eval(format!("VKRGSS_INPUT = {path:?}; {}", ruby_of(target)));
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

/// The inputs with which a target failed before the defect was fixed, reduced, under
/// `fuzz/fixed/<target>/`; `PATCHES.md` of the vendored tree names the patch of each. Here
/// they run without the sanitizers, so the test shows that the interpreter survives them
/// and still computes: two of the defects were reads through a null pointer, which end the
/// process in any build, and the others are reported by `cargo xtask fuzz` again if one
/// comes back. Most of these inputs are refused with an exception, which is the right
/// answer to them.
#[test]
fn the_inputs_of_fixed_findings_no_longer_fail() {
    let mut ran = 0;
    for target in &TARGETS {
        for (input, result) in run_inputs(target.name, &fuzz_dir().join("fixed").join(target.name))
        {
            assert!(
                result.as_deref() == Ok(":done") || result.is_err(),
                "{input}: {result:?}"
            );
            assert_eq!(
                eval("1 + 1".to_owned()),
                Ok("2".to_owned()),
                "after {input}"
            );
            ran += 1;
        }
    }
    assert!(ran >= 10, "only {ran} inputs of fixed findings were found");

    // What four of the inputs are now: syntax errors that name the cause.
    let error = |name: &str| -> String {
        let path = fuzz_dir().join("fixed/parse").join(name);
        let path = path.to_str().expect("UTF-8 path");
        eval(format!("VKRGSS_INPUT = {path:?}; {}", ruby_of("parse"))).expect_err("a syntax error")
    };
    let range = error("range-null-operand.rb");
    assert!(range.contains("constant re-assignment"), "{range}");
    let condition = error("condition-null-value.rb");
    assert!(
        condition.contains("Can't assign to __FILE__"),
        "{condition}"
    );
    // The token of the number is read after the error, and ends inside its buffer.
    let number = error("number-trailing-underscore.rb");
    assert!(number.contains("trailing `_' in number"), "{number}");
    // The error after `__END__` quotes the line of `__END__`, which the parser no longer
    // lets the collector free; under stress it collects while the error is reported.
    let end = eval(
        "GC.stress = true
         begin
           eval(\"module A\\n__END__\\n\")
         rescue SyntaxError => error
           error.message
         ensure
           GC.stress = false
         end"
        .to_owned(),
    )
    .expect("the message");
    assert!(end.ends_with("expecting kEND\\n__END__\\n ^\""), "{end}");
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

/// The target of the name.
fn target(name: &str) -> &'static Target {
    TARGETS
        .iter()
        .find(|target| target.name == name)
        .expect("a target")
}

/// A report of AddressSanitizer of a stack overflow with the functions of `frames`, the
/// first frame first, as a process of a target writes it.
fn stack_overflow(frames: &[&str]) -> String {
    let mut report = "AddressSanitizer:DEADLYSIGNAL\n\
        ==4242==ERROR: AddressSanitizer: stack-overflow on address 0x7ffc8e3f9ff8 \
        (pc 0x55d0c1a2b3c4 bp 0x7ffc8e3fa010 sp 0x7ffc8e3fa000 T0)\n"
        .to_owned();
    for (index, function) in frames.iter().enumerate() {
        report.push_str(&format!(
            "    #{index} 0x55d0c1a2{index:04x} in {function} /src/third_party/oniguruma-5.9.6/regcomp.c:2030:3\n"
        ));
    }
    report.push_str("SUMMARY: AddressSanitizer: stack-overflow in the first frame\n");
    report
}

#[test]
fn only_the_limits_are_not_findings() {
    let recursive = target("regex").recursive;
    assert_eq!(ended(Some(0), "", recursive), Ended::Clean);
    assert_eq!(ended(Some(70), "", recursive), Ended::Limit("time"));
    assert_eq!(ended(Some(71), "", recursive), Ended::Limit("memory"));
    // The sanitizers end with 1, libFuzzer's own errors with 77, a signal with no status.
    for status in [Some(1), Some(77), None] {
        assert_eq!(ended(status, "", recursive), Ended::Finding);
    }
    assert_eq!(
        executed_units("stat::number_of_executed_units: 306"),
        Some(306)
    );
    assert_eq!(executed_units("stat::peak_rss_mb:              45"), None);
    assert_eq!(
        written_input("artifact_prefix='/t/a/'; Test unit written to /t/a/crash-0f1e"),
        Some(PathBuf::from("/t/a/crash-0f1e"))
    );
}

#[test]
fn a_stack_exhausted_by_nesting_is_a_limit() {
    let deep = stack_overflow(&[
        "get_min_match_length",
        "get_min_match_length",
        "get_min_match_length",
        "setup_tree",
    ]);
    assert_eq!(
        ended(Some(1), &deep, target("regex").recursive),
        Ended::Limit("stack")
    );
    // The end of the stack reached in a function that a recursive one calls.
    let called = stack_overflow(&[
        "__sanitizer_cov_trace_cmp4",
        "onig_node_new_enclose",
        "parse_enclose",
        "parse_exp",
        "parse_branch",
        "parse_subexp",
    ]);
    assert_eq!(
        ended(Some(1), &called, target("regex").recursive),
        Ended::Limit("stack")
    );
    // A pattern literal is compiled by the parser.
    assert_eq!(
        ended(Some(1), &deep, target("parse").recursive),
        Ended::Limit("stack")
    );
    let marshal = stack_overflow(&["rb_ary_push", "r_object0", "r_object", "r_object0"]);
    assert_eq!(
        ended(Some(1), &marshal, target("marshal").recursive),
        Ended::Limit("stack")
    );
    // `Marshal.load` compiles the pattern of a Regexp, as nightly run 39227119222 found.
    assert_eq!(
        ended(Some(1), &deep, target("marshal").recursive),
        Ended::Limit("stack")
    );
}

#[test]
fn another_stack_overflow_is_a_finding() {
    // No listed function among the first frames.
    let elsewhere = stack_overflow(&[
        "rb_str_format",
        "rb_f_sprintf",
        "compile",
        "vkrgss_fuzz_protected",
    ]);
    for target in &TARGETS {
        assert_eq!(
            ended(Some(1), &elsewhere, target.recursive),
            Ended::Finding,
            "{}",
            target.name
        );
    }
    // A function of another target's list.
    let parser = stack_overflow(&["value_expr0", "value_expr0"]);
    assert_eq!(
        ended(Some(1), &parser, target("marshal").recursive),
        Ended::Finding
    );
    // A listed function only below the first frames.
    let mut frames = vec!["alloca_in_a_loop"; STACK_FRAMES];
    frames.push("get_min_match_length");
    assert_eq!(
        ended(Some(1), &stack_overflow(&frames), target("regex").recursive),
        Ended::Finding
    );
    // Another report with a listed function in its frames.
    let overflow = stack_overflow(&["get_min_match_length"]).replace(
        "stack-overflow on address",
        "heap-buffer-overflow on address",
    );
    assert_eq!(
        ended(Some(1), &overflow, target("regex").recursive),
        Ended::Finding
    );
    // A trace without names, when the symbolizer is missing.
    let unnamed = "==7==ERROR: AddressSanitizer: stack-overflow on address 0x7ffc8e3f9ff8\n    \
                   #0 0x55d0c1a2b3c4  (/src/third_party/ruby-1.8.7/fuzz-regex+0x1b3c4)\n";
    assert_eq!(
        ended(Some(1), unnamed, target("regex").recursive),
        Ended::Finding
    );
}

#[test]
fn the_input_of_an_exhausted_stack_is_renamed_beside_the_others() {
    let dir = common::host_dir("CARGO_TARGET_TMPDIR", env!("CARGO_TARGET_TMPDIR"))
        .join("fuzz-set-aside-regex");
    fresh_dir(&dir);
    let input = dir.join("crash-0f1e");
    fs::write(&input, b"((((").expect("an input");
    set_aside_as_stack(&input);
    assert!(!input.exists());
    assert_eq!(
        fs::read(dir.join("stack-0f1e")).expect("the input"),
        b"(((("
    );
}
