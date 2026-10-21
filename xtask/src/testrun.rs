//! The test steps of the gate, run at the same time. Each step builds its test binaries with
//! one `cargo test --no-run`, with its own arguments, and every binary then runs as `cargo
//! test` runs it: one process, in the directory of its package, with `CARGO`,
//! `CARGO_MANIFEST_DIR` and `CARGO_MANIFEST_PATH` set. The binaries of all the steps and their
//! documentation tests, which `cargo test --doc` runs, share as many places at a time as the
//! machine has hardware threads. The output of each binary is kept in `target/gate/<step>/`
//! and printed in one piece when it ends, and every binary runs, whatever another one gave.

use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, ensure};
use serde_json::Value;

use crate::gate::{Step, format_duration};
use crate::quick::Selection;

/// The variable that sets the cases of the property tests. The gate clears it, so that it does
/// the same work everywhere, unless a step sets it, as `cargo xtask gate --quick` does.
pub(crate) const PROPTEST_CASES: &str = "PROPTEST_CASES";

/// A test binary that `cargo test --no-run` built.
#[derive(Debug)]
pub(crate) struct Binary {
    /// The manifest of its package.
    pub(crate) manifest_path: PathBuf,
    /// The name of the target.
    pub(crate) name: String,
    /// The root source file of the target.
    pub(crate) source: PathBuf,
    pub(crate) executable: PathBuf,
}

/// One process to run: a test binary, or the documentation tests of a step.
struct Job {
    step: usize,
    label: String,
    /// The name of its log in the directory of the step.
    log: String,
    program: PathBuf,
    args: Vec<String>,
    dir: PathBuf,
    env: Vec<(String, String)>,
}

/// What became of the jobs of a step.
#[derive(Default)]
struct Progress {
    /// Whether all of its jobs are queued.
    queued: bool,
    pending: usize,
    failures: Vec<String>,
    took: Option<Duration>,
}

/// The jobs waiting for a place, and whether more may come.
#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    closed: bool,
}

/// Runs the test steps `steps` at the same time and returns, for each, how long it took from
/// the start until its last job ended and whether it passed. `selection` chooses the binaries
/// of `cargo xtask gate --quick`.
pub(crate) fn run(
    steps: &[Step],
    root: &Path,
    selection: Option<&Selection>,
) -> Vec<(&'static str, Duration, Result<()>)> {
    let started = Instant::now();
    let logs = root.join("target").join("gate");
    for step in steps {
        let _ = fs::remove_dir_all(logs.join(step.name));
    }
    let queue = (Mutex::new(Queue::default()), Condvar::new());
    let progress: Mutex<Vec<Progress>> =
        Mutex::new(steps.iter().map(|_| Progress::default()).collect());
    let output = Mutex::new(());
    let without_doctests = without_doctests(root);
    let places = std::thread::available_parallelism().map_or(4, |count| count.get());

    std::thread::scope(|scope| {
        for _ in 0..places {
            scope.spawn(|| {
                while let Some(job) = next_job(&queue) {
                    let failure = run_job(&job, steps[job.step].name, &logs, &output);
                    let mut progress = progress.lock().unwrap();
                    let step = &mut progress[job.step];
                    step.pending -= 1;
                    step.failures.extend(failure);
                    end_if_done(step, steps[job.step].name, started);
                }
            });
        }
        // Built one after another, as they share the build directory, and run as soon as
        // each is built.
        for (index, step) in steps.iter().enumerate() {
            let jobs = match without_doctests
                .as_ref()
                .map_err(|error| anyhow!("{error:#}"))
                .and_then(|without| jobs_of(step, index, without, root, selection))
            {
                Ok(jobs) => jobs,
                Err(error) => {
                    let failure = format!("{error:#}");
                    progress.lock().unwrap()[index].failures.push(failure);
                    Vec::new()
                }
            };
            let mut progress = progress.lock().unwrap();
            progress[index].pending += jobs.len();
            progress[index].queued = true;
            queue.0.lock().unwrap().jobs.extend(jobs);
            queue.1.notify_all();
            end_if_done(&mut progress[index], step.name, started);
        }
        queue.0.lock().unwrap().closed = true;
        queue.1.notify_all();
    });

    let progress = progress.into_inner().unwrap();
    steps
        .iter()
        .zip(progress)
        .map(|(step, progress)| {
            let result = match progress.failures.is_empty() {
                true => Ok(()),
                false => Err(anyhow!(
                    "gate: FAILED at step `{}`: {}",
                    step.name,
                    progress.failures.join("; ")
                )),
            };
            (step.name, progress.took.unwrap_or_default(), result)
        })
        .collect()
}

/// The next job, waiting while the queue is empty but not closed.
fn next_job((queue, ready): &(Mutex<Queue>, Condvar)) -> Option<Job> {
    let mut queue = queue.lock().unwrap();
    loop {
        if let Some(job) = queue.jobs.pop_front() {
            return Some(job);
        }
        if queue.closed {
            return None;
        }
        queue = ready.wait(queue).unwrap();
    }
}

/// Records when a step whose jobs have all ended took and prints it.
fn end_if_done(progress: &mut Progress, name: &str, started: Instant) {
    if progress.queued && progress.pending == 0 && progress.took.is_none() {
        let took = started.elapsed();
        progress.took = Some(took);
        eprintln!("gate: step `{name}` took {}", format_duration(took));
    }
}

/// Builds the test binaries of `step` and returns the jobs that run them and its
/// documentation tests, leaving out the packages `without_doctests`; only the binaries that
/// `selection` chooses, if there is one. A step that selects only the documentation tests is
/// one job.
fn jobs_of(
    step: &Step,
    index: usize,
    without_doctests: &[String],
    root: &Path,
    selection: Option<&Selection>,
) -> Result<Vec<Job>> {
    let command_line = format!("cargo {}", step.args.join(" "));
    eprintln!("gate: running step `{}` ({command_line})", step.name);
    let env: Vec<(String, String)> = step
        .env
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    let cargo_job = |label: &str, args: Vec<String>| Job {
        step: index,
        label: label.to_owned(),
        log: format!("doc-{index}.log"),
        program: PathBuf::from(crate::cargo()),
        args,
        dir: root.to_owned(),
        env: env.clone(),
    };
    let args: Vec<String> = step.args.iter().map(|arg| (*arg).to_owned()).collect();
    if step.args.contains(&"--doc") {
        return Ok(vec![cargo_job(&command_line, args)]);
    }
    let mut binaries = build(step, root).context("the build of the test binaries")?;
    if let Some(selection) = selection {
        binaries = selection.choose(binaries, root);
    }
    let mut jobs = Vec::new();
    let selects_targets = step
        .args
        .iter()
        .any(|arg| matches!(*arg, "--lib" | "--bins" | "--bin" | "--test" | "--tests"));
    if !selects_targets && let Some(doc) = doc_args(step.args, without_doctests) {
        jobs.push(cargo_job(&format!("cargo {}", doc.join(" ")), doc));
    }
    for binary in binaries {
        let dir = binary
            .manifest_path
            .parent()
            .expect("a manifest is in a directory")
            .to_owned();
        let mut env = env.clone();
        env.push(("CARGO".to_owned(), crate::cargo()));
        env.push(("CARGO_MANIFEST_DIR".to_owned(), path_text(&dir)));
        env.push((
            "CARGO_MANIFEST_PATH".to_owned(),
            path_text(&binary.manifest_path),
        ));
        let name = binary.executable.file_name().unwrap_or_default();
        jobs.push(Job {
            step: index,
            label: format!(
                "{} ({})",
                relative(&binary.source, root),
                relative(&binary.executable, root)
            ),
            log: format!("{}.log", name.to_string_lossy()),
            program: binary.executable,
            args: Vec::new(),
            dir,
            env,
        });
    }
    Ok(jobs)
}

/// The arguments of `cargo test` that run the documentation tests that `cargo test args`
/// runs: `--doc` would also run those of the packages `without`, whose manifests turn them
/// off, so these are left out. `None` when no package is left.
fn doc_args(args: &[&str], without: &[String]) -> Option<Vec<String>> {
    let mut doc = Vec::new();
    let (mut named, mut kept) = (false, false);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if *arg == "-p" {
            let package = args.next()?;
            named = true;
            if !without.iter().any(|name| name == package) {
                kept = true;
                doc.extend(["-p".to_owned(), (*package).to_owned()]);
            }
        } else {
            doc.push((*arg).to_owned());
        }
    }
    if named && !kept {
        return None;
    }
    doc.push("--doc".to_owned());
    if doc.iter().any(|arg| arg == "--workspace") {
        for package in without {
            doc.extend(["--exclude".to_owned(), package.clone()]);
        }
    }
    Some(doc)
}

/// The packages of the workspace whose library has its documentation tests turned off.
fn without_doctests(root: &Path) -> Result<Vec<String>> {
    let output = Command::new(crate::cargo())
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .context("could not start cargo metadata")?;
    ensure!(
        output.status.success(),
        "cargo metadata ended with {}",
        output.status
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).context("cargo metadata")?;
    let packages = metadata["packages"].as_array().context("cargo metadata")?;
    Ok(packages
        .iter()
        .filter(|package| {
            package["targets"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|target| target["kind"][0] == "lib" && target["doctest"] == false)
        })
        .filter_map(|package| package["name"].as_str().map(str::to_owned))
        .collect())
}

/// Builds the test binaries that `step` runs, with its arguments and `--no-run`. The
/// messages of the compiler go to the terminal as they come.
fn build(step: &Step, root: &Path) -> Result<Vec<Binary>> {
    let output = Command::new(crate::cargo())
        .args(step.args)
        .args(["--no-run", "--message-format=json-render-diagnostics"])
        .envs(step.env.iter().copied())
        .current_dir(root)
        .stderr(Stdio::inherit())
        .output()
        .context("could not start cargo")?;
    ensure!(
        output.status.success(),
        "`cargo {} --no-run` ended with {}",
        step.args.join(" "),
        output.status
    );
    Ok(binaries(&String::from_utf8_lossy(&output.stdout)))
}

/// The test binaries among the messages of `cargo test --no-run --message-format=json`.
fn binaries(messages: &str) -> Vec<Binary> {
    messages
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-artifact")
        .filter(|message| message["profile"]["test"] == true)
        .filter_map(|message| {
            let text = |value: &Value| value.as_str().map(str::to_owned);
            Some(Binary {
                manifest_path: text(&message["manifest_path"])?.into(),
                name: text(&message["target"]["name"])?,
                source: text(&message["target"]["src_path"])?.into(),
                executable: text(&message["executable"])?.into(),
            })
        })
        .collect()
}

/// Runs one job with its output in a file below `logs`, prints that output in one piece and
/// returns a description of the failure, if it failed.
fn run_job(job: &Job, step: &str, logs: &Path, output: &Mutex<()>) -> Option<String> {
    let started = Instant::now();
    let dir = logs.join(step);
    let log = dir.join(&job.log);
    let status = start(job, &dir, &log);
    let took = format_duration(started.elapsed());
    let text = fs::read(&log).unwrap_or_default();
    let (verdict, failure) = match &status {
        Ok(status) if status.success() => ("passed".to_owned(), None),
        Ok(status) => (format!("FAILED ({status})"), Some(job.label.clone())),
        Err(error) => (
            format!("FAILED: {error:#}"),
            Some(format!("{}: {error:#}", job.label)),
        ),
    };
    let _printing = output.lock().unwrap();
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "     Running {} [{step}]", job.label);
    let _ = stderr.write_all(&text);
    let _ = writeln!(stderr, "gate: {step}: {} {verdict} in {took}", job.label);
    failure
}

/// Starts the job with both of its output streams in `log` and waits for it.
fn start(job: &Job, dir: &Path, log: &Path) -> Result<ExitStatus> {
    fs::create_dir_all(dir).with_context(|| dir.display().to_string())?;
    let file = File::create(log).with_context(|| log.display().to_string())?;
    let errors = file.try_clone().context("the log is opened twice")?;
    Command::new(&job.program)
        .args(&job.args)
        .current_dir(&job.dir)
        .env_remove(PROPTEST_CASES)
        .envs(job.env.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::null())
        .stdout(file)
        .stderr(errors)
        .status()
        .with_context(|| format!("could not start {}", job.program.display()))
}

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_binaries_are_read_from_the_messages_of_cargo() {
        let messages = r#"{"reason":"compiler-artifact","manifest_path":"/w/crates/a/Cargo.toml","target":{"kind":["lib"],"name":"a","src_path":"/w/crates/a/src/lib.rs"},"profile":{"test":true},"executable":"/w/target/debug/deps/a-1"}
{"reason":"compiler-artifact","manifest_path":"/w/crates/a/Cargo.toml","target":{"kind":["lib"],"name":"a","src_path":"/w/crates/a/src/lib.rs"},"profile":{"test":false},"executable":null}
{"reason":"compiler-artifact","manifest_path":"/w/crates/a/Cargo.toml","target":{"kind":["bin"],"name":"a","src_path":"/w/crates/a/src/main.rs"},"profile":{"test":false},"executable":"/w/target/debug/a"}
{"reason":"build-finished","success":true}"#;
        let found = binaries(messages);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].executable, Path::new("/w/target/debug/deps/a-1"));
        assert_eq!(found[0].source, Path::new("/w/crates/a/src/lib.rs"));
        assert_eq!(found[0].name, "a");
    }

    #[test]
    fn the_documentation_tests_leave_out_the_packages_that_turn_them_off() {
        let without = ["b3".to_owned()];
        let doc = |args: &[&str]| doc_args(args, &without).map(|doc| doc.join(" "));
        assert_eq!(
            doc(&["test", "--workspace", "--locked", "--no-fail-fast"]).unwrap(),
            "test --workspace --locked --no-fail-fast --doc --exclude b3"
        );
        assert_eq!(
            doc(&["test", "-p", "a", "-p", "b3", "--locked"]).unwrap(),
            "test -p a --locked --doc"
        );
        assert_eq!(doc(&["test", "-p", "b3", "--locked"]), None);
    }
}
