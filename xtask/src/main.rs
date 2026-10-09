//! Repository automation for vkxp, run as `cargo xtask <command>`.
//!
//! - `gate`: the merge gate (formatting, lints, tests, OpenSpec validation).
//! - `deps`: the dependency-direction rules on their own.

#![forbid(unsafe_code)]

mod deps;
mod gate;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <gate|deps>";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let result = match (args.next().as_deref(), args.next()) {
        (Some("gate"), None) => gate::run(&workspace_root()),
        (Some("deps"), None) => deps::run(&workspace_root()),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

/// The workspace root, which is the parent of this crate's directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace root")
        .to_path_buf()
}

/// The cargo executable that is running us, so nested calls use the same toolchain.
fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}
