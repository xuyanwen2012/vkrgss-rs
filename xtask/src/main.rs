//! Repository automation for vkxp, run as `cargo xtask <command>`.
//!
//! - `gate`: the merge gate (formatting, lints, tests, OpenSpec validation).
//! - `deps`: the dependency-direction rules on their own.
//! - `extract-rpg <DLL> <OUTPUT>`: copies the built-in `RPG` scripts out of an RGSS DLL.
//! - `oracle <GAME_DIR> --frames N[,N...] [OPTIONS]`: compares frames of a game between the
//!   original runtime and vkxp.

#![forbid(unsafe_code)]

mod deps;
mod extract;
mod gate;
mod oracle;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str =
    "usage: cargo xtask <gate|deps|extract-rpg <DLL> <OUTPUT>|oracle <GAME_DIR> [OPTIONS]>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args[..] {
        ["gate"] => gate::run(&workspace_root()),
        ["deps"] => deps::run(&workspace_root()),
        ["extract-rpg", dll, output] => extract::run(Path::new(dll), Path::new(output)),
        ["oracle", ref arguments @ ..] => {
            let root = workspace_root();
            match oracle::parse(arguments, &root) {
                Ok(options) => oracle::run(&options, &root),
                Err(error) => {
                    eprintln!("{error:#}\n{}", oracle::USAGE);
                    return ExitCode::from(2);
                }
            }
        }
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
