//! Repository automation for vkrgss, run as `cargo xtask <command>`.
//!
//! - `gate`: the merge gate (formatting, lints, tests, documentation, OpenSpec validation).
//! - `deps`: the dependency-direction rules on their own.
//! - `audit`: checks the dependencies against security advisories and the licence allow-list
//!   with `cargo deny`. Not a step of the gate.
//! - `stress`: runs the upstream Ruby suites in the sanitized build, the tests of the binding
//!   and the tests of the executable that start a guest, each under collector stress. Not a
//!   step of the gate.
//! - `fuzz <target> [--seconds N]`: builds one of the fuzz targets of the embedded
//!   interpreters (`marshal`, `parse`, `regex` of Ruby 1.8.7, `marshal192`, `parse192`,
//!   `regex192` of Ruby 1.9.2) with libFuzzer and the sanitizers and runs it. Not a step of
//!   the gate.
//! - `dist`: builds the release executable and packs it with the licence texts into an archive
//!   with a checksum under `target/dist/`.
//! - `extract-rpg <DLL> <OUTPUT>`: copies the built-in scripts out of an RGSS DLL, or out of
//!   the image of one as it lies in the memory of the running original.
//! - `oracle <GAME_DIR> --frames N[,N...] [OPTIONS]`: compares frames of a game between the
//!   original runtime and vkrgss.
//! - `pack-rgssad <GAME_DIR>`: packs the `Data` and `Graphics` folders of a copy of a game
//!   into `Game.rgssad`.

#![forbid(unsafe_code)]

mod audit;
mod deps;
mod dist;
mod extract;
mod fuzz;
mod gate;
mod oracle;
mod rgss3a;
mod rgssad;
mod stress;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <gate|deps|audit|stress|fuzz <TARGET> [--seconds N]|dist|\
                     extract-rpg <DLL> <OUTPUT>|oracle <GAME_DIR> [OPTIONS]|pack-rgssad <GAME_DIR>>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args[..] {
        ["gate"] => gate::run(&workspace_root()),
        ["deps"] => deps::run(&workspace_root()),
        ["audit"] => audit::run(&cargo(), &workspace_root()),
        ["stress"] => stress::run(&workspace_root()),
        ["dist"] => dist::run(&cargo(), &workspace_root()),
        ["extract-rpg", dll, output] => extract::run(Path::new(dll), Path::new(output)),
        ["pack-rgssad", game] => rgssad::pack(Path::new(game)),
        ["fuzz", ref arguments @ ..] => match fuzz::parse(arguments) {
            Ok(options) => fuzz::run(&options, &workspace_root()),
            Err(error) => {
                eprintln!("{error:#}\n{}", fuzz::USAGE);
                return ExitCode::from(2);
            }
        },
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
