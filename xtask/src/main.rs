//! Repository automation for vkrgss, run as `cargo xtask <command>`.
//!
//! - `gate [--job <NAME>]`: the merge gate (formatting, lints, tests, documentation, OpenSpec
//!   validation), or one named part of it, as a job of the CI workflow runs it.
//! - `gate --quick [--base <REV>]`: the format check, the lints and the tests of the packages
//!   that the change since the merge base with `origin/main`, or with `REV`, touches, and of
//!   those that depend on them, with fewer cases of the property tests. For the work on a
//!   change; the full gate stays the check before it is done.
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
//! - `android-test <CRATE> [--device <SERIAL>]`: cross-builds the tests of an interpreter
//!   crate (`vkrgss-ruby-sys`, `vkrgss-ruby192-sys`) for Android and runs them on a phone over
//!   adb. Not a step of the gate.
//! - `apk [--release]`: builds the debug APK of the Android app under `target/apk/`, or the
//!   release APK signed with the key the environment names. Not a step of the gate.
//! - `android-run [--validation] <SERIAL> <GAME_DIR>`: installs the APK on a phone, runs a
//!   game on it, under the Vulkan validation layer with `--validation`, and pulls the app's
//!   log and dumped frames. Not a step of the gate.
//! - `dist`: builds the release executable and packs it with the licence texts into an archive
//!   with a checksum under `target/dist/`.
//! - `app`: on macOS, builds the release executables and assembles the app `vkrgss.app`, with
//!   the Vulkan loader and MoltenVK, and a zip of it under `target/app/`. Not a step of the
//!   gate.
//! - `macos-loader`: on macOS, builds the Vulkan loader of the app from its source for the
//!   oldest macOS the app declares, under `target/macos-loader/`. Not a step of the gate.
//! - `extract-rpg <DLL> <OUTPUT>`: copies the built-in scripts out of an RGSS DLL, or out of
//!   the image of one as it lies in the memory of the running original.
//! - `oracle <GAME_DIR> --frames N[,N...] [OPTIONS]`: compares frames of a game between the
//!   original runtime and vkrgss.
//! - `pack-rgssad <GAME_DIR>`: packs the `Data` and `Graphics` folders of a copy of a game
//!   into `Game.rgssad`.
//! - `xarch run|android|compare|baseline|check ...`: runs the comparison games in vkrgss
//!   alone on this machine or on an Android phone, there also under the Vulkan validation
//!   layer, and compares the frames and saves of two such runs; records a run as the baseline
//!   and checks the current tree against it. Not a step of the gate.

#![forbid(unsafe_code)]

mod android;
mod apk;
mod app;
mod audit;
mod deps;
mod dist;
mod extract;
mod fuzz;
mod gate;
mod macho;
mod macos_loader;
mod oracle;
mod quick;
mod rgss3a;
mod rgssad;
mod stress;
mod testrun;
mod xarch;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <gate [--job <NAME>|--quick [--base <REV>]]|deps|audit|stress|fuzz <TARGET> [--seconds N]|\
                     android-test <CRATE> [--device <SERIAL>]|apk [--release]|android-run [--validation] <SERIAL> <GAME_DIR>|dist|app|macos-loader|\
                     extract-rpg <DLL> <OUTPUT>|oracle <GAME_DIR> [OPTIONS]|pack-rgssad <GAME_DIR>|\
                     xarch run|android|compare|baseline|check ...>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args[..] {
        ["gate"] => gate::run(&workspace_root()),
        ["gate", "--job", name] => gate::run_part(name, &workspace_root()),
        ["gate", "--quick"] => gate::run_quick(&workspace_root(), quick::DEFAULT_BASE),
        ["gate", "--quick", "--base", base] => gate::run_quick(&workspace_root(), base),
        ["deps"] => deps::run(&workspace_root()),
        ["audit"] => audit::run(&cargo(), &workspace_root()),
        ["stress"] => stress::run(&workspace_root()),
        ["dist"] => dist::run(&cargo(), &workspace_root()),
        ["app"] => app::run(&cargo(), &workspace_root()),
        ["macos-loader"] => macos_loader::run(&workspace_root()),
        ["extract-rpg", dll, output] => extract::run(Path::new(dll), Path::new(output)),
        ["pack-rgssad", game] => rgssad::pack(Path::new(game)),
        ["fuzz", ref arguments @ ..] => match fuzz::parse(arguments) {
            Ok(options) => fuzz::run(&options, &workspace_root()),
            Err(error) => {
                eprintln!("{error:#}\n{}", fuzz::USAGE);
                return ExitCode::from(2);
            }
        },
        ["android-test", ref arguments @ ..] => match android::parse(arguments) {
            Ok(options) => android::run(&options, &workspace_root()),
            Err(error) => {
                eprintln!("{error:#}\n{}", android::USAGE);
                return ExitCode::from(2);
            }
        },
        ["apk"] => apk::build(&cargo(), &workspace_root(), apk::Flavor::Debug).map(|_| ()),
        ["apk", "--release"] => {
            apk::build(&cargo(), &workspace_root(), apk::Flavor::Release).map(|_| ())
        }
        ["android-run", device, game] => {
            apk::run(&cargo(), &workspace_root(), device, Path::new(game), false)
        }
        ["android-run", "--validation", device, game] => {
            apk::run(&cargo(), &workspace_root(), device, Path::new(game), true)
        }
        ["apk" | "android-run", ..] => {
            eprintln!("{}", apk::USAGE);
            return ExitCode::from(2);
        }
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
        ["xarch", ref arguments @ ..] => match xarch::parse(arguments) {
            Ok(command) => xarch::run(&command, &cargo(), &workspace_root()),
            Err(error) => {
                eprintln!("{error:#}\n{}", xarch::USAGE);
                return ExitCode::from(2);
            }
        },
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

/// Whether the file system of `dir` folds case, as that of macOS does by default: a name that
/// differs from that of a file only in case names that file.
#[cfg(test)]
fn folds_case(dir: &Path) -> bool {
    let probe = dir.join("case-probe");
    std::fs::write(&probe, b"").expect("the probe is written");
    let folds = dir.join("CASE-PROBE").exists();
    std::fs::remove_file(probe).expect("the probe is removed");
    folds
}

/// Whether `dir` lists an entry of exactly the name `name`: on a file system that folds case,
/// a path that differs from that of an entry only in case exists too.
#[cfg(test)]
fn has_entry(dir: &Path, name: &str) -> bool {
    std::fs::read_dir(dir)
        .expect("the directory is listed")
        .any(|entry| entry.expect("an entry").file_name() == name)
}

/// The cargo executable that is running us, so nested calls use the same toolchain.
fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}
