//! The macOS app: `cargo xtask app` builds the release executables of the package of `dist`,
//! `vkrgss` and the guest executables it starts from beside itself, and assembles
//! `target/app/vkrgss.app` from them, with the Vulkan loader and MoltenVK of Homebrew, so that
//! the app runs on a Mac without either installed and without `DYLD_*` variables. Run by
//! hand, on macOS only, not by the gate.
//!
//! - `Contents/MacOS/`: the three executables.
//! - `Contents/Frameworks/`: Homebrew's `libvulkan.1.dylib` and `libMoltenVK.dylib`, each
//!   with the install name `@rpath/<name>` (`install_name_tool -id`). `vkrgss-vk` loads the
//!   loader of the bundle by its path; the executables link neither.
//! - `Contents/Resources/vulkan/icd.d/MoltenVK_icd.json`: Homebrew's manifest of MoltenVK with
//!   a `library_path` relative to the manifest ([`ICD_LIBRARY_PATH`]). `vkrgss-vk` points the
//!   loader at it.
//! - `Contents/Resources/`: the documents and licence texts of the package of `dist`, and the
//!   licence texts of MoltenVK and the loader.
//! - `Contents/Info.plist` ([`info_plist`]); no icon.
//!
//! The app is signed ad hoc (`codesign --sign - --deep`), which the changed install names
//! need on Apple silicon, and `otool -L` of every executable and library of the bundle must
//! name nothing under `/opt/homebrew`. A zip of the app, written by `ditto`, lies beside it.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::apk::PACKAGE;
use crate::dist::{BUILD, EXECUTABLES, FILES};

/// The target the app is built for.
const TARGET: &str = "aarch64-apple-darwin";

/// Where Homebrew installs on Apple silicon.
const HOMEBREW: &str = "/opt/homebrew";

/// The libraries of the bundle: where each is below Homebrew's prefix, and its name in
/// `Contents/Frameworks/`.
const LIBRARIES: [(&str, &str); 2] = [
    ("lib/libvulkan.1.dylib", "libvulkan.1.dylib"),
    ("lib/libMoltenVK.dylib", "libMoltenVK.dylib"),
];

/// Homebrew's manifest of MoltenVK, below Homebrew's prefix.
const HOMEBREW_ICD: &str = "etc/vulkan/icd.d/MoltenVK_icd.json";

/// The manifest of MoltenVK below the bundle's directory, where `vkrgss-vk` looks for it.
const ICD: &str = "Contents/Resources/vulkan/icd.d/MoltenVK_icd.json";

/// The library of the manifest, relative to the manifest's directory.
const ICD_LIBRARY_PATH: &str = "../../../Frameworks/libMoltenVK.dylib";

/// The licence texts of the libraries: where each is below Homebrew's prefix, and where it
/// goes below `Contents/Resources/`.
const LIBRARY_LICENCES: [(&str, &str); 2] = [
    ("opt/molten-vk/LICENSE", "licenses/MoltenVK/LICENSE"),
    (
        "opt/vulkan-loader/LICENSE.txt",
        "licenses/Vulkan-Loader/LICENSE.txt",
    ),
];

/// The oldest macOS the app declares.
pub(crate) const MINIMUM_MACOS: &str = "14.0";

/// Builds the release executables through the cargo executable `cargo` and writes the app
/// and its zip under `target/app/` of `root`.
pub(crate) fn run(cargo: &str, root: &Path) -> Result<()> {
    if !cfg!(target_os = "macos") {
        bail!("app: the macOS app is made on macOS only");
    }
    let homebrew = Path::new(HOMEBREW);
    // The build takes minutes; a file that is missing is reported before it.
    for (source, _) in documents() {
        require(&root.join(source), "")?;
    }
    for source in homebrew_files() {
        require(
            &homebrew.join(source),
            " (from Homebrew: vulkan-loader, molten-vk)",
        )?;
    }
    let command_line = format!("cargo {}", BUILD.join(" "));
    eprintln!("app: running `{command_line}`");
    let status = Command::new(cargo)
        .args(BUILD)
        .current_dir(root)
        .status()
        .with_context(|| format!("app: could not start `{command_line}`"))?;
    if !status.success() {
        bail!("app: `{command_line}` ended with {status}");
    }
    let out = root.join("target/app");
    let app = assemble(root, homebrew, &out)?;
    let zip = out.join(format!("vkrgss-{}-{TARGET}.zip", env!("CARGO_PKG_VERSION")));
    if zip.exists() {
        fs::remove_file(&zip)
            .with_context(|| format!("app: could not remove {}", zip.display()))?;
    }
    let mut ditto = Command::new("ditto");
    tool(
        "ditto",
        ditto.args(["-c", "-k", "--keepParent"]).arg(&app).arg(&zip),
    )?;
    eprintln!("app: wrote {} and {}", app.display(), zip.display());
    Ok(())
}

/// The documents and licence texts of the package of `dist`: where each is below the
/// workspace root and where it goes below `Contents/Resources/`.
fn documents() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    FILES
        .iter()
        .filter(|(source, _)| !EXECUTABLES.contains(source))
}

/// The files the bundle takes from Homebrew, below its prefix.
fn homebrew_files() -> impl Iterator<Item = &'static str> {
    LIBRARIES
        .iter()
        .map(|(source, _)| *source)
        .chain([HOMEBREW_ICD])
        .chain(LIBRARY_LICENCES.iter().map(|(source, _)| *source))
}

/// The file `path`, or an error that names it, followed by `hint`, when it does not exist.
fn require(path: &Path, hint: &str) -> Result<()> {
    if !path.is_file() {
        bail!("app: `{}` is missing{hint}", path.display());
    }
    Ok(())
}

/// Assembles `vkrgss.app` in `out` from the built executables below `root` and the files of
/// Homebrew below `homebrew`, signs it and checks the libraries it names. Returns its path.
fn assemble(root: &Path, homebrew: &Path, out: &Path) -> Result<PathBuf> {
    let app = out.join("vkrgss.app");
    if app.exists() {
        fs::remove_dir_all(&app)
            .with_context(|| format!("app: could not remove {}", app.display()))?;
    }
    let contents = app.join("Contents");
    let mut binaries = Vec::new();
    for executable in EXECUTABLES {
        let from = root.join(executable);
        let name = from.file_name().expect("an executable has a name");
        let to = contents.join("MacOS").join(name);
        copy(&from, &to, 0o755)?;
        binaries.push(to);
    }
    for (source, name) in LIBRARIES {
        let to = contents.join("Frameworks").join(name);
        copy(&homebrew.join(source), &to, 0o755)?;
        let mut install_name_tool = Command::new("install_name_tool");
        tool(
            "install_name_tool",
            install_name_tool
                .args(["-id", &format!("@rpath/{name}")])
                .arg(&to),
        )?;
        binaries.push(to);
    }
    let icd = homebrew.join(HOMEBREW_ICD);
    let text = fs::read_to_string(&icd)
        .with_context(|| format!("app: could not read {}", icd.display()))?;
    write(&app.join(ICD), &bundle_icd(&text)?)?;
    write(
        &contents.join("Info.plist"),
        &info_plist(env!("CARGO_PKG_VERSION")),
    )?;
    let resources = contents.join("Resources");
    for (source, destination) in documents() {
        copy(&root.join(source), &resources.join(destination), 0o644)?;
    }
    for (source, destination) in LIBRARY_LICENCES {
        copy(&homebrew.join(source), &resources.join(destination), 0o644)?;
    }

    let mut codesign = Command::new("codesign");
    tool(
        "codesign",
        codesign
            .args(["--force", "--sign", "-", "--deep"])
            .arg(&app),
    )?;
    for binary in &binaries {
        let mut otool = Command::new("otool");
        let listed = tool("otool", otool.arg("-L").arg(binary))?;
        let foreign = homebrew_libraries(&String::from_utf8_lossy(&listed));
        if !foreign.is_empty() {
            bail!(
                "app: `{}` names {}, which a Mac without Homebrew lacks",
                binary.display(),
                foreign.join(", ")
            );
        }
    }
    Ok(app)
}

/// Copies `from` to `to`, making its directory, and gives the copy the mode `mode`.
fn copy(from: &Path, to: &Path, mode: u32) -> Result<()> {
    let copy = || -> std::io::Result<()> {
        fs::create_dir_all(to.parent().expect("a file below the bundle"))?;
        fs::copy(from, to)?;
        fs::set_permissions(to, fs::Permissions::from_mode(mode))
    };
    copy().with_context(|| format!("app: could not copy {} to {}", from.display(), to.display()))
}

/// Writes `text` to `path`, making its directory.
fn write(path: &Path, text: &str) -> Result<()> {
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(path.parent().expect("a file below the bundle"))?;
        fs::write(path, text)
    };
    write().with_context(|| format!("app: could not write {}", path.display()))
}

/// The manifest of MoltenVK of the bundle: Homebrew's, `homebrew`, with the library of the
/// bundle.
fn bundle_icd(homebrew: &str) -> Result<String> {
    let mut manifest: serde_json::Value =
        serde_json::from_str(homebrew).context("app: the manifest of MoltenVK does not parse")?;
    let Some(icd) = manifest.get_mut("ICD").and_then(|icd| icd.as_object_mut()) else {
        bail!("app: the manifest of MoltenVK has no object `ICD`");
    };
    icd.insert("library_path".to_owned(), ICD_LIBRARY_PATH.into());
    Ok(serde_json::to_string_pretty(&manifest)? + "\n")
}

/// The `Info.plist` of the app of version `version`.
fn info_plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>vkrgss</string>
	<key>CFBundleIdentifier</key>
	<string>{PACKAGE}</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>vkrgss</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>{version}</string>
	<key>CFBundleVersion</key>
	<string>{version}</string>
	<key>LSApplicationCategoryType</key>
	<string>public.app-category.role-playing-games</string>
	<key>LSMinimumSystemVersion</key>
	<string>{MINIMUM_MACOS}</string>
	<key>NSHighResolutionCapable</key>
	<true/>
</dict>
</plist>
"#
    )
}

/// The libraries under Homebrew's prefix in a listing of `otool -L`: a line with the file,
/// then one line per library, `\t<path> (compatibility version ..., current version ...)`;
/// the first of a library is its own install name.
fn homebrew_libraries(listing: &str) -> Vec<String> {
    listing
        .lines()
        .skip(1)
        .filter_map(|line| line.trim().split(" (").next())
        .filter(|path| Path::new(path).starts_with(HOMEBREW))
        .map(str::to_owned)
        .collect()
}

/// Runs the tool `name` and returns what it printed; a tool that is not installed or fails
/// is an error that names it.
fn tool(name: &str, command: &mut Command) -> Result<Vec<u8>> {
    let output = command
        .output()
        .with_context(|| format!("app: could not start `{name}`"))?;
    if !output.status.success() {
        bail!(
            "app: `{name}` ended with {}: {}",
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
    fn the_manifest_of_the_bundle_names_the_library_of_the_bundle_and_keeps_the_rest() {
        let homebrew = r#"{
    "file_format_version" : "1.0.0",
    "ICD": {
        "library_path": "../../../lib/libMoltenVK.dylib",
        "api_version" : "1.4.0",
        "is_portability_driver" : true
    }
}"#;
        let manifest: serde_json::Value =
            serde_json::from_str(&bundle_icd(homebrew).unwrap()).unwrap();
        assert_eq!(
            manifest,
            serde_json::json!({
                "file_format_version": "1.0.0",
                "ICD": {
                    "library_path": "../../../Frameworks/libMoltenVK.dylib",
                    "api_version": "1.4.0",
                    "is_portability_driver": true
                }
            })
        );
        // From the manifest's directory the library is that of `Contents/Frameworks/`.
        let library = Path::new(ICD).parent().unwrap().join(ICD_LIBRARY_PATH);
        let mut resolved = PathBuf::new();
        for part in library.components() {
            match part {
                std::path::Component::ParentDir => assert!(resolved.pop()),
                part => resolved.push(part),
            }
        }
        assert_eq!(resolved, Path::new("Contents/Frameworks/libMoltenVK.dylib"));
        assert!(
            LIBRARIES
                .iter()
                .any(|(_, name)| *name == "libMoltenVK.dylib")
        );
        assert!(bundle_icd("{}").is_err());
    }

    #[test]
    fn a_library_under_homebrew_is_named_and_one_of_the_system_or_the_bundle_is_not() {
        let listing = "\
target/app/vkrgss.app/Contents/Frameworks/libMoltenVK.dylib:
\t@rpath/libMoltenVK.dylib (compatibility version 1.0.0, current version 1.0.0)
\t/opt/homebrew/opt/foo/lib/libfoo.1.dylib (compatibility version 1.0.0, current version 1.0.0)
\t/System/Library/Frameworks/Metal.framework/Versions/A/Metal (compatibility version 1.0.0, current version 372.16.0)
\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1356.0.0)
";
        assert_eq!(
            homebrew_libraries(listing),
            ["/opt/homebrew/opt/foo/lib/libfoo.1.dylib"]
        );
        let homebrew_id = "\
libvulkan.1.dylib:
\t/opt/homebrew/opt/vulkan-loader/lib/libvulkan.1.dylib (compatibility version 1.0.0, current version 1.4.0)
";
        assert_eq!(homebrew_libraries(homebrew_id).len(), 1);
        // The line of the file itself is not a library.
        assert!(homebrew_libraries("/opt/homebrew/lib/libvulkan.1.dylib:\n").is_empty());
    }

    #[test]
    fn the_property_list_names_the_executable_the_identifier_and_the_minimum_macos() {
        let plist = info_plist("1.2.3");
        for wanted in [
            "<key>CFBundleExecutable</key>\n\t<string>vkrgss</string>",
            "<key>CFBundleIdentifier</key>\n\t<string>io.github.xuyanwen2012.vkrgss</string>",
            "<key>CFBundleShortVersionString</key>\n\t<string>1.2.3</string>",
            "<key>LSMinimumSystemVersion</key>\n\t<string>14.0</string>",
        ] {
            assert!(plist.contains(wanted), "{wanted}");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_property_list_is_accepted_by_plutil() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Info.plist");
        fs::write(&path, info_plist(env!("CARGO_PKG_VERSION"))).unwrap();
        let mut plutil = Command::new("plutil");
        tool("plutil", plutil.arg("-lint").arg(&path)).unwrap();
    }

    #[test]
    fn the_executables_of_the_bundle_are_those_of_the_package() {
        let names: Vec<_> = EXECUTABLES
            .iter()
            .map(|path| Path::new(path).file_name().unwrap().to_str().unwrap())
            .collect();
        assert_eq!(
            names,
            ["vkrgss", "vkrgss-guest-rgss12", "vkrgss-guest-rgss3"]
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_command_says_that_the_app_is_made_on_macos_only() {
        let root = tempfile::tempdir().unwrap();
        // A cargo that would fail differently if it were started.
        let error = run("/nonexistent/cargo", root.path()).unwrap_err();
        assert!(error.to_string().contains("made on macOS only"), "{error}");
    }
}
