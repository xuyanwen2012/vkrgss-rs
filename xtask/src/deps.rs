//! Dependency-direction rules, checked over `cargo metadata`.
//!
//! Two rules hold for the workspace:
//!
//! 1. No guest crate reaches a host crate, directly or transitively.
//! 2. Everything `vkxp-core` reaches is on its allow-list of pure crates.
//!
//! The graph is taken with every feature of every workspace crate enabled, so a forbidden
//! dependency cannot hide behind an optional dependency or a feature that is off by default.
//!
//! Only normal and build dependencies count. Dev-dependencies are not linked into the guest, so
//! they do not cross the guest/host boundary.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// Crates that run on the script thread.
pub const GUEST_CRATES: &[&str] = &[
    "vkxp-core",
    "vkxp-image",
    "vkxp-text",
    "vkxp-vfs",
    "vkxp-ruby-sys",
    "vkxp-binding",
];

/// Crates that own the window, the GPU or the audio device.
pub const HOST_CRATES: &[&str] = &["vkxp-vk", "vkxp-audio", "vkxp-host"];

/// The crate that must stay free of I/O, Ruby and Vulkan.
pub const CORE_CRATE: &str = "vkxp-core";

/// Everything `vkxp-core` may reach: the two shared crates, then pure crates (no I/O, no FFI
/// to Ruby, Vulkan, windowing or audio) that those need.
pub const CORE_ALLOW_LIST: &[&str] = &[
    "vkxp-proto",
    "vkxp-pixel",
    // Serialization of protocol types.
    "serde",
    "serde_core",
    "serde_derive",
    "postcard",
    "cobs",
    // Not built: `cargo metadata` lists it because postcard's `alloc` feature names it as a
    // weak dependency feature. It is a no_std trait crate in any case.
    "embedded-io",
    // Error types.
    "thiserror",
    "thiserror-impl",
    // Proc-macro support for the derives above.
    "proc-macro2",
    "quote",
    "syn",
    "unicode-ident",
];

/// Crate name to the names of its normal and build dependencies.
pub type Graph = BTreeMap<String, BTreeSet<String>>;

/// A broken dependency rule, with the dependency path that breaks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// A guest crate reaches a host crate. The path starts at the guest crate and ends at the
    /// host crate.
    GuestReachesHost { path: Vec<String> },
    /// `vkxp-core` reaches a crate outside its allow-list. The path starts at `vkxp-core` and
    /// ends at the offending dependency.
    CoreDependency { path: Vec<String> },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Violation::GuestReachesHost { path } => write!(
                f,
                "guest crate `{}` reaches host crate `{}`: {}",
                path.first().map_or("", String::as_str),
                path.last().map_or("", String::as_str),
                path.join(" -> ")
            ),
            Violation::CoreDependency { path } => write!(
                f,
                "`{CORE_CRATE}` depends on `{}`, which is not on its allow-list: {}",
                path.last().map_or("", String::as_str),
                path.join(" -> ")
            ),
        }
    }
}

/// Shortest dependency path from `start` to every crate it reaches, `start` excluded.
fn paths_from(graph: &Graph, start: &str) -> BTreeMap<String, Vec<String>> {
    let mut paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut queue = VecDeque::from([vec![start.to_owned()]]);
    while let Some(path) = queue.pop_front() {
        let current = path.last().expect("paths are never empty");
        for next in graph.get(current).into_iter().flatten() {
            if next == start || paths.contains_key(next) {
                continue;
            }
            let mut longer = path.clone();
            longer.push(next.clone());
            paths.insert(next.clone(), longer.clone());
            queue.push_back(longer);
        }
    }
    paths
}

/// Checks both rules and returns every violation, in a stable order.
pub fn check(graph: &Graph) -> Vec<Violation> {
    let mut violations = Vec::new();
    for guest in GUEST_CRATES {
        let paths = paths_from(graph, guest);
        for host in HOST_CRATES {
            if let Some(path) = paths.get(*host) {
                violations.push(Violation::GuestReachesHost { path: path.clone() });
            }
        }
    }
    violations.extend(outside_allow_list(graph, CORE_CRATE));
    violations
}

/// Violations for every crate that `start` reaches outside the core allow-list.
fn outside_allow_list(graph: &Graph, start: &str) -> Vec<Violation> {
    paths_from(graph, start)
        .into_iter()
        .filter(|(name, _)| !CORE_ALLOW_LIST.contains(&name.as_str()))
        .map(|(_, path)| Violation::CoreDependency { path })
        .collect()
}

/// Builds the graph from the JSON printed by `cargo metadata --format-version 1`.
pub fn graph_from_metadata(metadata: &Value) -> Result<Graph> {
    let packages = metadata["packages"]
        .as_array()
        .context("metadata has no `packages` array")?;
    let mut names = BTreeMap::new();
    for package in packages {
        let id = package["id"].as_str().context("package without `id`")?;
        let name = package["name"].as_str().context("package without `name`")?;
        names.insert(id, name);
    }
    let name_of = |id: &str| -> Result<String> {
        let name = names
            .get(id)
            .with_context(|| format!("unknown package id `{id}`"))?;
        Ok((*name).to_owned())
    };

    let nodes = metadata["resolve"]["nodes"]
        .as_array()
        .context("metadata has no `resolve.nodes` array; was it produced with --no-deps?")?;
    let mut graph = Graph::new();
    for node in nodes {
        let id = node["id"].as_str().context("resolve node without `id`")?;
        let edges = graph.entry(name_of(id)?).or_default();
        for dep in node["deps"].as_array().into_iter().flatten() {
            let kinds = dep["dep_kinds"].as_array();
            let dev_only = kinds.is_some_and(|kinds| {
                !kinds.is_empty() && kinds.iter().all(|kind| kind["kind"] == "dev")
            });
            if !dev_only {
                let pkg = dep["pkg"].as_str().context("dependency without `pkg`")?;
                edges.insert(name_of(pkg)?);
            }
        }
    }
    Ok(graph)
}

/// Runs `cargo metadata` for the workspace at `root`, with all features of its members
/// enabled, and builds its graph.
pub fn workspace_graph(root: &Path) -> Result<Graph> {
    let output = Command::new(crate::cargo())
        .args(["metadata", "--format-version", "1", "--all-features"])
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .output()
        .context("could not start `cargo metadata`")?;
    if !output.status.success() {
        bail!(
            "`cargo metadata` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata = serde_json::from_slice(&output.stdout).context("invalid metadata JSON")?;
    graph_from_metadata(&metadata)
}

/// The `deps` command: prints every violation and fails if there is one.
pub fn run(root: &Path) -> Result<()> {
    let violations = check(&workspace_graph(root)?);
    for violation in &violations {
        eprintln!("deps: {violation}");
    }
    if !violations.is_empty() {
        bail!("deps: {} dependency rule violation(s)", violations.len());
    }
    eprintln!("deps: dependency rules hold");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(&str, &str)]) -> Graph {
        let mut graph = Graph::new();
        for (from, to) in edges {
            graph
                .entry((*from).to_owned())
                .or_default()
                .insert((*to).to_owned());
            graph.entry((*to).to_owned()).or_default();
        }
        graph
    }

    fn path(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn allowed_graph_has_no_violations() {
        let graph = graph(&[
            ("vkxp-binding", "vkxp-core"),
            ("vkxp-core", "vkxp-proto"),
            ("vkxp-core", "vkxp-pixel"),
            ("vkxp-proto", "serde"),
            ("vkxp-host", "vkxp-vk"),
            ("vkxp-host", "vkxp-core"),
            ("vkxp-vk", "vkxp-proto"),
        ]);
        assert_eq!(check(&graph), vec![]);
    }

    #[test]
    fn direct_guest_to_host_edge_is_reported() {
        let graph = graph(&[("vkxp-vfs", "vkxp-audio")]);
        assert_eq!(
            check(&graph),
            vec![Violation::GuestReachesHost {
                path: path(&["vkxp-vfs", "vkxp-audio"]),
            }]
        );
    }

    #[test]
    fn transitive_guest_to_host_path_is_reported_in_full() {
        let graph = graph(&[
            ("vkxp-binding", "helper"),
            ("helper", "glue"),
            ("glue", "vkxp-vk"),
        ]);
        let violations = check(&graph);
        assert_eq!(
            violations,
            vec![Violation::GuestReachesHost {
                path: path(&["vkxp-binding", "helper", "glue", "vkxp-vk"]),
            }]
        );
        assert_eq!(
            violations[0].to_string(),
            "guest crate `vkxp-binding` reaches host crate `vkxp-vk`: \
             vkxp-binding -> helper -> glue -> vkxp-vk"
        );
    }

    #[test]
    fn host_reaching_guest_is_allowed() {
        let graph = graph(&[("vkxp-host", "vkxp-binding"), ("vkxp-binding", "vkxp-core")]);
        assert_eq!(check(&graph), vec![]);
    }

    #[test]
    fn core_dependency_outside_allow_list_is_reported() {
        let graph = graph(&[("vkxp-core", "vkxp-proto"), ("vkxp-proto", "libc")]);
        let violations = check(&graph);
        assert_eq!(
            violations,
            vec![Violation::CoreDependency {
                path: path(&["vkxp-core", "vkxp-proto", "libc"]),
            }]
        );
        assert_eq!(
            violations[0].to_string(),
            "`vkxp-core` depends on `libc`, which is not on its allow-list: \
             vkxp-core -> vkxp-proto -> libc"
        );
    }

    #[test]
    fn core_reaching_host_breaks_both_rules() {
        let graph = graph(&[("vkxp-core", "vkxp-vk")]);
        assert_eq!(
            check(&graph),
            vec![
                Violation::GuestReachesHost {
                    path: path(&["vkxp-core", "vkxp-vk"]),
                },
                Violation::CoreDependency {
                    path: path(&["vkxp-core", "vkxp-vk"]),
                },
            ]
        );
    }

    #[test]
    fn dependency_cycles_terminate() {
        let graph = graph(&[("vkxp-text", "a"), ("a", "b"), ("b", "a")]);
        assert_eq!(check(&graph), vec![]);
    }

    #[test]
    fn metadata_dev_dependencies_are_ignored() {
        let metadata = serde_json::json!({
            "packages": [
                { "id": "core 0.1", "name": "vkxp-core" },
                { "id": "host 0.1", "name": "vkxp-host" },
                { "id": "proto 0.1", "name": "vkxp-proto" },
            ],
            "resolve": { "nodes": [
                { "id": "core 0.1", "deps": [
                    { "pkg": "host 0.1", "dep_kinds": [{ "kind": "dev" }] },
                    { "pkg": "proto 0.1", "dep_kinds": [{ "kind": null }, { "kind": "dev" }] },
                ] },
                { "id": "host 0.1", "deps": [] },
                { "id": "proto 0.1", "deps": [] },
            ] },
        });
        let graph = graph_from_metadata(&metadata).unwrap();
        assert_eq!(
            graph["vkxp-core"],
            BTreeSet::from(["vkxp-proto".to_owned()])
        );
        assert_eq!(check(&graph), vec![]);
    }

    #[test]
    fn metadata_build_dependency_on_host_is_reported() {
        let metadata = serde_json::json!({
            "packages": [
                { "id": "sys 0.1", "name": "vkxp-ruby-sys" },
                { "id": "audio 0.1", "name": "vkxp-audio" },
            ],
            "resolve": { "nodes": [
                { "id": "sys 0.1", "deps": [
                    { "pkg": "audio 0.1", "dep_kinds": [{ "kind": "build" }] },
                ] },
                { "id": "audio 0.1", "deps": [] },
            ] },
        });
        let graph = graph_from_metadata(&metadata).unwrap();
        assert_eq!(
            check(&graph),
            vec![Violation::GuestReachesHost {
                path: path(&["vkxp-ruby-sys", "vkxp-audio"]),
            }]
        );
    }

    /// Writes a throwaway workspace: each entry is a crate name and the body of its manifest
    /// after the `[package]` table.
    fn write_workspace(root: &Path, crates: &[(&str, &str)]) {
        let members: Vec<String> = crates
            .iter()
            .map(|(name, _)| format!("\"{name}\""))
            .collect();
        let manifest = format!(
            "[workspace]\nresolver = \"2\"\nmembers = [{}]\n",
            members.join(", ")
        );
        std::fs::write(root.join("Cargo.toml"), manifest).unwrap();
        for (name, body) in crates {
            let directory = root.join(name);
            std::fs::create_dir_all(directory.join("src")).unwrap();
            std::fs::write(directory.join("src/lib.rs"), "").unwrap();
            let manifest = format!(
                "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n{body}"
            );
            std::fs::write(directory.join("Cargo.toml"), manifest).unwrap();
        }
    }

    /// Forbidden dependencies that are optional and switched off by default are still found,
    /// for the guest/host rule and for the core allow-list, through real `cargo metadata`.
    #[test]
    fn inactive_optional_forbidden_dependencies_are_reported() {
        let directory = tempfile::tempdir().unwrap();
        write_workspace(
            directory.path(),
            &[
                (
                    "vkxp-image",
                    "[dependencies]\nvkxp-vk = { path = \"../vkxp-vk\", optional = true }\n",
                ),
                (
                    "vkxp-text",
                    "[dependencies]\nglue = { path = \"../glue\" }\n\n\
                     [features]\nsound = [\"glue/sound\"]\n",
                ),
                (
                    "glue",
                    "[dependencies]\nvkxp-audio = { path = \"../vkxp-audio\", optional = true }\n\n\
                     [features]\nsound = [\"dep:vkxp-audio\"]\n",
                ),
                (
                    "vkxp-core",
                    "[dependencies]\nvkxp-proto = { path = \"../vkxp-proto\" }\n\
                     impure-io = { path = \"../impure-io\", optional = true }\n\n\
                     [features]\ndefault = []\nfiles = [\"dep:impure-io\"]\n",
                ),
                ("vkxp-proto", ""),
                ("vkxp-vk", ""),
                ("vkxp-audio", ""),
                ("impure-io", ""),
            ],
        );

        let graph = workspace_graph(directory.path()).unwrap();
        assert_eq!(
            check(&graph),
            vec![
                Violation::GuestReachesHost {
                    path: path(&["vkxp-image", "vkxp-vk"]),
                },
                Violation::GuestReachesHost {
                    path: path(&["vkxp-text", "glue", "vkxp-audio"]),
                },
                Violation::CoreDependency {
                    path: path(&["vkxp-core", "impure-io"]),
                },
            ]
        );
    }

    /// The same throwaway layout without the forbidden edges passes, so the test above fails
    /// for the right reason.
    #[test]
    fn optional_allowed_dependencies_pass() {
        let directory = tempfile::tempdir().unwrap();
        write_workspace(
            directory.path(),
            &[
                (
                    "vkxp-core",
                    "[dependencies]\nvkxp-proto = { path = \"../vkxp-proto\", optional = true }\n",
                ),
                (
                    "vkxp-host",
                    "[dependencies]\nvkxp-core = { path = \"../vkxp-core\", optional = true }\n",
                ),
                ("vkxp-proto", ""),
            ],
        );
        let graph = workspace_graph(directory.path()).unwrap();
        assert!(graph["vkxp-core"].contains("vkxp-proto"));
        assert!(graph["vkxp-host"].contains("vkxp-core"));
        assert_eq!(check(&graph), vec![]);
    }

    fn real_graph() -> Graph {
        workspace_graph(&crate::workspace_root()).expect("cargo metadata for the workspace")
    }

    #[test]
    fn real_workspace_contains_every_named_crate() {
        let graph = real_graph();
        for name in GUEST_CRATES.iter().chain(HOST_CRATES) {
            assert!(
                graph.contains_key(*name),
                "`{name}` is missing from the workspace"
            );
        }
    }

    #[test]
    fn real_workspace_obeys_the_dependency_rules() {
        let violations = check(&real_graph());
        let report: Vec<String> = violations.iter().map(Violation::to_string).collect();
        assert!(violations.is_empty(), "{}", report.join("\n"));
    }

    /// `vkxp-core` is allowed to depend on the shared crates, so whatever they pull in has to
    /// be on the allow-list as well.
    #[test]
    fn shared_crates_stay_inside_the_core_allow_list() {
        let graph = real_graph();
        for shared in ["vkxp-proto", "vkxp-pixel"] {
            let violations = outside_allow_list(&graph, shared);
            let report: Vec<String> = violations.iter().map(Violation::to_string).collect();
            assert!(violations.is_empty(), "{}", report.join("\n"));
        }
    }
}
