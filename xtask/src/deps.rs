//! Dependency-direction rules, checked over `cargo metadata`.
//!
//! Four rules hold for the workspace:
//!
//! 1. No guest crate reaches a host crate, directly or transitively.
//! 2. No host crate, and not the reference compositor, reaches a guest crate, directly or
//!    transitively.
//! 3. Everything `vkrgss-core` reaches is on its allow-list of pure crates.
//! 4. The guest role, `vkrgss-guest`, reaches no interpreter crate and neither `vkrgss-binding`
//!    nor `vkrgss-binding-rgss3`, directly or transitively: it links no interpreter.
//! 5. No package reaches both `vkrgss-ruby-sys` and `vkrgss-ruby192-sys`: the two interpreters
//!    define the same symbols and cannot be linked into one program. For this rule alone
//!    development dependencies count as well, because a test binary links them.
//!
//! Beside the rules, the twin packages that compile one source over the two interpreters,
//! `vkrgss-binding` and `vkrgss-binding-rgss3` for the RGSS classes and `vkrgss-guest-main` and
//! `vkrgss-guest-main-rgss3` for the main of a guest, declare the same dependencies apart from
//! the interpreter crate and the binding package, and every test file of the first of a pair
//! has its counterpart in the second.
//!
//! The `vkrgss` binary is on neither side: it is the one crate that may depend on both. The
//! guest executable `vkrgss-guest-rgss3` is a guest crate.
//!
//! The graph is taken with every feature of every workspace crate enabled, so a forbidden
//! dependency cannot hide behind an optional dependency or a feature that is off by default.
//!
//! For the first three rules only normal and build dependencies count. Dev-dependencies are
//! not linked into either side, so they do not cross the guest/host boundary.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// Crates that run on the script thread.
pub(crate) const GUEST_CRATES: &[&str] = &[
    "vkrgss-core",
    "vkrgss-image",
    "vkrgss-text",
    "vkrgss-vfs",
    "vkrgss-ruby-sys",
    "vkrgss-ruby192-sys",
    "vkrgss-binding",
    "vkrgss-binding-rgss3",
    "vkrgss-guest",
    "vkrgss-guest-main",
    "vkrgss-guest-main-rgss3",
    "vkrgss-guest-rgss3",
];

/// Crates that own the window, the GPU or the audio device.
pub(crate) const HOST_CRATES: &[&str] = &["vkrgss-vk", "vkrgss-audio", "vkrgss-host"];

/// Crates that must not reach a guest crate: the host crates and the reference compositor,
/// which runs on the host and in tests.
pub(crate) const HOST_SIDE_CRATES: &[&str] = &[
    "vkrgss-vk",
    "vkrgss-audio",
    "vkrgss-host",
    "vkrgss-compose-ref",
];

/// The crate of the guest role, which an executable of any interpreter links.
pub(crate) const GUEST_ROLE_CRATE: &str = "vkrgss-guest";

/// Crates that link an interpreter, which the guest role must not reach.
pub(crate) const INTERPRETER_CRATES: &[&str] = &[
    "vkrgss-ruby-sys",
    "vkrgss-ruby192-sys",
    "vkrgss-binding",
    "vkrgss-binding-rgss3",
];

/// The crate that must stay free of I/O, Ruby and Vulkan.
pub(crate) const CORE_CRATE: &str = "vkrgss-core";

/// Everything `vkrgss-core` may reach: the two shared crates, then pure crates (no I/O, no FFI
/// to Ruby, Vulkan, windowing or audio) that those need.
pub(crate) const CORE_ALLOW_LIST: &[&str] = &[
    "vkrgss-proto",
    "vkrgss-pixel",
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
    // Sines and cosines for sprite rotation that give the same bits on every platform. Pure
    // Rust without dependencies, I/O or FFI; reviewed in the design of add-core-scene.
    "libm",
];

/// Crate name to the names of its normal and build dependencies.
pub(crate) type Graph = BTreeMap<String, BTreeSet<String>>;

/// A broken dependency rule, with the dependency path that breaks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Violation {
    /// A guest crate reaches a host crate. The path starts at the guest crate and ends at the
    /// host crate.
    GuestReachesHost { path: Vec<String> },
    /// A host crate or the reference compositor reaches a guest crate. The path starts at the
    /// former and ends at the guest crate.
    HostReachesGuest { path: Vec<String> },
    /// `vkrgss-core` reaches a crate outside its allow-list. The path starts at `vkrgss-core` and
    /// ends at the offending dependency.
    CoreDependency { path: Vec<String> },
    /// `vkrgss-guest` reaches a crate that links an interpreter. The path starts at
    /// `vkrgss-guest` and ends at that crate.
    GuestRoleReachesInterpreter { path: Vec<String> },
    /// A package reaches both interpreters. Both paths start at the package; the first ends
    /// at `vkrgss-ruby-sys`, the second at `vkrgss-ruby192-sys`.
    BothInterpreters { paths: [Vec<String>; 2] },
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
            Violation::HostReachesGuest { path } => write!(
                f,
                "host-side crate `{}` reaches guest crate `{}`: {}",
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
            Violation::GuestRoleReachesInterpreter { path } => write!(
                f,
                "`{GUEST_ROLE_CRATE}` reaches `{}`, which links an interpreter: {}",
                path.last().map_or("", String::as_str),
                path.join(" -> ")
            ),
            Violation::BothInterpreters { paths } => write!(
                f,
                "`{}` reaches both interpreters, which cannot be linked into one program: \
                 {} and {}",
                paths[0].first().map_or("", String::as_str),
                paths[0].join(" -> "),
                paths[1].join(" -> ")
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

/// Checks every rule and returns every violation, in a stable order.
pub(crate) fn check(graph: &Graph) -> Vec<Violation> {
    let mut violations = Vec::new();
    for guest in GUEST_CRATES {
        let paths = paths_from(graph, guest);
        for host in HOST_CRATES {
            if let Some(path) = paths.get(*host) {
                violations.push(Violation::GuestReachesHost { path: path.clone() });
            }
        }
    }
    for host in HOST_SIDE_CRATES {
        let paths = paths_from(graph, host);
        for guest in GUEST_CRATES {
            if let Some(path) = paths.get(*guest) {
                violations.push(Violation::HostReachesGuest { path: path.clone() });
            }
        }
    }
    violations.extend(outside_allow_list(graph, CORE_CRATE));
    let paths = paths_from(graph, GUEST_ROLE_CRATE);
    for interpreter in INTERPRETER_CRATES {
        if let Some(path) = paths.get(*interpreter) {
            violations.push(Violation::GuestRoleReachesInterpreter { path: path.clone() });
        }
    }
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

/// The two crates that each build an interpreter, in the order of the paths of
/// [`Violation::BothInterpreters`].
pub(crate) const INTERPRETER_SYS_CRATES: [&str; 2] = ["vkrgss-ruby-sys", "vkrgss-ruby192-sys"];

/// Checks the rule on the two interpreters over a graph in which development dependencies
/// count (see [`linked_graph_from_metadata`]), and returns every violation, in a stable
/// order.
pub(crate) fn check_interpreters(linked: &Graph) -> Vec<Violation> {
    let mut violations = Vec::new();
    for package in linked.keys() {
        let paths = paths_from(linked, package);
        let path_to = |interpreter: &str| -> Option<Vec<String>> {
            if package == interpreter {
                Some(vec![package.clone()])
            } else {
                paths.get(interpreter).cloned()
            }
        };
        if let (Some(first), Some(second)) = (
            path_to(INTERPRETER_SYS_CRATES[0]),
            path_to(INTERPRETER_SYS_CRATES[1]),
        ) {
            violations.push(Violation::BothInterpreters {
                paths: [first, second],
            });
        }
    }
    violations
}

/// Builds the graph from the JSON printed by `cargo metadata --format-version 1`.
pub(crate) fn graph_from_metadata(metadata: &Value) -> Result<Graph> {
    graph_of_kinds(metadata, false)
}

/// As [`graph_from_metadata`], with development dependencies as edges too: what a test
/// binary of a package links.
pub(crate) fn linked_graph_from_metadata(metadata: &Value) -> Result<Graph> {
    graph_of_kinds(metadata, true)
}

fn graph_of_kinds(metadata: &Value, development: bool) -> Result<Graph> {
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
            if development || !dev_only {
                let pkg = dep["pkg"].as_str().context("dependency without `pkg`")?;
                edges.insert(name_of(pkg)?);
            }
        }
    }
    Ok(graph)
}

/// Runs `cargo metadata` for the workspace at `root`, with all features of its members
/// enabled, and builds its graph.
pub(crate) fn workspace_graph(root: &Path) -> Result<Graph> {
    graph_from_metadata(&workspace_metadata(root)?)
}

/// As [`workspace_graph`], with development dependencies as edges too.
pub(crate) fn linked_workspace_graph(root: &Path) -> Result<Graph> {
    linked_graph_from_metadata(&workspace_metadata(root)?)
}

fn workspace_metadata(root: &Path) -> Result<Value> {
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
    serde_json::from_slice(&output.stdout).context("invalid metadata JSON")
}

/// The `deps` command: prints every violation and fails if there is one.
pub(crate) fn run(root: &Path) -> Result<()> {
    let mut violations = check(&workspace_graph(root)?);
    violations.extend(check_interpreters(&linked_workspace_graph(root)?));
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
            ("vkrgss-binding", "vkrgss-core"),
            ("vkrgss-core", "vkrgss-proto"),
            ("vkrgss-core", "vkrgss-pixel"),
            ("vkrgss-proto", "serde"),
            ("vkrgss-host", "vkrgss-vk"),
            ("vkrgss-host", "vkrgss-compose-ref"),
            ("vkrgss-vk", "vkrgss-proto"),
            ("vkrgss-compose-ref", "vkrgss-pixel"),
        ]);
        assert_eq!(check(&graph), vec![]);
    }

    #[test]
    fn direct_guest_to_host_edge_is_reported() {
        let graph = graph(&[("vkrgss-vfs", "vkrgss-audio")]);
        assert_eq!(
            check(&graph),
            vec![Violation::GuestReachesHost {
                path: path(&["vkrgss-vfs", "vkrgss-audio"]),
            }]
        );
    }

    #[test]
    fn transitive_guest_to_host_path_is_reported_in_full() {
        let graph = graph(&[
            ("vkrgss-binding", "helper"),
            ("helper", "glue"),
            ("glue", "vkrgss-vk"),
        ]);
        let violations = check(&graph);
        assert_eq!(
            violations,
            vec![Violation::GuestReachesHost {
                path: path(&["vkrgss-binding", "helper", "glue", "vkrgss-vk"]),
            }]
        );
        assert_eq!(
            violations[0].to_string(),
            "guest crate `vkrgss-binding` reaches host crate `vkrgss-vk`: \
             vkrgss-binding -> helper -> glue -> vkrgss-vk"
        );
    }

    #[test]
    fn direct_host_to_guest_edge_is_reported() {
        for host in HOST_SIDE_CRATES {
            let graph = graph(&[(host, "vkrgss-image")]);
            let violations = check(&graph);
            assert_eq!(
                violations,
                vec![Violation::HostReachesGuest {
                    path: path(&[host, "vkrgss-image"]),
                }]
            );
            assert_eq!(
                violations[0].to_string(),
                format!(
                    "host-side crate `{host}` reaches guest crate `vkrgss-image`: \
                     {host} -> vkrgss-image"
                )
            );
        }
    }

    #[test]
    fn transitive_host_to_guest_path_is_reported_in_full() {
        let graph = graph(&[
            ("vkrgss-compose-ref", "helper"),
            ("helper", "glue"),
            ("glue", "vkrgss-vfs"),
        ]);
        let violations = check(&graph);
        assert_eq!(
            violations,
            vec![Violation::HostReachesGuest {
                path: path(&["vkrgss-compose-ref", "helper", "glue", "vkrgss-vfs"]),
            }]
        );
        assert_eq!(
            violations[0].to_string(),
            "host-side crate `vkrgss-compose-ref` reaches guest crate `vkrgss-vfs`: \
             vkrgss-compose-ref -> helper -> glue -> vkrgss-vfs"
        );
    }

    /// Every guest crate a host crate reaches is reported, each with its own shortest path.
    #[test]
    fn host_reaching_two_guest_crates_reports_both() {
        let graph = graph(&[
            ("vkrgss-host", "vkrgss-binding"),
            ("vkrgss-binding", "vkrgss-core"),
        ]);
        assert_eq!(
            check(&graph),
            vec![
                Violation::HostReachesGuest {
                    path: path(&["vkrgss-host", "vkrgss-binding", "vkrgss-core"]),
                },
                Violation::HostReachesGuest {
                    path: path(&["vkrgss-host", "vkrgss-binding"]),
                },
            ]
        );
    }

    /// The binary joins the two sides; neither rule starts from it or passes through it.
    #[test]
    fn binary_may_depend_on_both_sides() {
        let graph = graph(&[
            ("vkrgss", "vkrgss-binding"),
            ("vkrgss", "vkrgss-core"),
            ("vkrgss", "vkrgss-host"),
            ("vkrgss", "vkrgss-compose-ref"),
            ("vkrgss-binding", "vkrgss-core"),
            ("vkrgss-host", "vkrgss-vk"),
        ]);
        assert_eq!(check(&graph), vec![]);
    }

    #[test]
    fn metadata_dev_dependency_of_a_host_crate_on_a_guest_crate_is_allowed() {
        let metadata = serde_json::json!({
            "packages": [
                { "id": "vk 0.1", "name": "vkrgss-vk" },
                { "id": "ref 0.1", "name": "vkrgss-compose-ref" },
                { "id": "core 0.1", "name": "vkrgss-core" },
                { "id": "image 0.1", "name": "vkrgss-image" },
            ],
            "resolve": { "nodes": [
                { "id": "vk 0.1", "deps": [
                    { "pkg": "core 0.1", "dep_kinds": [{ "kind": "dev" }] },
                ] },
                { "id": "ref 0.1", "deps": [
                    { "pkg": "image 0.1", "dep_kinds": [{ "kind": "dev" }] },
                ] },
                { "id": "core 0.1", "deps": [] },
                { "id": "image 0.1", "deps": [] },
            ] },
        });
        let graph = graph_from_metadata(&metadata).unwrap();
        assert!(graph["vkrgss-vk"].is_empty());
        assert!(graph["vkrgss-compose-ref"].is_empty());
        assert_eq!(check(&graph), vec![]);

        // The same edge as a normal dependency is the violation.
        let mut linked = graph;
        linked
            .get_mut("vkrgss-vk")
            .unwrap()
            .insert("vkrgss-core".to_owned());
        assert_eq!(
            check(&linked),
            vec![Violation::HostReachesGuest {
                path: path(&["vkrgss-vk", "vkrgss-core"]),
            }]
        );
    }

    #[test]
    fn core_dependency_outside_allow_list_is_reported() {
        let graph = graph(&[("vkrgss-core", "vkrgss-proto"), ("vkrgss-proto", "libc")]);
        let violations = check(&graph);
        assert_eq!(
            violations,
            vec![Violation::CoreDependency {
                path: path(&["vkrgss-core", "vkrgss-proto", "libc"]),
            }]
        );
        assert_eq!(
            violations[0].to_string(),
            "`vkrgss-core` depends on `libc`, which is not on its allow-list: \
             vkrgss-core -> vkrgss-proto -> libc"
        );
    }

    #[test]
    fn core_reaching_host_breaks_two_rules() {
        let graph = graph(&[("vkrgss-core", "vkrgss-vk")]);
        assert_eq!(
            check(&graph),
            vec![
                Violation::GuestReachesHost {
                    path: path(&["vkrgss-core", "vkrgss-vk"]),
                },
                Violation::CoreDependency {
                    path: path(&["vkrgss-core", "vkrgss-vk"]),
                },
            ]
        );
    }

    /// Scenario "The guest role".
    #[test]
    fn the_guest_role_reaching_a_host_crate_is_reported() {
        let reaching = graph(&[("vkrgss-guest", "helper"), ("helper", "vkrgss-host")]);
        let violations = check(&reaching);
        assert_eq!(
            violations,
            vec![Violation::GuestReachesHost {
                path: path(&["vkrgss-guest", "helper", "vkrgss-host"]),
            }]
        );
        assert_eq!(
            violations[0].to_string(),
            "guest crate `vkrgss-guest` reaches host crate `vkrgss-host`: \
             vkrgss-guest -> helper -> vkrgss-host"
        );
        // And no host crate reaches it.
        let reached = graph(&[("vkrgss-host", "vkrgss-guest")]);
        assert_eq!(
            check(&reached),
            vec![Violation::HostReachesGuest {
                path: path(&["vkrgss-host", "vkrgss-guest"]),
            }]
        );
    }

    /// Scenario "Interpreter dependency is rejected".
    /// Scenario "The main of a guest".
    #[test]
    fn the_main_of_a_guest_reaching_a_host_crate_is_reported() {
        for host in HOST_CRATES {
            let reaching = graph(&[("vkrgss-guest-main", "helper"), ("helper", host)]);
            let violations = check(&reaching);
            assert_eq!(
                violations,
                vec![Violation::GuestReachesHost {
                    path: path(&["vkrgss-guest-main", "helper", host]),
                }]
            );
            assert_eq!(
                violations[0].to_string(),
                format!(
                    "guest crate `vkrgss-guest-main` reaches host crate `{host}`: \
                     vkrgss-guest-main -> helper -> {host}"
                )
            );

            let reached = graph(&[(host, "vkrgss-guest-main")]);
            assert_eq!(
                check(&reached),
                vec![Violation::HostReachesGuest {
                    path: path(&[host, "vkrgss-guest-main"]),
                }]
            );
        }
        // It is what links the interpreter, and the program links it beside the host.
        let allowed = graph(&[
            ("vkrgss-guest-main", "vkrgss-binding"),
            ("vkrgss-guest-main", "vkrgss-guest"),
            ("vkrgss", "vkrgss-guest-main"),
            ("vkrgss", "vkrgss-host"),
        ]);
        assert_eq!(check(&allowed), vec![]);
    }

    /// Scenario "The guest executable of RGSS3".
    #[test]
    fn the_guest_executable_reaching_a_host_crate_is_reported() {
        for host in HOST_CRATES {
            let reaching = graph(&[
                ("vkrgss-guest-rgss3", "vkrgss-guest-main-rgss3"),
                ("vkrgss-guest-main-rgss3", host),
            ]);
            let violations = check(&reaching);
            assert_eq!(
                violations,
                vec![
                    Violation::GuestReachesHost {
                        path: path(&["vkrgss-guest-main-rgss3", host]),
                    },
                    Violation::GuestReachesHost {
                        path: path(&["vkrgss-guest-rgss3", "vkrgss-guest-main-rgss3", host]),
                    },
                ]
            );
            assert_eq!(
                violations[1].to_string(),
                format!(
                    "guest crate `vkrgss-guest-rgss3` reaches host crate `{host}`: \
                     vkrgss-guest-rgss3 -> vkrgss-guest-main-rgss3 -> {host}"
                )
            );
        }
    }

    #[test]
    fn the_guest_role_reaching_an_interpreter_is_reported() {
        for interpreter in INTERPRETER_CRATES {
            let direct = graph(&[("vkrgss-guest", interpreter)]);
            let violations = check(&direct);
            assert_eq!(
                violations,
                vec![Violation::GuestRoleReachesInterpreter {
                    path: path(&["vkrgss-guest", interpreter]),
                }]
            );
            assert_eq!(
                violations[0].to_string(),
                format!(
                    "`vkrgss-guest` reaches `{interpreter}`, which links an interpreter: \
                     vkrgss-guest -> {interpreter}"
                )
            );
        }
        // The binding reaches the sys crate: both are reported, each with its own path.
        let through = graph(&[
            ("vkrgss-guest", "glue"),
            ("glue", "vkrgss-binding"),
            ("vkrgss-binding", "vkrgss-ruby-sys"),
        ]);
        assert_eq!(
            check(&through),
            vec![
                Violation::GuestRoleReachesInterpreter {
                    path: path(&["vkrgss-guest", "glue", "vkrgss-binding", "vkrgss-ruby-sys"]),
                },
                Violation::GuestRoleReachesInterpreter {
                    path: path(&["vkrgss-guest", "glue", "vkrgss-binding"]),
                },
            ]
        );
        // The executable links the role and an interpreter side by side.
        let beside = graph(&[("vkrgss", "vkrgss-guest"), ("vkrgss", "vkrgss-binding")]);
        assert_eq!(check(&beside), vec![]);
    }

    #[test]
    fn dependency_cycles_terminate() {
        let graph = graph(&[("vkrgss-text", "a"), ("a", "b"), ("b", "a")]);
        assert_eq!(check(&graph), vec![]);
    }

    #[test]
    fn metadata_dev_dependencies_are_ignored() {
        let metadata = serde_json::json!({
            "packages": [
                { "id": "core 0.1", "name": "vkrgss-core" },
                { "id": "host 0.1", "name": "vkrgss-host" },
                { "id": "proto 0.1", "name": "vkrgss-proto" },
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
            graph["vkrgss-core"],
            BTreeSet::from(["vkrgss-proto".to_owned()])
        );
        assert_eq!(check(&graph), vec![]);
    }

    #[test]
    fn metadata_build_dependency_on_host_is_reported() {
        let metadata = serde_json::json!({
            "packages": [
                { "id": "sys 0.1", "name": "vkrgss-ruby-sys" },
                { "id": "audio 0.1", "name": "vkrgss-audio" },
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
                path: path(&["vkrgss-ruby-sys", "vkrgss-audio"]),
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
    /// for both directions of the guest/host rule and for the core allow-list, through real
    /// `cargo metadata`.
    #[test]
    fn inactive_optional_forbidden_dependencies_are_reported() {
        let directory = tempfile::tempdir().unwrap();
        write_workspace(
            directory.path(),
            &[
                (
                    "vkrgss-image",
                    "[dependencies]\nvkrgss-vk = { path = \"../vkrgss-vk\", optional = true }\n",
                ),
                (
                    "vkrgss-text",
                    "[dependencies]\nglue = { path = \"../glue\" }\n\n\
                     [features]\nsound = [\"glue/sound\"]\n",
                ),
                (
                    "glue",
                    "[dependencies]\nvkrgss-audio = { path = \"../vkrgss-audio\", optional = true }\n\n\
                     [features]\nsound = [\"dep:vkrgss-audio\"]\n",
                ),
                (
                    "vkrgss-core",
                    "[dependencies]\nvkrgss-proto = { path = \"../vkrgss-proto\" }\n\
                     impure-io = { path = \"../impure-io\", optional = true }\n\n\
                     [features]\ndefault = []\nfiles = [\"dep:impure-io\"]\n",
                ),
                (
                    "vkrgss-host",
                    "[dependencies]\nvkrgss-vfs = { path = \"../vkrgss-vfs\", optional = true }\n",
                ),
                (
                    "vkrgss-compose-ref",
                    "[dependencies]\nglue = { path = \"../glue\" }\n\n\
                     [build-dependencies]\nvkrgss-vfs = { path = \"../vkrgss-vfs\" }\n",
                ),
                ("vkrgss-proto", ""),
                ("vkrgss-vk", ""),
                ("vkrgss-audio", ""),
                ("vkrgss-vfs", ""),
                ("impure-io", ""),
            ],
        );

        let graph = workspace_graph(directory.path()).unwrap();
        assert_eq!(
            check(&graph),
            vec![
                Violation::GuestReachesHost {
                    path: path(&["vkrgss-image", "vkrgss-vk"]),
                },
                Violation::GuestReachesHost {
                    path: path(&["vkrgss-text", "glue", "vkrgss-audio"]),
                },
                Violation::HostReachesGuest {
                    path: path(&["vkrgss-host", "vkrgss-vfs"]),
                },
                Violation::HostReachesGuest {
                    path: path(&["vkrgss-compose-ref", "vkrgss-vfs"]),
                },
                Violation::CoreDependency {
                    path: path(&["vkrgss-core", "impure-io"]),
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
                    "vkrgss-core",
                    "[dependencies]\nvkrgss-proto = { path = \"../vkrgss-proto\", optional = true }\n",
                ),
                (
                    "vkrgss-host",
                    "[dependencies]\nvkrgss-vk = { path = \"../vkrgss-vk\", optional = true }\n\n\
                     [dev-dependencies]\nvkrgss-core = { path = \"../vkrgss-core\" }\n",
                ),
                (
                    "vkrgss",
                    "[dependencies]\nvkrgss-core = { path = \"../vkrgss-core\" }\n\
                     vkrgss-host = { path = \"../vkrgss-host\" }\n",
                ),
                ("vkrgss-proto", ""),
                ("vkrgss-vk", ""),
            ],
        );
        let graph = workspace_graph(directory.path()).unwrap();
        assert!(graph["vkrgss-core"].contains("vkrgss-proto"));
        assert!(graph["vkrgss-host"].contains("vkrgss-vk"));
        assert!(
            !graph["vkrgss-host"].contains("vkrgss-core"),
            "development only"
        );
        assert!(graph["vkrgss"].contains("vkrgss-core") && graph["vkrgss"].contains("vkrgss-host"));
        assert_eq!(check(&graph), vec![]);
    }

    fn real_graph() -> Graph {
        workspace_graph(&crate::workspace_root()).expect("cargo metadata for the workspace")
    }

    #[test]
    fn real_workspace_contains_every_named_crate() {
        let graph = real_graph();
        for name in GUEST_CRATES
            .iter()
            .chain(HOST_SIDE_CRATES)
            .chain(&["vkrgss"])
        {
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

    /// No test binary and no executable of the workspace links both interpreters.
    #[test]
    fn real_workspace_links_one_interpreter_per_package() {
        let linked = linked_workspace_graph(&crate::workspace_root())
            .expect("cargo metadata for the workspace");
        for interpreter in INTERPRETER_SYS_CRATES {
            assert!(
                linked.contains_key(interpreter),
                "`{interpreter}` is missing"
            );
        }
        let violations = check_interpreters(&linked);
        let report: Vec<String> = violations.iter().map(Violation::to_string).collect();
        assert!(violations.is_empty(), "{}", report.join("\n"));
    }

    /// One interpreter through a normal dependency and the other through a development
    /// dependency: the package is named with both paths.
    #[test]
    fn a_package_that_reaches_both_interpreters_is_reported_with_both_paths() {
        let metadata = serde_json::json!({
            "packages": [
                { "id": "both 0.1", "name": "both" },
                { "id": "binding 0.1", "name": "vkrgss-binding" },
                { "id": "sys 0.1", "name": "vkrgss-ruby-sys" },
                { "id": "sys192 0.1", "name": "vkrgss-ruby192-sys" },
            ],
            "resolve": { "nodes": [
                { "id": "both 0.1", "deps": [
                    { "pkg": "binding 0.1", "dep_kinds": [{ "kind": null }] },
                    { "pkg": "sys192 0.1", "dep_kinds": [{ "kind": "dev" }] },
                ] },
                { "id": "binding 0.1", "deps": [
                    { "pkg": "sys 0.1", "dep_kinds": [{ "kind": null }] },
                ] },
                { "id": "sys 0.1", "deps": [] },
                { "id": "sys192 0.1", "deps": [] },
            ] },
        });
        let violations = check_interpreters(&linked_graph_from_metadata(&metadata).unwrap());
        assert_eq!(
            violations,
            vec![Violation::BothInterpreters {
                paths: [
                    path(&["both", "vkrgss-binding", "vkrgss-ruby-sys"]),
                    path(&["both", "vkrgss-ruby192-sys"]),
                ],
            }]
        );
        assert_eq!(
            violations[0].to_string(),
            "`both` reaches both interpreters, which cannot be linked into one program: \
             both -> vkrgss-binding -> vkrgss-ruby-sys and both -> vkrgss-ruby192-sys"
        );
        // Without the development dependency the graph of the other rules has no such
        // package, and neither interpreter alone is a violation.
        assert_eq!(
            check_interpreters(&graph_from_metadata(&metadata).unwrap()),
            vec![]
        );
    }

    /// One of two packages that compile one source: its name, the interpreter crate it is
    /// built over and the binding package it is built over.
    #[derive(Clone, Copy)]
    struct Twin {
        package: &'static str,
        interpreter: &'static str,
        binding: &'static str,
    }

    /// The two packages that compile the one source of the RGSS classes.
    const BINDING_PACKAGES: [Twin; 2] = [
        Twin {
            package: "vkrgss-binding",
            interpreter: "vkrgss-ruby-sys",
            binding: "vkrgss-binding",
        },
        Twin {
            package: "vkrgss-binding-rgss3",
            interpreter: "vkrgss-ruby192-sys",
            binding: "vkrgss-binding-rgss3",
        },
    ];

    /// The two packages that compile the one source of the main of a guest.
    const GUEST_MAIN_PACKAGES: [Twin; 2] = [
        Twin {
            package: "vkrgss-guest-main",
            interpreter: "vkrgss-ruby-sys",
            binding: "vkrgss-binding",
        },
        Twin {
            package: "vkrgss-guest-main-rgss3",
            interpreter: "vkrgss-ruby192-sys",
            binding: "vkrgss-binding-rgss3",
        },
    ];

    /// The pairs of twin packages.
    const TWIN_PACKAGES: [[Twin; 2]; 2] = [BINDING_PACKAGES, GUEST_MAIN_PACKAGES];

    /// The dependencies a package declares, each as one line of name, kind, features and
    /// whether it is optional, with the package's own name, its interpreter crate and its
    /// binding package replaced by placeholders.
    fn declared_dependencies(metadata: &Value, twin: Twin) -> Result<BTreeSet<String>> {
        let Twin {
            package,
            interpreter,
            binding,
        } = twin;
        let packages = metadata["packages"]
            .as_array()
            .context("metadata has no `packages` array")?;
        let found = packages
            .iter()
            .find(|candidate| candidate["name"] == package)
            .with_context(|| format!("`{package}` is missing from the workspace"))?;
        let dependencies = found["dependencies"]
            .as_array()
            .with_context(|| format!("`{package}` has no `dependencies` array"))?;
        let mut lines = BTreeSet::new();
        for dependency in dependencies {
            let name = dependency["name"]
                .as_str()
                .context("dependency without `name`")?;
            let name = if name == package {
                "<itself>"
            } else if name == interpreter {
                "<the interpreter crate>"
            } else if name == binding {
                "<the binding package>"
            } else {
                name
            };
            let kind = dependency["kind"].as_str().unwrap_or("normal");
            let mut features: Vec<&str> = dependency["features"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            features.sort_unstable();
            let optional = if dependency["optional"] == true {
                ", optional"
            } else {
                ""
            };
            lines.insert(format!(
                "{name} ({kind}{optional}, features [{}])",
                features.join(", ")
            ));
        }
        Ok(lines)
    }

    /// What two twin packages declare differently, apart from the interpreter crate and the
    /// binding package: one entry per dependency that one of them has and the other has not,
    /// or has otherwise.
    fn twin_manifest_differences(metadata: &Value, twins: [Twin; 2]) -> Result<Vec<String>> {
        let [first, second] = twins;
        let of_first = declared_dependencies(metadata, first)?;
        let of_second = declared_dependencies(metadata, second)?;
        let (first, second) = (first.package, second.package);
        let only = |package: &str, lines: &BTreeSet<String>, other: &BTreeSet<String>| {
            lines
                .difference(other)
                .map(|line| format!("only `{package}` declares {line}"))
                .collect::<Vec<String>>()
        };
        let mut differences = only(first, &of_first, &of_second);
        differences.extend(only(second, &of_second, &of_first));
        Ok(differences)
    }

    /// The names of the `.rs` files directly in `dir`, sorted; none when there is no `dir`.
    fn test_files(dir: &Path) -> Result<BTreeSet<String>> {
        let mut names = BTreeSet::new();
        if !dir.exists() {
            return Ok(names);
        }
        for entry in std::fs::read_dir(dir).with_context(|| format!("{}", dir.display()))? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if name.ends_with(".rs") {
                names.insert(name);
            }
        }
        Ok(names)
    }

    /// The test files of `tests` that have no file of the same name in `counterparts`.
    fn tests_without_counterpart(tests: &Path, counterparts: &Path) -> Result<Vec<String>> {
        let present = test_files(counterparts)?;
        Ok(test_files(tests)?
            .into_iter()
            .filter(|name| !present.contains(name))
            .collect())
    }

    /// A package that reaches both binding packages reaches both interpreters through them.
    #[test]
    fn a_package_that_reaches_both_binding_packages_is_reported_with_both_paths() {
        let graph = graph(&[
            ("both", "vkrgss-binding"),
            ("both", "vkrgss-binding-rgss3"),
            ("vkrgss-binding", "vkrgss-ruby-sys"),
            ("vkrgss-binding-rgss3", "vkrgss-ruby192-sys"),
        ]);
        let violations = check_interpreters(&graph);
        assert_eq!(
            violations,
            vec![Violation::BothInterpreters {
                paths: [
                    path(&["both", "vkrgss-binding", "vkrgss-ruby-sys"]),
                    path(&["both", "vkrgss-binding-rgss3", "vkrgss-ruby192-sys"]),
                ],
            }]
        );
        assert_eq!(
            violations[0].to_string(),
            "`both` reaches both interpreters, which cannot be linked into one program: \
             both -> vkrgss-binding -> vkrgss-ruby-sys and \
             both -> vkrgss-binding-rgss3 -> vkrgss-ruby192-sys"
        );
    }

    /// Each package that links an interpreter reaches its own and not the other,
    /// development dependencies counted. Scenario "One interpreter each".
    #[test]
    fn each_package_reaches_its_own_interpreter_only() {
        let linked = linked_workspace_graph(&crate::workspace_root())
            .expect("cargo metadata for the workspace");
        let [first, second] = INTERPRETER_SYS_CRATES;
        for (package, interpreter, other) in [
            ("vkrgss-binding", first, second),
            ("vkrgss-guest-main", first, second),
            ("vkrgss", first, second),
            ("vkrgss-binding-rgss3", second, first),
            ("vkrgss-guest-main-rgss3", second, first),
            ("vkrgss-guest-rgss3", second, first),
        ] {
            let paths = paths_from(&linked, package);
            assert!(
                paths.contains_key(interpreter),
                "`{package}` misses `{interpreter}`"
            );
            assert!(!paths.contains_key(other), "`{package}` reaches `{other}`");
        }
    }

    /// Scenario "Nothing of the host is linked": the guest executable reaches none of the
    /// host crates and none of the crates of the window, the device and the sound.
    #[test]
    fn the_guest_executable_links_nothing_of_the_host() {
        let paths = paths_from(&real_graph(), "vkrgss-guest-rgss3");
        for host in [
            "vkrgss-host",
            "vkrgss-vk",
            "vkrgss-audio",
            "ash",
            "winit",
            "cpal",
        ] {
            assert!(
                !paths.contains_key(host),
                "vkrgss-guest-rgss3 reaches {host}: {}",
                paths[host].join(" -> ")
            );
        }
        assert!(paths.contains_key("vkrgss-guest-main-rgss3"));
    }

    /// Scenario "Direct dependencies": the guest executable declares one package of the
    /// workspace, its development dependencies left out.
    #[test]
    fn the_guest_executable_depends_on_the_glue_alone() {
        let metadata = metadata_without_resolution();
        let packages = metadata["packages"].as_array().expect("the packages");
        let workspace: BTreeSet<&str> = packages
            .iter()
            .filter_map(|package| package["name"].as_str())
            .collect();
        let executable = packages
            .iter()
            .find(|package| package["name"] == "vkrgss-guest-rgss3")
            .expect("the guest executable is in the workspace");
        let declared: Vec<&str> = executable["dependencies"]
            .as_array()
            .expect("its dependencies")
            .iter()
            .filter(|dependency| dependency["kind"].as_str() != Some("dev"))
            .filter_map(|dependency| dependency["name"].as_str())
            .filter(|name| workspace.contains(name))
            .collect();
        assert_eq!(declared, ["vkrgss-guest-main-rgss3"]);
    }

    /// Scenario "One version": the program and the guest executable it starts state one
    /// version.
    #[test]
    fn the_program_and_the_guest_executable_have_one_version() {
        let metadata = metadata_without_resolution();
        let version = |name: &str| {
            metadata["packages"]
                .as_array()
                .expect("the packages")
                .iter()
                .find(|package| package["name"] == name)
                .map(|package| package["version"].clone())
                .unwrap_or_else(|| panic!("`{name}` is in the workspace"))
        };
        assert_eq!(version("vkrgss"), version("vkrgss-guest-rgss3"));
    }

    fn metadata_without_resolution() -> Value {
        let output = Command::new(crate::cargo())
            .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
            .current_dir(crate::workspace_root())
            .output()
            .expect("cargo metadata for the workspace");
        assert!(output.status.success());
        serde_json::from_slice(&output.stdout).expect("metadata JSON")
    }

    /// Twin packages declare the same dependencies apart from the interpreter crate and the
    /// binding package.
    #[test]
    fn the_twin_packages_declare_the_same_dependencies() {
        let metadata = metadata_without_resolution();
        for twins in TWIN_PACKAGES {
            let differences = twin_manifest_differences(&metadata, twins)
                .expect("both packages are in the workspace");
            assert!(differences.is_empty(), "{}", differences.join("\n"));
        }
    }

    #[test]
    fn a_dependency_of_one_twin_package_only_is_named() {
        let dependency = |name: &str, kind: Value, features: &[&str]| {
            serde_json::json!({
                "name": name, "kind": kind, "optional": false, "features": features,
            })
        };
        let metadata = serde_json::json!({ "packages": [
            { "name": "vkrgss-binding", "dependencies": [
                dependency("vkrgss-ruby-sys", Value::Null, &[]),
                dependency("vkrgss-core", Value::Null, &[]),
                dependency("vkrgss-binding", "dev".into(), &["testing"]),
            ] },
            { "name": "vkrgss-binding-rgss3", "dependencies": [
                dependency("vkrgss-ruby192-sys", Value::Null, &[]),
                dependency("vkrgss-core", Value::Null, &["extra"]),
                dependency("vkrgss-binding-rgss3", "dev".into(), &["testing"]),
                dependency("libc", Value::Null, &[]),
            ] },
        ] });
        assert_eq!(
            twin_manifest_differences(&metadata, BINDING_PACKAGES).unwrap(),
            [
                "only `vkrgss-binding` declares vkrgss-core (normal, features [])",
                "only `vkrgss-binding-rgss3` declares libc (normal, features [])",
                "only `vkrgss-binding-rgss3` declares vkrgss-core (normal, features [extra])",
            ]
        );

        // The main of a guest names the binding package of its interpreter.
        let metadata = serde_json::json!({ "packages": [
            { "name": "vkrgss-guest-main", "dependencies": [
                dependency("vkrgss-ruby-sys", Value::Null, &[]),
                dependency("vkrgss-binding", Value::Null, &[]),
                dependency("tempfile", "dev".into(), &[]),
            ] },
            { "name": "vkrgss-guest-main-rgss3", "dependencies": [
                dependency("vkrgss-ruby192-sys", Value::Null, &[]),
                dependency("vkrgss-binding-rgss3", Value::Null, &[]),
                dependency("libc", Value::Null, &[]),
            ] },
        ] });
        assert_eq!(
            twin_manifest_differences(&metadata, GUEST_MAIN_PACKAGES).unwrap(),
            [
                "only `vkrgss-guest-main` declares tempfile (dev, features [])",
                "only `vkrgss-guest-main-rgss3` declares libc (normal, features [])",
            ]
        );
    }

    /// Every test file of the first of twin packages is built a second time by the second.
    #[test]
    fn every_test_file_of_a_twin_package_has_its_counterpart() {
        let crates = crate::workspace_root().join("crates");
        for [first, second] in TWIN_PACKAGES {
            let missing = tests_without_counterpart(
                &crates.join(first.package).join("tests"),
                &crates.join(second.package).join("tests"),
            )
            .expect("the directories are read");
            assert!(
                missing.is_empty(),
                "crates/{}/tests has files without a counterpart in crates/{}/tests: {}",
                first.package,
                second.package,
                missing.join(", ")
            );
        }
    }

    /// An interpreter crate that depends on the other one reaches both.
    #[test]
    fn an_interpreter_that_reaches_the_other_is_reported() {
        let graph = graph(&[("vkrgss-ruby192-sys", "vkrgss-ruby-sys")]);
        assert_eq!(
            check_interpreters(&graph),
            vec![Violation::BothInterpreters {
                paths: [
                    path(&["vkrgss-ruby192-sys", "vkrgss-ruby-sys"]),
                    path(&["vkrgss-ruby192-sys"]),
                ],
            }]
        );
    }

    /// `vkrgss-core` is allowed to depend on the shared crates, so whatever they pull in has to
    /// be on the allow-list as well.
    #[test]
    fn shared_crates_stay_inside_the_core_allow_list() {
        let graph = real_graph();
        for shared in ["vkrgss-proto", "vkrgss-pixel"] {
            let violations = outside_allow_list(&graph, shared);
            let report: Vec<String> = violations.iter().map(Violation::to_string).collect();
            assert!(violations.is_empty(), "{}", report.join("\n"));
        }
    }

    /// The test doubles of `vkrgss-host` are behind its feature `testing`, which only its own
    /// development dependency on itself enables: the executable is built without it.
    #[test]
    fn the_executable_is_built_without_the_test_doubles_of_the_host() {
        let output = Command::new(crate::cargo())
            .args(["tree", "-p", "vkrgss", "-e", "features", "--locked"])
            .current_dir(crate::workspace_root())
            .output()
            .expect("cargo tree for the executable");
        assert!(output.status.success());
        let tree = String::from_utf8_lossy(&output.stdout);
        assert!(tree.contains("vkrgss-host feature \"default\""), "{tree}");
        assert!(!tree.contains("vkrgss-host feature \"testing\""), "{tree}");
    }

    /// The address every crate gives as its repository.
    const REPOSITORY: &str = "https://github.com/xuyanwen2012/vkrgss-rs";

    /// What a reader of a published project expects of every crate, and nothing that could
    /// be sent to a registry.
    #[test]
    fn every_crate_has_the_package_metadata() {
        let output = Command::new(crate::cargo())
            .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
            .current_dir(crate::workspace_root())
            .output()
            .expect("cargo metadata for the workspace");
        assert!(output.status.success());
        let metadata: Value = serde_json::from_slice(&output.stdout).expect("metadata JSON");
        let packages = metadata["packages"].as_array().expect("the packages");
        assert!(!packages.is_empty());
        for package in packages {
            let name = package["name"].as_str().expect("a name");
            let description = package["description"].as_str().unwrap_or_default();
            assert!(
                !description.trim().is_empty(),
                "`{name}` has no description"
            );
            assert_eq!(package["license"], "MIT", "the licence of `{name}`");
            assert_eq!(
                package["repository"], REPOSITORY,
                "the repository of `{name}`"
            );
            assert_eq!(
                package["version"],
                env!("CARGO_PKG_VERSION"),
                "the version of `{name}`"
            );
            assert_eq!(
                package["publish"],
                serde_json::json!([]),
                "`{name}` can be published"
            );
        }
    }
}
