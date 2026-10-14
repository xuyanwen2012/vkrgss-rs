//! The release package: the release executables, `vkxp` and the guest executable of games
//! of RGSS3 that it starts from beside itself, with the documents and licence texts that
//! accompany them, in one archive under `target/dist/`, and the archive's SHA-256 checksum
//! beside it. The archive is written by `tar` and the checksum by `sha256sum`.
//!
//! Both executables are built in one build with libcrypt linked statically
//! (`VKXP_STATIC_CRYPT=1`, read by the build scripts of `vkxp-ruby-sys` and
//! `vkxp-ruby192-sys`), because the shared library has a different name on Ubuntu and on
//! Fedora. The command looks for the static library before the build and reads the needed
//! libraries of each executable with `readelf` after it.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

/// The only target vkxp builds for.
const TARGET: &str = "x86_64-unknown-linux-gnu";

/// The arguments of the build, after the cargo executable.
const BUILD: [&str; 7] = [
    "build",
    "--release",
    "--locked",
    "-p",
    "vkxp",
    "-p",
    "vkxp-guest-rgss3",
];

/// The variable that makes the build scripts of the interpreter crates link libcrypt
/// statically.
const STATIC_CRYPT: (&str, &str) = ("VKXP_STATIC_CRYPT", "1");

/// The static library the build needs.
const CRYPT_ARCHIVE: &str = "libcrypt.a";

/// Where the build leaves the executable, below the workspace root.
const EXECUTABLE: &str = "target/release/vkxp";

/// Where the build leaves the guest executable of games of RGSS3, below the workspace root.
const GUEST_EXECUTABLE: &str = "target/release/vkxp-guest-rgss3";

/// The executables of the package.
const EXECUTABLES: [&str; 2] = [EXECUTABLE, GUEST_EXECUTABLE];

/// The files of the package: where each is below the workspace root and where it goes below
/// the directory of the archive. The licence texts are the ones the bundled sources ask to
/// accompany a binary.
const FILES: [(&str, &str); 14] = [
    (EXECUTABLE, "vkxp"),
    (GUEST_EXECUTABLE, "vkxp-guest-rgss3"),
    ("README.md", "README.md"),
    ("CHANGELOG.md", "CHANGELOG.md"),
    ("LICENSE", "LICENSE"),
    ("THIRD-PARTY.md", "THIRD-PARTY.md"),
    (
        "third_party/ruby-1.8.7/COPYING",
        "licenses/ruby-1.8.7/COPYING",
    ),
    ("third_party/ruby-1.8.7/GPL", "licenses/ruby-1.8.7/GPL"),
    ("third_party/ruby-1.8.7/LGPL", "licenses/ruby-1.8.7/LGPL"),
    ("third_party/ruby-1.8.7/LEGAL", "licenses/ruby-1.8.7/LEGAL"),
    (
        "third_party/ruby-1.9.2/COPYING",
        "licenses/ruby-1.9.2/COPYING",
    ),
    ("third_party/ruby-1.9.2/GPL", "licenses/ruby-1.9.2/GPL"),
    ("third_party/ruby-1.9.2/LEGAL", "licenses/ruby-1.9.2/LEGAL"),
    (
        "third_party/oniguruma-5.9.6/COPYING",
        "licenses/oniguruma-5.9.6/COPYING",
    ),
];

/// The name of the archive's directory, which is the archive's name without its extension.
/// The version is the workspace's, which this crate inherits.
fn package_name() -> String {
    format!("vkxp-{}-{TARGET}", env!("CARGO_PKG_VERSION"))
}

/// Builds the release executables through the cargo executable `cargo` and writes the
/// package under `target/dist/` of `root`.
pub(crate) fn run(cargo: &str, root: &Path) -> Result<()> {
    // The build takes minutes; a document that is missing is reported before it.
    for (source, _) in FILES
        .iter()
        .filter(|(source, _)| !EXECUTABLES.contains(source))
    {
        require(root, source)?;
    }
    let mut cc = Command::new("cc");
    let printed = tool("cc", cc.arg(format!("-print-file-name={CRYPT_ARCHIVE}")))?;
    require_static_crypt(&String::from_utf8_lossy(&printed))?;
    let command_line = format!("cargo {}", BUILD.join(" "));
    eprintln!("dist: running `{command_line}`");
    let status = Command::new(cargo)
        .args(BUILD)
        .env(STATIC_CRYPT.0, STATIC_CRYPT.1)
        .current_dir(root)
        .status()
        .with_context(|| format!("dist: could not start `{command_line}`"))?;
    if !status.success() {
        bail!("dist: `{command_line}` ended with {status}");
    }
    for executable in EXECUTABLES {
        // The messages of `readelf` are parsed, so they are not translated.
        let mut readelf = Command::new("readelf");
        readelf.arg("--dynamic").arg(root.join(executable));
        let dynamic = tool("readelf", readelf.env("LC_ALL", "C"))?;
        let needed = needed_libraries(&String::from_utf8_lossy(&dynamic));
        eprintln!("dist: `{executable}` needs {}", needed.join(", "));
        refuse_shared_crypt_in(executable, &needed)?;
    }
    let archive = package(root, &root.join("target/dist"))?;
    eprintln!("dist: wrote {} and its .sha256", archive.display());
    Ok(())
}

/// The file `source` below `root`, or an error that names it when it does not exist.
fn require(root: &Path, source: &str) -> Result<PathBuf> {
    let path = root.join(source);
    if !path.is_file() {
        bail!("dist: `{source}` is missing");
    }
    Ok(path)
}

/// Checks what `cc -print-file-name=libcrypt.a` printed: the path of the library when the
/// compiler finds it, and the bare name when it does not.
fn require_static_crypt(printed: &str) -> Result<()> {
    if Path::new(printed.trim()).is_absolute() {
        return Ok(());
    }
    bail!(
        "dist: the static library `{CRYPT_ARCHIVE}` is missing; it is in the package \
         `libcrypt-dev` on Debian and Ubuntu and `libxcrypt-static` on Fedora"
    );
}

/// The names of the needed libraries in a dynamic section as `readelf --dynamic` prints it:
/// `0x... (NEEDED)  Shared library: [libc.so.6]`.
fn needed_libraries(dynamic: &str) -> Vec<String> {
    dynamic
        .lines()
        .filter(|line| line.contains("(NEEDED)"))
        .filter_map(|line| {
            let (_, rest) = line.split_once('[')?;
            let (name, _) = rest.split_once(']')?;
            Some(name.to_owned())
        })
        .collect()
}

/// Fails when one of the needed libraries is a libcrypt, and names it.
fn refuse_shared_crypt(needed: &[String]) -> Result<()> {
    if let Some(name) = needed.iter().find(|name| name.starts_with("libcrypt.so")) {
        bail!(
            "dist: the executable needs `{name}` at run time; libcrypt must be linked statically"
        );
    }
    Ok(())
}

/// [`refuse_shared_crypt`] for the executable `executable`, which the error names.
fn refuse_shared_crypt_in(executable: &str, needed: &[String]) -> Result<()> {
    refuse_shared_crypt(needed).map_err(|error| anyhow!("{error} (`{executable}`)"))
}

/// Copies the files of the package from `root` into a directory below `dist`, packs that
/// directory and writes the checksum. Returns the path of the archive.
fn package(root: &Path, dist: &Path) -> Result<PathBuf> {
    let name = package_name();
    let staged = dist.join(&name);
    if staged.exists() {
        fs::remove_dir_all(&staged)
            .with_context(|| format!("dist: could not remove {}", staged.display()))?;
    }
    for (source, destination) in FILES {
        let from = require(root, source)?;
        let to = staged.join(destination);
        let write = || -> std::io::Result<()> {
            fs::create_dir_all(to.parent().expect("a file below the package directory"))?;
            fs::copy(&from, &to)?;
            let mode = if EXECUTABLES.contains(&source) {
                0o755
            } else {
                0o644
            };
            fs::set_permissions(&to, fs::Permissions::from_mode(mode))
        };
        write().with_context(|| format!("dist: could not write {}", to.display()))?;
    }

    // Sorted entries with a fixed time, owner and mode, and no time in the gzip header: two
    // runs on the same source differ only in the executable.
    let archive_name = format!("{name}.tar.gz");
    let mut tar = Command::new("tar");
    tar.args([
        "--create",
        "--file",
        &archive_name,
        "--sort=name",
        "--mtime=@0",
        "--owner=0",
        "--group=0",
        "--numeric-owner",
        "--mode=u=rwX,go=rX",
        "--use-compress-program=gzip -n",
        &name,
    ]);
    tool("tar", tar.current_dir(dist))?;

    let mut sha256sum = Command::new("sha256sum");
    let checksum = tool("sha256sum", sha256sum.arg(&archive_name).current_dir(dist))?;
    let checksum_file = dist.join(format!("{archive_name}.sha256"));
    fs::write(&checksum_file, checksum)
        .with_context(|| format!("dist: could not write {}", checksum_file.display()))?;
    Ok(dist.join(archive_name))
}

/// Runs the tool `name` and returns what it printed; a tool that is not installed or fails
/// is an error that names it.
fn tool(name: &str, command: &mut Command) -> Result<Vec<u8>> {
    let output = command
        .output()
        .with_context(|| format!("dist: could not start `{name}`"))?;
    if !output.status.success() {
        bail!(
            "dist: `{name}` ended with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory with every file of the package, each holding its own path.
    fn source_tree() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for (source, _) in FILES {
            let path = root.path().join(source);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, source).unwrap();
        }
        root
    }

    /// The entries of the archive as `tar` lists them: mode and name.
    fn entries(archive: &Path) -> Vec<(String, String)> {
        let mut tar = Command::new("tar");
        let listing = tool("tar", tar.arg("-tvzf").arg(archive)).unwrap();
        String::from_utf8(listing)
            .unwrap()
            .lines()
            .map(|line| {
                let mut words = line.split_whitespace();
                let mode = words.next().unwrap().to_owned();
                (mode, words.next_back().unwrap().to_owned())
            })
            .collect()
    }

    #[test]
    fn the_archive_holds_one_directory_with_the_files_of_the_list() {
        let root = source_tree();
        let dist = root.path().join("target/dist");
        let archive = package(root.path(), &dist).unwrap();

        let name = format!(
            "vkxp-{}-x86_64-unknown-linux-gnu",
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(archive, dist.join(format!("{name}.tar.gz")));
        let expected = [
            ("drwxr-xr-x", ""),
            ("-rw-r--r--", "CHANGELOG.md"),
            ("-rw-r--r--", "LICENSE"),
            ("-rw-r--r--", "README.md"),
            ("-rw-r--r--", "THIRD-PARTY.md"),
            ("drwxr-xr-x", "licenses/"),
            ("drwxr-xr-x", "licenses/oniguruma-5.9.6/"),
            ("-rw-r--r--", "licenses/oniguruma-5.9.6/COPYING"),
            ("drwxr-xr-x", "licenses/ruby-1.8.7/"),
            ("-rw-r--r--", "licenses/ruby-1.8.7/COPYING"),
            ("-rw-r--r--", "licenses/ruby-1.8.7/GPL"),
            ("-rw-r--r--", "licenses/ruby-1.8.7/LEGAL"),
            ("-rw-r--r--", "licenses/ruby-1.8.7/LGPL"),
            ("drwxr-xr-x", "licenses/ruby-1.9.2/"),
            ("-rw-r--r--", "licenses/ruby-1.9.2/COPYING"),
            ("-rw-r--r--", "licenses/ruby-1.9.2/GPL"),
            ("-rw-r--r--", "licenses/ruby-1.9.2/LEGAL"),
            ("-rwxr-xr-x", "vkxp"),
            ("-rwxr-xr-x", "vkxp-guest-rgss3"),
        ]
        .map(|(mode, path)| (mode.to_owned(), format!("{name}/{path}")));
        assert_eq!(entries(&archive), expected);
    }

    #[test]
    fn the_checksum_file_is_accepted_by_sha256sum_and_a_second_run_gives_the_same_archive() {
        let root = source_tree();
        let dist = root.path().join("target/dist");
        let archive = package(root.path(), &dist).unwrap();
        let first = fs::read(&archive).unwrap();

        let mut check = Command::new("sha256sum");
        let checksum_file = format!("{}.tar.gz.sha256", package_name());
        tool(
            "sha256sum",
            check.args(["-c", &checksum_file]).current_dir(&dist),
        )
        .unwrap();

        package(root.path(), &dist).unwrap();
        assert_eq!(fs::read(&archive).unwrap(), first);
    }

    #[test]
    fn a_missing_file_fails_the_command_with_its_name() {
        for (missing, _) in FILES {
            let root = source_tree();
            fs::remove_file(root.path().join(missing)).unwrap();
            let dist = root.path().join("target/dist");
            let error = package(root.path(), &dist).unwrap_err().to_string();
            assert_eq!(error, format!("dist: `{missing}` is missing"));
        }
    }

    #[test]
    fn a_needed_libcrypt_fails_the_command_with_its_name() {
        let dynamic = "
Dynamic section at offset 0x1f3a2d0 contains 33 entries:
  Tag        Type                         Name/Value
 0x0000000000000001 (NEEDED)             Shared library: [libcrypt.so.1]
 0x0000000000000001 (NEEDED)             Shared library: [libm.so.6]
 0x0000000000000001 (NEEDED)             Shared library: [libc.so.6]
 0x000000000000000c (INIT)               0x1a9000
";
        let needed = needed_libraries(dynamic);
        assert_eq!(needed, ["libcrypt.so.1", "libm.so.6", "libc.so.6"]);
        let error = refuse_shared_crypt(&needed).unwrap_err().to_string();
        assert_eq!(
            error,
            "dist: the executable needs `libcrypt.so.1` at run time; libcrypt must be linked \
             statically"
        );
        refuse_shared_crypt(&needed[1..]).unwrap();
    }

    /// Scenario "Shared libcrypt in the guest executable".
    #[test]
    fn a_needed_libcrypt_of_the_guest_executable_is_named_with_it() {
        let needed = ["libcrypt.so.1".to_owned(), "libc.so.6".to_owned()];
        let error = refuse_shared_crypt_in(GUEST_EXECUTABLE, &needed)
            .unwrap_err()
            .to_string();
        assert!(error.contains("libcrypt.so.1"), "{error}");
        assert!(error.contains("vkxp-guest-rgss3"), "{error}");
        refuse_shared_crypt_in(GUEST_EXECUTABLE, &needed[1..]).unwrap();
    }

    /// Scenario "Missing guest executable".
    #[test]
    fn a_tree_without_the_guest_executable_fails_naming_it() {
        let root = source_tree();
        fs::remove_file(root.path().join(GUEST_EXECUTABLE)).unwrap();
        let dist = root.path().join("target/dist");
        let error = package(root.path(), &dist).unwrap_err().to_string();
        assert_eq!(error, "dist: `target/release/vkxp-guest-rgss3` is missing");
    }

    #[test]
    fn a_missing_static_library_is_named_with_its_packages() {
        // The compiler prints the bare name when it does not find the file.
        let error = require_static_crypt("libcrypt.a\n").unwrap_err();
        assert_eq!(
            error.to_string(),
            "dist: the static library `libcrypt.a` is missing; it is in the package \
             `libcrypt-dev` on Debian and Ubuntu and `libxcrypt-static` on Fedora"
        );
        require_static_crypt("/usr/lib/x86_64-linux-gnu/libcrypt.a\n").unwrap();
    }

    #[test]
    fn a_missing_document_is_reported_before_the_build() {
        let root = source_tree();
        fs::remove_file(root.path().join("LICENSE")).unwrap();
        // A cargo that would fail differently if it were started.
        let error = run("/nonexistent/cargo", root.path()).unwrap_err();
        assert_eq!(error.to_string(), "dist: `LICENSE` is missing");
    }
}
