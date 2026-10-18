//! The Android app: its debug APK and a run of a game on a phone. Run by hand, not by the
//! gate.
//!
//! `cargo xtask apk` builds `vkrgss-android` for `aarch64-linux-android` at API level 34
//! with the NDK that `ANDROID_NDK_HOME` names and assembles `target/apk/vkrgss-debug.apk`
//! with the tools of the SDK that `ANDROID_HOME` names, without Gradle: the Java class of the
//! activities ([`JAVA_SOURCE`]) is compiled by the compiler of the JDK (`java -m
//! jdk.compiler/...`, which a runtime with the module `jdk.compiler` has, against
//! `platforms/android-34/android.jar` at the class version of Java 8, which `--release` cannot
//! give against that library) and turned into `classes.dex` by `d8` of the build tools;
//! aapt2 links the manifest against `android.jar`, `classes.dex` goes to the top of the APK
//! and the library and the two guest executables (`vkrgss-guest-rgss12`,
//! `vkrgss-guest-rgss3`) go into `lib/arm64-v8a/` without their debug information (`zip`),
//! the executables under the names of libraries that the guest is started by on Android
//! ([`GUEST_PROGRAMS`]); zipalign aligns the APK and apksigner signs it with a debug key
//! that is made with keytool in `target/apk/` on first use. The manifest sets
//! `extractNativeLibs`, so that the installer extracts all three, the executables with the
//! execute bit, into the app's native library directory, the one place an app may execute
//! files from. It names two activities of the class: the library, which the launcher
//! starts, and the activity of games ([`GAME_ACTIVITY`]), in a process of its own
//! ([`GAME_PROCESS`]).
//!
//! `cargo xtask android-run <SERIAL> <GAME_DIR>` builds the APK, installs it on the phone,
//! starts the activity of games once if the app has no files directory yet, so that the app
//! makes it, replaces the game of the app's files directory with the directory `GAME_DIR`,
//! starts the activity of games without a game, which then runs that one, and waits until
//! its process has ended. Then it pulls the app's log and the frames that the `dump`
//! entries of the game's `vkrgss.toml` name into `target/android-run/<SERIAL>/`, with the
//! lines of logcat under the tag `vkrgss`. It uninstalls nothing; the next run installs over
//! it. See `docs/running.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use crate::android::{API_LEVEL, TARGET, adb, ndk_tools, run_host};

/// The package of the app.
pub(crate) const PACKAGE: &str = "io.github.xuyanwen2012.vkrgss";

/// The name of the native library, which the manifest gives the activities.
const LIBRARY: &str = "vkrgss_android";

/// The Java class of the activities, below the repository.
const JAVA_SOURCE: &str =
    "crates/vkrgss-android/java/io/github/xuyanwen2012/vkrgss/VkrgssActivity.java";

/// The activity of the library, relative to the package.
const LIBRARY_ACTIVITY: &str = ".VkrgssActivity";

/// The activity of games, relative to the package: a class nested in that of the library.
const GAME_ACTIVITY: &str = ".VkrgssActivity$Game";

/// The process of the activity of games, relative to the package's.
const GAME_PROCESS: &str = ":game";

/// The guest executables the APK carries: each package and the name of its file in
/// `lib/arm64-v8a/`, which the installer extracts only for a name `lib*.so`. They are the
/// names `process.rs` of `vkrgss` looks for on Android.
const GUEST_PROGRAMS: [(&str, &str); 2] = [
    ("vkrgss-guest-rgss12", "libvkrgss_guest_rgss12.so"),
    ("vkrgss-guest-rgss3", "libvkrgss_guest_rgss3.so"),
];

/// The API level the app targets.
const TARGET_SDK: u32 = 37;

/// The app's files directory on a phone, where the game and the log are.
pub(crate) const FILES_DIR: &str = "/sdcard/Android/data/io.github.xuyanwen2012.vkrgss/files";

/// How long a run may last on the phone.
const RUN_TIMEOUT: Duration = Duration::from_secs(900);

pub(crate) const USAGE: &str =
    "usage: cargo xtask apk | cargo xtask android-run <SERIAL> <GAME_DIR>";

/// The manifest: two subclasses of `NativeActivity` with the library, the library's
/// activity started by the launcher and that of games in a process of its own, the native
/// libraries extracted at installation (the guest executables are run from there),
/// landscape on either side, fullscreen, not restarted by a change of the configuration (the
/// game would start again), and Vulkan 1.3. The activity of games is exported, so that
/// `android-run` can start it.
fn manifest(version: &str) -> String {
    let activity = |name: &str, more: &str, filter: &str| {
        format!(
            r#"        <activity android:name="{name}"{more}
            android:exported="true"
            android:screenOrientation="sensorLandscape"
            android:theme="@android:style/Theme.NoTitleBar.Fullscreen"
            android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation|uiMode|density">
            <meta-data android:name="android.app.lib_name" android:value="{LIBRARY}" />{filter}
        </activity>
"#
        )
    };
    let library = activity(
        LIBRARY_ACTIVITY,
        r#"
            android:launchMode="singleTask""#,
        r#"
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>"#,
    );
    let game = activity(
        GAME_ACTIVITY,
        &format!(
            r#"
            android:process="{GAME_PROCESS}""#
        ),
        "",
    );
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{PACKAGE}"
    android:versionCode="{code}"
    android:versionName="{version}">
    <uses-feature android:name="android.hardware.vulkan.version" android:version="0x403000" android:required="true" />
    <application android:label="vkrgss" android:hasCode="true" android:appCategory="game"
        android:extractNativeLibs="true">
{library}{game}    </application>
</manifest>
"#,
        code = version_code(version)
    )
}

/// The version code of a version `MAJOR.MINOR.PATCH`: `MAJOR * 10000 + MINOR * 100 + PATCH`.
fn version_code(version: &str) -> u32 {
    version
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0))
        .chain(std::iter::repeat(0))
        .take(3)
        .fold(0, |code, part| code * 100 + part)
}

/// The SDK's directory.
fn sdk() -> Result<PathBuf> {
    let sdk = std::env::var_os("ANDROID_HOME")
        .or_else(|| std::env::var_os("ANDROID_SDK_ROOT"))
        .context("neither ANDROID_HOME nor ANDROID_SDK_ROOT names the Android SDK")?;
    Ok(PathBuf::from(sdk))
}

/// The newest build tools of the SDK.
fn build_tools(sdk: &Path) -> Result<PathBuf> {
    let dir = sdk.join("build-tools");
    let newest = fs::read_dir(&dir)
        .with_context(|| format!("the SDK has no {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("aapt2").is_file())
        .max_by_key(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.split('.')
                .map(|part| part.parse::<u32>().unwrap_or(0))
                .collect::<Vec<_>>()
        });
    newest.with_context(|| format!("no build tools with aapt2 in {}", dir.display()))
}

/// Builds the debug APK and returns its path.
pub(crate) fn build(cargo: &str, root: &Path) -> Result<PathBuf> {
    let (clang, ar) = ndk_tools()?;
    let strip = ar.with_file_name("llvm-strip");
    let sdk = sdk()?;
    let tools = build_tools(&sdk)?;
    let platform = sdk.join(format!("platforms/android-{API_LEVEL}/android.jar"));
    if !platform.is_file() {
        bail!("the SDK has no {}", platform.display());
    }

    eprintln!("apk: building {LIBRARY} and the guest executables for {TARGET}");
    let mut packages = vec!["-p", "vkrgss-android"];
    for (package, _) in GUEST_PROGRAMS {
        packages.extend(["-p", package]);
    }
    run_host(
        Command::new(cargo)
            .arg("build")
            .args(packages)
            .args(["--locked", "--target", TARGET])
            .env("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", &clang)
            .env("CC_aarch64_linux_android", &clang)
            .env("AR_aarch64_linux_android", &ar)
            .current_dir(root),
    )?;
    let built = root.join(format!("target/{TARGET}/debug"));
    let mut files = vec![(
        built.join(format!("lib{LIBRARY}.so")),
        format!("lib{LIBRARY}.so"),
    )];
    for (package, name) in GUEST_PROGRAMS {
        files.push((built.join(package), name.to_owned()));
    }

    let out = root.join("target/apk");
    let work = out.join("build");
    if work.exists() {
        fs::remove_dir_all(&work).with_context(|| work.display().to_string())?;
    }
    let lib_dir = work.join("lib/arm64-v8a");
    fs::create_dir_all(&lib_dir).with_context(|| lib_dir.display().to_string())?;
    for (file, name) in &files {
        run_host(
            Command::new(&strip)
                .arg("--strip-debug")
                .arg("-o")
                .arg(lib_dir.join(name))
                .arg(file),
        )?;
    }
    let version = env!("CARGO_PKG_VERSION");
    fs::write(work.join("AndroidManifest.xml"), manifest(version)).context("apk: the manifest")?;
    compile_java(root, &tools, &platform, &work)?;

    let unaligned = work.join("unaligned.apk");
    run_host(
        Command::new(tools.join("aapt2"))
            .arg("link")
            .arg("-o")
            .arg(&unaligned)
            .arg("--manifest")
            .arg(work.join("AndroidManifest.xml"))
            .arg("-I")
            .arg(&platform)
            .args(["--min-sdk-version", &API_LEVEL.to_string()])
            .args(["--target-sdk-version", &TARGET_SDK.to_string()])
            .arg("--debug-mode"),
    )?;
    // Compressed: the installer extracts them.
    run_host(
        Command::new("zip")
            .args(["-X", "-q"])
            .arg(&unaligned)
            .arg("classes.dex")
            .args(
                files
                    .iter()
                    .map(|(_, name)| format!("lib/arm64-v8a/{name}")),
            )
            .current_dir(&work),
    )?;
    let aligned = work.join("aligned.apk");
    run_host(
        Command::new(tools.join("zipalign"))
            .args(["-f", "-P", "16", "4"])
            .arg(&unaligned)
            .arg(&aligned),
    )?;
    let keystore = debug_key(&out)?;
    let apk = out.join("vkrgss-debug.apk");
    run_host(
        Command::new(tools.join("apksigner"))
            .arg("sign")
            .arg("--ks")
            .arg(&keystore)
            .args(["--ks-pass", "pass:android", "--key-pass", "pass:android"])
            .arg("--out")
            .arg(&apk)
            .arg(&aligned),
    )?;
    eprintln!("apk: {}", apk.display());
    Ok(apk)
}

/// Compiles the Java class of the activities into `work/classes.dex`.
fn compile_java(root: &Path, tools: &Path, platform: &Path, work: &Path) -> Result<()> {
    eprintln!("apk: compiling {JAVA_SOURCE}");
    let classes = work.join("classes");
    let dex = work.join("dex");
    for dir in [&classes, &dex] {
        fs::create_dir_all(dir).with_context(|| dir.display().to_string())?;
    }
    run_host(
        Command::new("java")
            .args(["-m", "jdk.compiler/com.sun.tools.javac.Main"])
            .args([
                "-source",
                "8",
                "-target",
                "8",
                "-Xlint:-options",
                "-encoding",
                "UTF-8",
            ])
            .arg("-bootclasspath")
            .arg(platform)
            .arg("-d")
            .arg(&classes)
            .arg(root.join(JAVA_SOURCE)),
    )?;
    let mut compiled = Vec::new();
    let mut pending = vec![classes];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).with_context(|| dir.display().to_string())? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "class")
            {
                compiled.push(path);
            }
        }
    }
    compiled.sort();
    run_host(
        Command::new(tools.join("d8"))
            .args(["--min-api", &API_LEVEL.to_string(), "--release"])
            .arg("--lib")
            .arg(platform)
            .arg("--output")
            .arg(&dex)
            .args(&compiled),
    )?;
    fs::rename(dex.join("classes.dex"), work.join("classes.dex")).context("apk: classes.dex")
}

/// The debug keystore in `out`, made on first use.
fn debug_key(out: &Path) -> Result<PathBuf> {
    let keystore = out.join("debug.keystore");
    if keystore.is_file() {
        return Ok(keystore);
    }
    eprintln!("apk: making the debug key {}", keystore.display());
    run_host(
        Command::new("keytool")
            .args(["-genkeypair", "-keystore"])
            .arg(&keystore)
            .args(["-storepass", "android", "-keypass", "android"])
            .args([
                "-alias",
                "androiddebugkey",
                "-keyalg",
                "RSA",
                "-keysize",
                "2048",
            ])
            .args([
                "-validity",
                "10000",
                "-dname",
                "CN=Android Debug,O=Android,C=US",
            ]),
    )?;
    Ok(keystore)
}

/// Runs a command on the phone and gives its standard output.
pub(crate) fn output(device: &str, args: &[&str]) -> Result<String> {
    let output = adb(Some(device))
        .args(args)
        .output()
        .context("android-run: adb could not be started")?;
    if !output.status.success() {
        bail!(
            "android-run: adb {} ended with {}\n{}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Whether the process of the activity of games is alive on the phone.
fn running(device: &str) -> Result<bool> {
    // `pidof` answers with status 1 and no output when there is no such process.
    let output = adb(Some(device))
        .args(["shell", "pidof", &format!("{PACKAGE}{GAME_PROCESS}")])
        .output()
        .context("android-run: adb could not be started")?;
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

/// The files that the `dump` entries of the game's `vkrgss.toml` name, relative to the game
/// directory.
fn dumped_files(game: &Path) -> Result<Vec<String>> {
    let path = game.join("vkrgss.toml");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    let table: toml::Table = toml::from_str(&text).with_context(|| path.display().to_string())?;
    let Some(entries) = table.get("dump").and_then(toml::Value::as_array) else {
        return Ok(Vec::new());
    };
    Ok(entries
        .iter()
        .filter_map(toml::Value::as_str)
        .filter_map(|entry| entry.split_once('=').map(|(_, file)| file.to_owned()))
        .collect())
}

/// The name of the activity of games for `am start`, whose shell would read the `$`.
fn game_component() -> String {
    format!("{PACKAGE}/{}", GAME_ACTIVITY.replace('$', "\\$"))
}

/// Starts the activity of games without a game on the phone `device`, which runs the game of
/// the files directory, and waits until its process has ended, at most `timeout`; after
/// that the app is stopped and the run fails.
pub(crate) fn start_and_wait(device: &str, timeout: Duration) -> Result<()> {
    output(device, &["shell", "am", "start", "-n", &game_component()])?;
    eprintln!("android-run: started; waiting for the app to end");
    let started = Instant::now();
    // The process may take a moment to appear.
    thread::sleep(Duration::from_secs(2));
    while running(device)? {
        if started.elapsed() > timeout {
            output(device, &["shell", "am", "force-stop", PACKAGE])?;
            bail!(
                "android-run: the app still ran after {} seconds and was stopped",
                timeout.as_secs()
            );
        }
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

/// Builds the APK and installs it on the phone `device`, stops the app, and starts it once
/// if it has no files directory yet, so that the app makes it.
pub(crate) fn install(cargo: &str, root: &Path, device: &str) -> Result<()> {
    let apk = build(cargo, root)?;
    eprintln!("android-run: installing on {device}");
    run_host(adb(Some(device)).arg("install").arg("-r").arg(&apk))?;
    output(device, &["shell", "am", "force-stop", PACKAGE])?;
    // A directory that adb makes there is not the app's, which then cannot read it: a
    // freshly installed app makes its own when it first starts, and ends without a game.
    if !output(
        device,
        &[
            "shell",
            &format!("test -d {FILES_DIR} && echo yes || echo no"),
        ],
    )?
    .contains("yes")
    {
        eprintln!("android-run: starting the app once, to make its files directory");
        start_and_wait(device, RUN_TIMEOUT)?;
    }
    Ok(())
}

/// Runs the game `game` on the phone `device`; see the module documentation.
pub(crate) fn run(cargo: &str, root: &Path, device: &str, game: &Path) -> Result<()> {
    if !game.join("Game.ini").is_file() {
        bail!("android-run: {} has no Game.ini", game.display());
    }
    let dumps = dumped_files(game)?;
    install(cargo, root, device)?;
    let remote_game = format!("{FILES_DIR}/game");
    output(
        device,
        &[
            "shell",
            &format!("rm -rf {remote_game} {FILES_DIR}/vkrgss.log"),
        ],
    )?;
    eprintln!("android-run: pushing {} to {remote_game}", game.display());
    run_host(adb(Some(device)).arg("push").arg(game).arg(&remote_game))?;
    output(device, &["logcat", "-c"])?;
    start_and_wait(device, RUN_TIMEOUT)?;

    let out = root.join("target/android-run").join(device);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| out.display().to_string())?;
    }
    fs::create_dir_all(&out).with_context(|| out.display().to_string())?;
    let logcat = output(device, &["logcat", "-d", "-s", "vkrgss"])?;
    fs::write(out.join("logcat.txt"), &logcat).context("android-run: logcat.txt")?;
    run_host(
        adb(Some(device))
            .arg("pull")
            .arg(format!("{FILES_DIR}/vkrgss.log"))
            .arg(out.join("vkrgss.log")),
    )?;
    let mut missing = Vec::new();
    for file in &dumps {
        let local = out.join(file);
        if let Some(parent) = local.parent() {
            fs::create_dir_all(parent).with_context(|| parent.display().to_string())?;
        }
        let pulled = adb(Some(device))
            .arg("pull")
            .arg(format!("{remote_game}/{file}"))
            .arg(&local)
            .output()
            .context("android-run: adb could not be started")?;
        if !pulled.status.success() {
            missing.push(file.as_str());
        }
    }
    let log = fs::read_to_string(out.join("vkrgss.log")).unwrap_or_default();
    eprint!("{log}");
    eprintln!(
        "android-run: the log and the frames are in {}",
        out.display()
    );
    if !missing.is_empty() {
        bail!("android-run: the app wrote no {}", missing.join(", "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_code_has_two_digits_for_the_minor_version_and_the_patch() {
        assert_eq!(version_code("0.3.1"), 301);
        assert_eq!(version_code("1.12.5"), 11205);
        assert_eq!(version_code("2"), 20000);
    }

    #[test]
    fn the_manifest_names_both_activities_of_the_java_class_and_their_library() {
        let manifest = manifest("0.3.1");
        assert!(manifest.contains(r#"android:name=".VkrgssActivity""#));
        assert!(manifest.contains(r#"android:name=".VkrgssActivity$Game""#));
        assert!(manifest.contains(r#"android:process=":game""#));
        assert_eq!(
            manifest
                .matches(r#"android:value="vkrgss_android""#)
                .count(),
            2
        );
        assert_eq!(
            manifest.matches("android.intent.category.LAUNCHER").count(),
            1
        );
        assert!(manifest.contains(r#"android:hasCode="true""#));
        assert!(manifest.contains(r#"android:versionCode="301""#));
        assert!(manifest.contains(r#"android:extractNativeLibs="true""#));
        assert!(FILES_DIR.contains(PACKAGE));
        assert_eq!(
            game_component(),
            "io.github.xuyanwen2012.vkrgss/.VkrgssActivity\\$Game"
        );
    }

    /// The Java class is the one the manifest names, with the native method and the methods
    /// the native code calls.
    #[test]
    fn the_java_class_has_the_activities_and_the_methods_of_the_native_code() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let source = fs::read_to_string(root.join(JAVA_SOURCE)).unwrap();
        assert!(source.contains("package io.github.xuyanwen2012.vkrgss;"));
        assert!(source.contains("public class VkrgssActivity extends NativeActivity"));
        assert!(source.contains("public static final class Game extends VkrgssActivity"));
        assert!(source.contains(&format!("System.loadLibrary(\"{LIBRARY}\")")));
        let native = fs::read_to_string(root.join("crates/vkrgss-android/src/java.rs")).unwrap();
        assert!(native.contains("Java_io_github_xuyanwen2012_vkrgss_VkrgssActivity_nativeImport"));
        assert!(
            source.contains("private static native void nativeImport(String uri, boolean rtp)")
        );
        for method in ["gameDir", "openPicker", "copyToCache", "startGame"] {
            assert!(source.contains(&format!(" {method}(")), "{method}");
            assert!(
                native.contains(&format!("jni_str!(\"{method}\")")),
                "{method}"
            );
        }
    }

    #[test]
    fn the_guest_executables_have_names_the_installer_extracts() {
        for (_, name) in GUEST_PROGRAMS {
            assert!(name.starts_with("lib") && name.ends_with(".so"), "{name}");
        }
    }

    #[test]
    fn the_dumped_files_are_read_from_the_game_s_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(dumped_files(dir.path()).unwrap().is_empty());
        fs::write(
            dir.path().join("vkrgss.toml"),
            "frames = 9\ndump = [\"4=frames/4.png\", \"8=8.png\"]\n",
        )
        .unwrap();
        assert_eq!(dumped_files(dir.path()).unwrap(), ["frames/4.png", "8.png"]);
    }
}
