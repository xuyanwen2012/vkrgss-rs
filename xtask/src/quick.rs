//! `cargo xtask gate --quick`: the checks of a small change while it is made. It runs the
//! format check and clippy of the gate, and the tests of the packages that the change touches
//! and of the packages that depend on them, with fewer cases of the property tests; it says
//! what it leaves out. The full gate stays the check before a change is done.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, ensure};
use serde_json::Value;

use crate::testrun::{Binary, PROPTEST_CASES};

/// The base the change is compared with when none is named.
pub(crate) const DEFAULT_BASE: &str = "origin/main";

/// What the step `test` of a quick run adds to the environment: fewer cases of the property
/// tests that take their count from it.
pub(crate) const QUICK_TEST_ENV: &[(&str, &str)] = &[(PROPTEST_CASES, "32")];

/// The steps of the gate that a quick run leaves out.
pub(crate) const LEFT_OUT_STEPS: &str = "test-unvalidated, doc, openspec";

/// Test targets, by package and name, that a quick run leaves out because they have no
/// smaller count of cases, with the reason it gives.
const LEFT_OUT_TARGETS: &[(&str, &str, &str)] = &[(
    "vkrgss-ruby192-sys",
    "upstream_stress",
    "the files of `test/ruby` under collector stress, which have no smaller count",
)];

/// Packages whose tests run the executables of other packages of the workspace, which no
/// manifest records as a dependency.
const RUNS_EXECUTABLES_OF: &[(&str, &[&str])] = &[
    ("vkrgss", &["vkrgss-guest-rgss12", "vkrgss-guest-rgss3"]),
    ("xtask", &["vkrgss", "vkrgss-guest-rgss3"]),
];

/// The files below these, and the documents at the top, are read only by the tests of
/// `xtask`; a change of another file outside every package tests every package.
const DOCUMENT_DIRS: &[&str] = &["docs/", "openspec/", ".github/"];

/// A package of the workspace.
#[derive(Debug)]
struct Package {
    name: String,
    /// Its directory, relative to the workspace.
    dir: PathBuf,
    manifest_path: PathBuf,
    /// The packages of the workspace it depends on, for its build, its library or its tests.
    dependencies: Vec<String>,
}

/// The files that a change touches and the packages of the workspace, from which the tests
/// of a quick run are chosen.
#[derive(Debug)]
pub(crate) struct Selection {
    /// Relative to the workspace.
    changed: Vec<PathBuf>,
    packages: Vec<Package>,
}

impl Selection {
    /// The files changed since the merge base of `HEAD` and `base`, committed or not, both
    /// names of a file that was moved, and the files that git does not track and does not
    /// ignore.
    pub(crate) fn new(root: &Path, base: &str) -> Result<Selection> {
        let merge_base = git(root, &["merge-base", base, "HEAD"]).with_context(|| {
            format!("gate --quick: no merge base with `{base}`; name one with --base")
        })?;
        let merge_base = merge_base.trim();
        let mut changed: BTreeSet<PathBuf> = BTreeSet::new();
        for list in [
            git(
                root,
                &["diff", "--name-only", "--no-renames", "-z", merge_base],
            )?,
            git(root, &["ls-files", "--others", "--exclude-standard", "-z"])?,
        ] {
            changed.extend(
                list.split('\0')
                    .filter(|name| !name.is_empty())
                    .map(PathBuf::from),
            );
        }
        eprintln!(
            "gate --quick: {} file(s) changed since {} (the merge base with {base})",
            changed.len(),
            &merge_base[..merge_base.len().min(12)]
        );
        Ok(Selection {
            changed: changed.into_iter().collect(),
            packages: packages(root)?,
        })
    }

    /// The binaries of the packages that the change touches and of those that depend on
    /// them, less [`LEFT_OUT_TARGETS`]; prints what it chose and what it left out. A package
    /// is touched by a file in its directory, or by a file that one of its binaries is
    /// compiled from, as rustc lists them beside it.
    pub(crate) fn choose(&self, binaries: Vec<Binary>, root: &Path) -> Vec<Binary> {
        let mut touched = BTreeSet::new();
        for file in &self.changed {
            touched.extend(self.owners(file));
        }
        for binary in &binaries {
            let sources = compiled_from(&binary.executable, root);
            if self.changed.iter().any(|file| sources.contains(file)) {
                touched.insert(self.package_of(binary).name.as_str());
            }
        }
        let chosen = self.with_dependents(&touched);
        let names = |set: &BTreeSet<&str>| set.iter().copied().collect::<Vec<_>>().join(", ");
        let dependents: BTreeSet<&str> = chosen.difference(&touched).copied().collect();
        eprintln!(
            "gate --quick: testing the packages it touches: {}; and those that depend on them: {}",
            or_none(&names(&touched)),
            or_none(&names(&dependents))
        );
        let mut kept = Vec::new();
        for binary in binaries {
            let package = self.package_of(&binary).name.as_str();
            if !chosen.contains(package) {
                continue;
            }
            match LEFT_OUT_TARGETS
                .iter()
                .find(|(name, target, _)| *name == package && *target == binary.name)
            {
                Some((_, target, reason)) => {
                    eprintln!("gate --quick: left out: test `{target}` of {package}, {reason}");
                }
                None => kept.push(binary),
            }
        }
        let (name, cases) = QUICK_TEST_ENV[0];
        eprintln!(
            "gate --quick: the property tests run with {name}={cases}; \
             the documentation tests of every package run"
        );
        kept
    }

    /// The packages that the file `file` touches by its place.
    fn owners(&self, file: &Path) -> Vec<&str> {
        let owner = self
            .packages
            .iter()
            .filter(|package| file.starts_with(&package.dir))
            .max_by_key(|package| package.dir.components().count());
        if let Some(package) = owner {
            return vec![package.name.as_str()];
        }
        let text = file.to_string_lossy();
        let document = DOCUMENT_DIRS.iter().any(|dir| text.starts_with(dir))
            || (file.components().count() == 1 && text.ends_with(".md"));
        match document {
            true => vec!["xtask"],
            false => self
                .packages
                .iter()
                .map(|package| package.name.as_str())
                .collect(),
        }
    }

    fn package_of(&self, binary: &Binary) -> &Package {
        self.packages
            .iter()
            .find(|package| package.manifest_path == binary.manifest_path)
            .expect("a test binary belongs to a package of the workspace")
    }

    /// `touched` and every package that depends on one of them, also by running its
    /// executable ([`RUNS_EXECUTABLES_OF`]).
    fn with_dependents<'a>(&'a self, touched: &BTreeSet<&'a str>) -> BTreeSet<&'a str> {
        let mut chosen = touched.clone();
        loop {
            let before = chosen.len();
            for package in &self.packages {
                let runs = RUNS_EXECUTABLES_OF
                    .iter()
                    .filter(|(name, _)| *name == package.name)
                    .flat_map(|(_, executables)| executables.iter().copied());
                let mut needs = package.dependencies.iter().map(String::as_str).chain(runs);
                if needs.any(|name| chosen.contains(name)) {
                    chosen.insert(package.name.as_str());
                }
            }
            if chosen.len() == before {
                return chosen;
            }
        }
    }
}

fn or_none(text: &str) -> &str {
    if text.is_empty() { "none" } else { text }
}

/// The packages of the workspace, from `cargo metadata`.
fn packages(root: &Path) -> Result<Vec<Package>> {
    let output = Command::new(crate::cargo())
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .current_dir(root)
        .output()
        .context("could not start cargo metadata")?;
    ensure!(
        output.status.success(),
        "cargo metadata ended with {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).context("cargo metadata")?;
    let packages = metadata["packages"].as_array().context("cargo metadata")?;
    let names: BTreeSet<&str> = packages
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect();
    packages
        .iter()
        .map(|package| {
            let manifest_path = PathBuf::from(
                package["manifest_path"]
                    .as_str()
                    .context("a package without a manifest")?,
            );
            let dir = manifest_path
                .parent()
                .and_then(|dir| dir.strip_prefix(root).ok())
                .context("a package outside the workspace")?
                .to_owned();
            let dependencies = package["dependencies"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|dependency| dependency["name"].as_str())
                .filter(|name| names.contains(name))
                .map(str::to_owned)
                .collect();
            Ok(Package {
                name: package["name"].as_str().unwrap_or_default().to_owned(),
                dir,
                manifest_path,
                dependencies,
            })
        })
        .collect()
}

/// The files, relative to the workspace, that rustc compiled the executable from, from the
/// file of dependencies it wrote beside it.
fn compiled_from(executable: &Path, root: &Path) -> BTreeSet<PathBuf> {
    let mut file = executable.as_os_str().to_owned();
    file.push(".d");
    std::fs::read_to_string(file)
        .map(|text| dependencies(&text, root))
        .unwrap_or_default()
}

/// The files that a file of dependencies of rustc names: each on a line of its own, ending in
/// a colon, after the rules.
fn dependencies(text: &str, root: &Path) -> BTreeSet<PathBuf> {
    text.lines()
        .filter_map(|line| line.strip_suffix(':'))
        .filter(|line| !line.contains(": "))
        .map(|line| {
            let path = PathBuf::from(line.replace("\\ ", " "));
            normal(path.strip_prefix(root).unwrap_or(&path))
        })
        .collect()
}

/// `path` without `.` and with each `..` taken back.
fn normal(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other),
        }
    }
    normal
}

/// The standard output of git with `args` in `root`.
fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("could not start git")?;
    ensure!(
        output.status.success(),
        "`git {}` ended with {}: {}",
        args.join(" "),
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8(output.stdout).context("git printed text that is not UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, dir: &str, dependencies: &[&str]) -> Package {
        Package {
            name: name.to_owned(),
            dir: PathBuf::from(dir),
            manifest_path: PathBuf::from(format!("/w/{dir}/Cargo.toml")),
            dependencies: dependencies.iter().map(|name| (*name).to_owned()).collect(),
        }
    }

    fn selection(changed: &[&str]) -> Selection {
        Selection {
            changed: changed.iter().map(PathBuf::from).collect(),
            packages: vec![
                package("vkrgss-proto", "crates/vkrgss-proto", &[]),
                package("vkrgss-core", "crates/vkrgss-core", &["vkrgss-proto"]),
                package("vkrgss-guest-rgss3", "crates/vkrgss-guest-rgss3", &[]),
                package("vkrgss", "crates/vkrgss", &["vkrgss-core"]),
                package("vkrgss-android", "crates/vkrgss-android", &[]),
                package("xtask", "xtask", &[]),
            ],
        }
    }

    fn chosen(changed: &[&str]) -> Vec<String> {
        let selection = selection(changed);
        let mut touched = BTreeSet::new();
        for file in &selection.changed {
            touched.extend(selection.owners(file));
        }
        let chosen = selection.with_dependents(&touched);
        chosen.into_iter().map(str::to_owned).collect()
    }

    #[test]
    fn a_change_tests_its_package_and_those_that_depend_on_it() {
        assert_eq!(
            chosen(&["crates/vkrgss-core/src/lib.rs"]),
            ["vkrgss", "vkrgss-core", "xtask"]
        );
        assert_eq!(
            chosen(&["crates/vkrgss-android/src/lib.rs"]),
            ["vkrgss-android"]
        );
        // The tests of `vkrgss` run the guest executable, which it does not depend on.
        assert_eq!(
            chosen(&["crates/vkrgss-guest-rgss3/src/main.rs"]),
            ["vkrgss", "vkrgss-guest-rgss3", "xtask"]
        );
    }

    #[test]
    fn a_document_tests_xtask_and_another_file_outside_the_packages_tests_all() {
        assert_eq!(chosen(&["docs/building.md", "AGENTS.md"]), ["xtask"]);
        assert_eq!(chosen(&["Cargo.lock"]).len(), 6);
        assert_eq!(chosen(&["third_party/ruby-1.8.7/gc.c"]).len(), 6);
    }

    #[test]
    fn the_files_of_a_binary_are_read_from_the_file_of_dependencies() {
        let text = "/w/target/debug/deps/prelude-1.d: crates/b3/tests/prelude.rs crates/b3/tests/../../b/tests/prelude.rs /w/Cargo.toml\n\
                    \n\
                    crates/b3/tests/prelude.rs:\n\
                    crates/b3/tests/../../b/tests/prelude.rs:\n\
                    crates/b3/tests/a\\ b.txt:\n\
                    /w/Cargo.toml:\n";
        let files = dependencies(text, Path::new("/w"));
        let expected: BTreeSet<PathBuf> = [
            "crates/b3/tests/prelude.rs",
            "crates/b/tests/prelude.rs",
            "crates/b3/tests/a b.txt",
            "Cargo.toml",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();
        assert_eq!(files, expected);
    }
}
