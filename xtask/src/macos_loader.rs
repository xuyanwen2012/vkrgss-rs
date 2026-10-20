//! The Vulkan loader of the macOS app: `cargo xtask macos-loader` builds the Khronos loader
//! from its source for the oldest macOS the app declares, because Homebrew's is built for the
//! macOS of the machine. Run by hand, on macOS only, not by the gate; `cargo xtask app` runs
//! it too.
//!
//! The sources of the loader and of the Vulkan headers are cloned by git at [`TAG`] into
//! `target/macos-loader/src/`, and each clone must be at its pinned commit ([`SOURCES`]); a
//! later version is a change of these constants. The headers are installed with their own
//! cmake, the loader is built against them with the deployment target
//! [`MINIMUM_MACOS`](crate::app::MINIMUM_MACOS) for arm64, and the library is copied to
//! `target/macos-loader/libvulkan.1.dylib` ([`library`]), whose install name the build makes
//! `@rpath/libvulkan.1.dylib`. A second run reuses the clones and the build directories.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::app::MINIMUM_MACOS;
use crate::macho::{self, Version};

/// The tag of both sources: the version of Homebrew's `vulkan-loader`.
const TAG: &str = "vulkan-sdk-1.4.363.0";

/// A source the loader is built from.
struct Source {
    /// Its directory below `target/macos-loader/src/`.
    name: &'static str,
    /// The repository it is cloned from.
    url: &'static str,
    /// The commit that [`TAG`] must name.
    commit: &'static str,
}

/// The headers, then the loader.
const SOURCES: [Source; 2] = [HEADERS, LOADER];

const HEADERS: Source = Source {
    name: "Vulkan-Headers",
    url: "https://github.com/KhronosGroup/Vulkan-Headers",
    commit: "6802bb4733b63ed5efd3adb308a6c885ef180ea1",
};

const LOADER: Source = Source {
    name: "Vulkan-Loader",
    url: "https://github.com/KhronosGroup/Vulkan-Loader",
    commit: "fa218055ff4948b159f31d43f800c808a13d51df",
};

/// The directory of the loader's build and its result below `root`.
fn dir(root: &Path) -> PathBuf {
    root.join("target/macos-loader")
}

/// The loader built below `root`.
pub(crate) fn library(root: &Path) -> PathBuf {
    dir(root).join("libvulkan.1.dylib")
}

/// `cargo xtask macos-loader`: builds the loader below `root`.
pub(crate) fn run(root: &Path) -> Result<()> {
    let library = build(root)?;
    eprintln!("macos-loader: wrote {}", library.display());
    Ok(())
}

/// Clones the sources below `root` when they are missing, checks their commits, builds the
/// loader and checks that it needs no newer macOS than the app declares. Returns
/// [`library`].
pub(crate) fn build(root: &Path) -> Result<PathBuf> {
    if !cfg!(target_os = "macos") {
        bail!("macos-loader: the loader of the macOS app is built on macOS only");
    }
    let dir = dir(root);
    let sources = dir.join("src");
    for source in &SOURCES {
        fetch(source, &sources)?;
    }
    let headers = dir.join("headers");
    let headers_build = dir.join("build/headers");
    let loader_build = dir.join("build/loader");
    let mut configure = Command::new("cmake");
    run_tool(
        configure
            .arg("-S")
            .arg(sources.join(HEADERS.name))
            .arg("-B")
            .arg(&headers_build)
            .args(["-G", "Unix Makefiles"])
            .arg(define("CMAKE_INSTALL_PREFIX", &headers)),
    )?;
    let mut install = Command::new("cmake");
    run_tool(install.arg("--install").arg(&headers_build))?;
    let mut configure = Command::new("cmake");
    run_tool(
        configure
            .arg("-S")
            .arg(sources.join(LOADER.name))
            .arg("-B")
            .arg(&loader_build)
            .args(["-G", "Unix Makefiles"])
            .args([
                "-DCMAKE_BUILD_TYPE=Release",
                &format!("-DCMAKE_OSX_DEPLOYMENT_TARGET={MINIMUM_MACOS}"),
                "-DCMAKE_OSX_ARCHITECTURES=arm64",
                "-DBUILD_TESTS=OFF",
            ])
            .arg(define("VULKAN_HEADERS_INSTALL_DIR", &headers))
            // The loader finds the headers through `find_package`, which would otherwise
            // also look under Homebrew's prefix.
            .arg(define("CMAKE_PREFIX_PATH", &headers)),
    )?;
    eprintln!(
        "macos-loader: building the loader in {}",
        loader_build.display()
    );
    let mut make = Command::new("cmake");
    run_tool(make.arg("--build").arg(&loader_build).arg("--parallel"))?;

    let built = loader_build.join("loader/libvulkan.1.dylib");
    let library = library(root);
    fs::copy(&built, &library).with_context(|| {
        format!(
            "macos-loader: could not copy {} to {}",
            built.display(),
            library.display()
        )
    })?;
    let bytes = fs::read(&library)
        .with_context(|| format!("macos-loader: could not read {}", library.display()))?;
    match macho::minimum_macos(&bytes)? {
        Some(minimum) if minimum <= Version::parse(MINIMUM_MACOS)? => Ok(library),
        Some(minimum) => bail!(
            "macos-loader: {} needs macOS {minimum}, not {MINIMUM_MACOS}",
            library.display()
        ),
        None => bail!("macos-loader: {} names no minimum macOS", library.display()),
    }
}

/// Clones `source` at [`TAG`] into its directory below `sources` unless it is there, and
/// checks that it is at its pinned commit.
fn fetch(source: &Source, sources: &Path) -> Result<()> {
    let clone = sources.join(source.name);
    if !clone.join(".git").exists() {
        if clone.exists() {
            fs::remove_dir_all(&clone)
                .with_context(|| format!("macos-loader: could not remove {}", clone.display()))?;
        }
        fs::create_dir_all(sources)
            .with_context(|| format!("macos-loader: could not make {}", sources.display()))?;
        eprintln!("macos-loader: cloning {} at {TAG}", source.url);
        let mut git = Command::new("git");
        run_tool(
            git.args(["clone", "--quiet", "--depth", "1", "--branch", TAG])
                .arg(source.url)
                .arg(&clone),
        )?;
    }
    let mut git = Command::new("git");
    let head = run_tool(git.arg("-C").arg(&clone).args(["rev-parse", "HEAD"]))?;
    check_commit(source, &clone, String::from_utf8_lossy(&head).trim())
}

/// Fails unless `head`, the commit of the clone `clone` of `source`, is its pinned commit.
fn check_commit(source: &Source, clone: &Path, head: &str) -> Result<()> {
    if head != source.commit {
        bail!(
            "macos-loader: {} is at commit {head}, but {TAG} of {} is pinned to {}; remove {} \
             to clone it again",
            clone.display(),
            source.url,
            source.commit,
            clone.display()
        );
    }
    Ok(())
}

/// The cmake argument that defines `name` as `path`.
fn define(name: &str, path: &Path) -> String {
    format!("-D{name}={}", path.display())
}

/// Runs `command` and returns what it printed; a tool that is not installed or fails is an
/// error that names the command and shows its error output.
fn run_tool(command: &mut Command) -> Result<Vec<u8>> {
    let name = command.get_program().to_string_lossy().into_owned();
    let output = command
        .output()
        .with_context(|| format!("macos-loader: could not start `{name}`"))?;
    if !output.status.success() {
        bail!(
            "macos-loader: `{command:?}` ended with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clone_at_another_commit_is_refused_and_named() {
        let clone = Path::new("target/macos-loader/src/Vulkan-Loader");
        check_commit(&LOADER, clone, LOADER.commit).unwrap();
        let error = check_commit(&LOADER, clone, "0123456789abcdef0123456789abcdef01234567")
            .unwrap_err()
            .to_string();
        for wanted in [
            "Vulkan-Loader is at commit 0123456789abcdef",
            LOADER.commit,
            TAG,
        ] {
            assert!(error.contains(wanted), "{wanted}: {error}");
        }
    }

    #[test]
    fn the_pinned_commits_are_full_hashes() {
        for source in &SOURCES {
            assert_eq!(source.commit.len(), 40, "{}", source.name);
            assert!(source.commit.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_command_says_that_the_loader_is_built_on_macos_only() {
        let root = tempfile::tempdir().unwrap();
        let error = build(root.path()).unwrap_err();
        assert!(error.to_string().contains("built on macOS only"), "{error}");
    }
}
