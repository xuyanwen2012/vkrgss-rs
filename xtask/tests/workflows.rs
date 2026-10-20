//! The workflows: the parts of the gate on x86_64 and, allowed to fail, the whole gate on
//! aarch64; the release only for a tag; the checks outside the gate once a day, with an issue
//! when they fail; and in all of them inputs that do not move.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex_lite::Regex;

fn workflows_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows")
}

fn read_workflow(name: &str) -> String {
    let path = workflows_dir().join(name);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// Every workflow file with its name.
fn workflows() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(workflows_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".yml") || name.ends_with(".yaml"))
        .map(|name| {
            let text = read_workflow(&name);
            (name, text)
        })
        .collect();
    files.sort();
    files
}

/// The lines of the job `name` in the workflow text: from the line after its key to the
/// next line that is indented as little as the key.
fn job<'a>(workflow: &'a str, name: &str) -> Vec<&'a str> {
    let key = format!("  {name}:");
    workflow
        .lines()
        .skip_while(|line| *line != key)
        .skip(1)
        .take_while(|line| line.is_empty() || line.starts_with("    "))
        .collect()
}

/// The names of the jobs of a workflow: the keys indented by two spaces below `jobs:`.
fn job_names(workflow: &str) -> Vec<&str> {
    workflow
        .lines()
        .skip_while(|line| *line != "jobs:")
        .skip(1)
        .filter_map(|line| {
            let key = line.strip_prefix("  ")?.strip_suffix(':')?;
            (!key.starts_with([' ', '#'])).then_some(key)
        })
        .collect()
}

/// The lines of a job from `steps:` on, without comments and blank lines.
fn steps<'a>(job: &[&'a str]) -> Vec<&'a str> {
    job.iter()
        .copied()
        .skip_while(|line| line.trim() != "steps:")
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .collect()
}

/// The lines of a workflow without comment lines.
fn code_lines(workflow: &str) -> impl Iterator<Item = &str> {
    workflow
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
}

/// The names of the parts of the gate, from the message of `cargo xtask gate --job` for a
/// part that does not exist.
fn parts() -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["gate", "--job", "nothing"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let message = String::from_utf8(output.stderr).unwrap();
    let names = message
        .trim_end()
        .split_once("the parts are: ")
        .unwrap_or_else(|| panic!("{message}"))
        .1;
    names.split(", ").map(str::to_owned).collect()
}

const GATE: &str = "VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask gate";

#[test]
fn each_part_of_the_gate_is_a_job() {
    let workflow = read_workflow("ci.yml");
    let parts = parts();
    assert!(parts.len() > 1, "{parts:?}");
    let mut jobs = job_names(&workflow);
    jobs.retain(|name| *name != "gate-aarch64");
    assert_eq!(jobs, parts, "the jobs of ci.yml besides gate-aarch64");

    let first = job(&workflow, &parts[0]);
    for part in &parts {
        let lines = job(&workflow, part);
        assert!(lines.contains(&"    runs-on: ubuntu-24.04"), "{part}");
        assert!(
            !lines.iter().any(|line| line.contains("continue-on-error")),
            "{part}"
        );
        let command = format!("        run: {GATE} --job {part}");
        assert_eq!(steps(&lines).last(), Some(&command.as_str()), "{part}");
        // The same job for every part, apart from the part it runs.
        let renamed: Vec<String> = lines
            .iter()
            .map(|line| line.replace(&format!("--job {part}"), &format!("--job {}", parts[0])))
            .collect();
        assert_eq!(renamed, first, "{part}");
    }
}

#[test]
fn the_parts_cache_the_objects_of_c_and_rust() {
    let workflow = read_workflow("ci.yml");
    for name in job_names(&workflow) {
        let lines = job(&workflow, name);
        for variable in [
            "      RUSTC_WRAPPER: sccache",
            "      SCCACHE_GHA_ENABLED: \"true\"",
        ] {
            assert!(lines.contains(&variable), "{name}: `{variable}`");
        }
        let position = |action: &str| {
            lines
                .iter()
                .position(|line| line.contains(action))
                .unwrap_or_else(|| panic!("{name} does not use {action}"))
        };
        let sccache = position("uses: mozilla-actions/sccache-action@");
        assert!(sccache < position("uses: Swatinem/rust-cache@"), "{name}");
        assert!(sccache < position("cargo xtask gate"), "{name}");
    }
}

#[test]
fn sccache_is_installed_at_a_fixed_version() {
    let versioned = Regex::new(r"^version: v\d+\.\d+\.\d+$").unwrap();
    let mut installs = 0;
    for (name, workflow) in workflows() {
        let lines: Vec<&str> = code_lines(&workflow).collect();
        for (index, line) in lines.iter().enumerate() {
            if line.contains("uses: mozilla-actions/sccache-action@") {
                installs += 1;
                assert_eq!(lines[index + 1].trim(), "with:", "{name}");
                assert!(versioned.is_match(lines[index + 2].trim()), "{name}");
            }
        }
    }
    assert!(installs > 0, "no workflow installs sccache");
}

#[test]
fn the_aarch64_job_has_the_steps_of_the_parts_and_must_pass() {
    let workflow = read_workflow("ci.yml");
    let part = job(&workflow, &parts()[0]);
    let aarch64 = job(&workflow, "gate-aarch64");

    assert!(aarch64.contains(&"    runs-on: ubuntu-24.04-arm"));
    assert!(
        !aarch64
            .iter()
            .any(|line| line.contains("continue-on-error"))
    );
    let (steps_part, steps_aarch64) = (steps(&part), steps(&aarch64));
    let gate = format!("        run: {GATE}");
    assert_eq!(steps_aarch64.last(), Some(&gate.as_str()));
    let last = steps_part.len() - 1;
    assert_eq!(steps_aarch64[..last], steps_part[..last]);
    assert_eq!(steps_aarch64.len(), steps_part.len());
}

#[test]
fn the_release_is_created_only_for_a_tag() {
    let workflow = read_workflow("release.yml");
    let package = job(&workflow, "package");
    let create = package
        .iter()
        .position(|line| line.trim() == "- name: Create the release")
        .expect("a step that creates the release");
    let condition = package[create + 1].trim();
    assert!(
        condition.starts_with("if: ") && condition.contains("github.ref_type == 'tag'"),
        "{condition}"
    );
}

/// The step of a job named `name`: its lines from its `- name:` to the next step.
fn step<'a>(job: &[&'a str], name: &str) -> Vec<&'a str> {
    let start = job
        .iter()
        .position(|line| line.trim() == format!("- name: {name}"))
        .unwrap_or_else(|| panic!("no step `{name}`"));
    job[start..]
        .iter()
        .enumerate()
        .take_while(|(index, line)| *index == 0 || !line.trim_start().starts_with("- "))
        .map(|(_, line)| *line)
        .collect()
}

/// The release APK is built from the secrets when they exist, skipped with a notice when
/// they do not, and attached to the release with its checksum.
#[test]
fn the_release_attaches_the_apk_when_the_signing_secrets_exist() {
    let workflow = read_workflow("release.yml");
    let package = job(&workflow, "package");
    let apk = step(&package, "Build the Android APK");
    for variable in [
        "VKRGSS_ANDROID_KEYSTORE_PASSWORD",
        "VKRGSS_ANDROID_KEY_ALIAS",
        "VKRGSS_ANDROID_KEY_PASSWORD",
    ] {
        let line = format!("          {variable}: ${{{{ secrets.{variable} }}}}");
        assert!(apk.contains(&line.as_str()), "{variable}");
    }
    let text = apk.join("\n");
    assert!(
        text.contains("KEYSTORE_BASE64: ${{ secrets.VKRGSS_ANDROID_KEYSTORE_BASE64 }}"),
        "{text}"
    );
    let skip = text
        .find(r#"if [ -z "$KEYSTORE_BASE64" ]; then"#)
        .expect("a check for the secret");
    let notice = text
        .find("::notice::")
        .expect("a notice when it is missing");
    let build = text
        .find("cargo xtask apk --release")
        .expect("the release build");
    assert!(skip < notice && notice < build, "{text}");
    assert!(text[notice..build].contains("exit 0"), "{text}");
    // The passwords never appear in the script itself.
    assert!(!text.contains("--ks-pass"), "{text}");

    let create = step(&package, "Create the release").join("\n");
    assert!(
        create.contains(r#"apk="target/apk/vkrgss-$VERSION-android-arm64.apk""#),
        "{create}"
    );
    assert!(
        create.contains(r#"assets+=("$apk" "$apk.sha256")"#),
        "{create}"
    );
    assert!(create.contains(r#""${assets[@]}""#), "{create}");
    let position = |name: &str| {
        package
            .iter()
            .position(|line| line.trim() == format!("- name: {name}"))
            .unwrap()
    };
    assert!(position("Build the Android APK") < position("Create the release"));
}

#[test]
fn every_action_is_named_by_a_commit_hash() {
    let pinned = Regex::new(r"^-? ?uses: [\w.-]+/[\w./-]+@[0-9a-f]{40} # v\S+$").unwrap();
    for (name, workflow) in workflows() {
        for line in code_lines(&workflow).filter(|line| line.contains("uses:")) {
            assert!(pinned.is_match(line.trim()), "{name}: `{}`", line.trim());
        }
    }
}

#[test]
fn the_openspec_cli_is_installed_at_a_fixed_version() {
    let versioned = Regex::new(r"@fission-ai/openspec@\d+\.\d+\.\d+(\s|$)").unwrap();
    let mut installs = 0;
    for (name, workflow) in workflows() {
        for line in code_lines(&workflow).filter(|line| line.contains("@fission-ai/openspec")) {
            installs += 1;
            assert!(versioned.is_match(line), "{name}: `{}`", line.trim());
        }
    }
    assert!(installs > 0, "no workflow installs the OpenSpec CLI");
}

#[test]
fn a_run_is_cancelled_only_for_a_pull_request() {
    for (name, workflow) in workflows() {
        for line in code_lines(&workflow).filter(|line| line.contains("cancel-in-progress:")) {
            assert_eq!(
                line.trim(),
                "cancel-in-progress: ${{ github.event_name == 'pull_request' }}",
                "{name}"
            );
        }
    }
    assert!(read_workflow("ci.yml").contains("cancel-in-progress:"));
}

#[test]
fn every_job_that_builds_names_the_c_compiler() {
    for (name, workflow) in workflows() {
        let names = job_names(&workflow);
        assert!(!names.is_empty(), "{name} has no jobs");
        for job_name in names {
            let builds = steps(&job(&workflow, job_name))
                .iter()
                .any(|line| line.contains("cargo ") || line.contains("apt-get install"));
            if !builds {
                continue;
            }
            let lines = job(&workflow, job_name);
            let env = lines
                .iter()
                .position(|line| *line == "    env:")
                .unwrap_or_else(|| panic!("{name}: the job `{job_name}` has no `env`"));
            assert_eq!(
                lines.get(env + 1),
                Some(&"      CC: gcc-13"),
                "{name}: the job `{job_name}` does not set `CC`"
            );
            if let Some(install) = lines
                .iter()
                .position(|line| line.contains("apt-get install"))
            {
                assert!(
                    lines[install..]
                        .iter()
                        .take_while(|line| !line.trim().starts_with('-'))
                        .any(|line| line.split_whitespace().any(|word| word == "gcc-13")),
                    "{name}: the job `{job_name}` does not install `gcc-13`"
                );
            }
        }
    }
}

#[test]
fn the_nightly_workflow_runs_the_checks_outside_the_gate() {
    let workflow = read_workflow("nightly.yml");
    let on: Vec<&str> = workflow
        .lines()
        .skip_while(|line| *line != "on:")
        .skip(1)
        .take_while(|line| line.is_empty() || line.starts_with(' '))
        .collect();
    assert!(on.contains(&"  schedule:"), "{on:?}");
    assert!(on.contains(&"  workflow_dispatch:"), "{on:?}");
    assert!(!on.iter().any(|line| line.contains("pull_request")));

    let runs = |job_name: &str, command: &str| {
        let lines = job(&workflow, job_name);
        assert!(
            steps(&lines).join(" ").contains(command),
            "the job `{job_name}` does not run `{command}`"
        );
    };
    runs("audit", "cargo xtask audit");
    runs(
        "stress",
        "VK_LOADER_DRIVERS_SELECT='*lvp*' cargo xtask stress",
    );
    runs(
        "sanitized",
        "cargo test -p vkrgss-ruby-sys --locked --test sanitized -- \
         --ignored upstream_suites_pass_under_the_sanitizers",
    );
    runs(
        "sanitized192",
        "cargo test -p vkrgss-ruby192-sys --locked --test sanitized -- \
         --ignored upstream_suites_pass_under_the_sanitizers",
    );
    runs("fuzz", "cargo xtask fuzz ${{ matrix.target }} --seconds");
    assert!(
        job(&workflow, "fuzz")
            .contains(&"        target: [marshal, parse, regex, marshal192, parse192, regex192]"),
        "the job `fuzz` does not run each target"
    );
    let audit = job(&workflow, "audit");
    assert!(
        audit
            .iter()
            .any(|line| line.contains("cargo install --locked cargo-deny@0.20.2")),
        "cargo-deny is not installed at 0.20.2, which reads the current advisory database"
    );
}

#[test]
fn a_failed_fuzz_job_uploads_the_inputs_put_aside() {
    let workflow = read_workflow("nightly.yml");
    let fuzz = steps(&job(&workflow, "fuzz"));
    let upload = fuzz
        .iter()
        .position(|line| line.contains("uses: actions/upload-artifact@"))
        .expect("the job `fuzz` uploads an artifact");
    // The lines of the step: from its `- ` to the next.
    let start = fuzz[..=upload]
        .iter()
        .rposition(|line| line.trim_start().starts_with("- "))
        .unwrap();
    let step: Vec<&str> = fuzz[start..]
        .iter()
        .enumerate()
        .take_while(|(index, line)| *index == 0 || !line.trim_start().starts_with("- "))
        .map(|(_, line)| line.trim())
        .collect();
    assert!(step.contains(&"if: failure()"), "{step:?}");
    assert!(
        step.contains(&"path: target/tmp/fuzz/${{ matrix.target }}/artifacts/"),
        "{step:?}"
    );
    assert!(step.contains(&"retention-days: 30"), "{step:?}");
    // The step comes after the run of the target.
    let run = fuzz
        .iter()
        .position(|line| line.contains("cargo xtask fuzz"))
        .unwrap();
    assert!(run < start, "{fuzz:?}");
}

#[test]
fn a_failure_of_the_nightly_checks_opens_an_issue() {
    let workflow = read_workflow("nightly.yml");
    let issue = job(&workflow, "issue");
    let others: Vec<&str> = job_names(&workflow)
        .into_iter()
        .filter(|name| *name != "issue")
        .collect();
    assert_eq!(issue[0], format!("    needs: [{}]", others.join(", ")));
    assert!(issue.contains(&"    if: failure()"));
    let permissions = issue
        .iter()
        .position(|line| *line == "    permissions:")
        .expect("the job `issue` names its permissions");
    assert_eq!(issue[permissions + 1], "      issues: write");
    let text = issue.join("\n");
    for command in [
        "title=\"Nightly checks failed\"",
        "gh issue list --state open",
        "gh issue comment",
        "gh issue create --title \"$title\"",
        "/actions/runs/${{ github.run_id }}",
    ] {
        assert!(text.contains(command), "`{command}`");
    }

    // No other job, and not the workflow, may write.
    let writes: Vec<&str> = code_lines(&workflow)
        .filter(|line| line.contains(": write"))
        .collect();
    assert_eq!(writes, ["      issues: write"]);
}
