//! The Android app: its debug APK and a run of a game on a phone. Run by hand, not by the
//! gate.
//!
//! `cargo xtask apk` builds `vkrgss-android` for `aarch64-linux-android` at API level 34
//! with the NDK that `ANDROID_NDK_HOME` names and assembles `target/apk/vkrgss-debug.apk`
//! with the tools of the SDK that `ANDROID_HOME` names, without Gradle: aapt2 links the
//! manifest against `platforms/android-34/android.jar`, the library goes into
//! `lib/arm64-v8a/` uncompressed (`zip`) and without its debug information, zipalign aligns
//! it to pages of 16 KiB and apksigner signs the APK with a debug key that is made with
//! keytool in `target/apk/` on first use.
//!
//! `cargo xtask android-run <SERIAL> <GAME_DIR>` builds the APK, installs it on the phone,
//! replaces the game of the app's files directory with the directory `GAME_DIR`, starts the
//! activity and waits until the app has ended. Then it pulls the app's log and the frames
//! that the `dump` entries of the game's `vkrgss.toml` name into
//! `target/android-run/<SERIAL>/`, with the lines of logcat under the tag `vkrgss`. It
//! uninstalls nothing; the next run installs over it. See `docs/running.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use crate::android::{API_LEVEL, TARGET, adb, ndk_tools, run_host};

/// The package of the app.
pub(crate) const PACKAGE: &str = "io.github.xuyanwen2012.vkrgss";

/// The name of the native library, which the manifest gives the activity.
const LIBRARY: &str = "vkrgss_android";

/// The API level the app targets.
const TARGET_SDK: u32 = 37;

/// The app's files directory on a phone, where the game and the log are.
const FILES_DIR: &str = "/sdcard/Android/data/io.github.xuyanwen2012.vkrgss/files";

/// How long a run may last on the phone.
const RUN_TIMEOUT: Duration = Duration::from_secs(900);

pub(crate) const USAGE: &str =
    "usage: cargo xtask apk | cargo xtask android-run <SERIAL> <GAME_DIR>";

/// The manifest: `NativeActivity` with the library, no Java code, landscape on either side,
/// fullscreen, not restarted by a change of the configuration (the interpreter cannot be
/// started twice in one process), and Vulkan 1.3.
fn manifest(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{PACKAGE}"
    android:versionCode="{code}"
    android:versionName="{version}">
    <uses-feature android:name="android.hardware.vulkan.version" android:version="0x403000" android:required="true" />
    <application android:label="vkrgss" android:hasCode="false" android:appCategory="game">
        <activity android:name="android.app.NativeActivity"
            android:exported="true"
            android:launchMode="singleTask"
            android:screenOrientation="sensorLandscape"
            android:theme="@android:style/Theme.NoTitleBar.Fullscreen"
            android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation|uiMode|density">
            <meta-data android:name="android.app.lib_name" android:value="{LIBRARY}" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
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

    eprintln!("apk: building {LIBRARY} for {TARGET}");
    run_host(
        Command::new(cargo)
            .args([
                "build",
                "-p",
                "vkrgss-android",
                "--locked",
                "--target",
                TARGET,
            ])
            .env("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", &clang)
            .env("CC_aarch64_linux_android", &clang)
            .env("AR_aarch64_linux_android", &ar)
            .current_dir(root),
    )?;
    let library = root.join(format!("target/{TARGET}/debug/lib{LIBRARY}.so"));

    let out = root.join("target/apk");
    let work = out.join("build");
    if work.exists() {
        fs::remove_dir_all(&work).with_context(|| work.display().to_string())?;
    }
    let lib_dir = work.join("lib/arm64-v8a");
    fs::create_dir_all(&lib_dir).with_context(|| lib_dir.display().to_string())?;
    run_host(
        Command::new(&strip)
            .arg("--strip-debug")
            .arg("-o")
            .arg(lib_dir.join(format!("lib{LIBRARY}.so")))
            .arg(&library),
    )?;
    let version = env!("CARGO_PKG_VERSION");
    fs::write(work.join("AndroidManifest.xml"), manifest(version)).context("apk: the manifest")?;

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
    // Stored, so that the library is loaded from the APK in place.
    run_host(
        Command::new("zip")
            .args(["-0", "-X", "-q"])
            .arg(&unaligned)
            .arg(format!("lib/arm64-v8a/lib{LIBRARY}.so"))
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
fn output(device: &str, args: &[&str]) -> Result<String> {
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

/// Whether the app's process is alive on the phone.
fn running(device: &str) -> Result<bool> {
    // `pidof` answers with status 1 and no output when there is no such process.
    let output = adb(Some(device))
        .args(["shell", "pidof", PACKAGE])
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

/// Runs the game `game` on the phone `device`; see the module documentation.
pub(crate) fn run(cargo: &str, root: &Path, device: &str, game: &Path) -> Result<()> {
    if !game.join("Game.ini").is_file() {
        bail!("android-run: {} has no Game.ini", game.display());
    }
    let dumps = dumped_files(game)?;
    let apk = build(cargo, root)?;
    eprintln!("android-run: installing on {device}");
    run_host(adb(Some(device)).arg("install").arg("-r").arg(&apk))?;
    output(device, &["shell", "am", "force-stop", PACKAGE])?;
    let remote_game = format!("{FILES_DIR}/game");
    output(
        device,
        &[
            "shell",
            &format!("rm -rf {remote_game} {FILES_DIR}/vkrgss.log && mkdir -p {FILES_DIR}"),
        ],
    )?;
    eprintln!("android-run: pushing {} to {remote_game}", game.display());
    run_host(adb(Some(device)).arg("push").arg(game).arg(&remote_game))?;
    output(device, &["logcat", "-c"])?;
    output(
        device,
        &[
            "shell",
            "am",
            "start",
            "-n",
            &format!("{PACKAGE}/android.app.NativeActivity"),
        ],
    )?;
    eprintln!("android-run: started; waiting for the app to end");
    let started = Instant::now();
    // The process may take a moment to appear.
    thread::sleep(Duration::from_secs(2));
    while running(device)? {
        if started.elapsed() > RUN_TIMEOUT {
            bail!(
                "android-run: the app still runs after {} seconds",
                RUN_TIMEOUT.as_secs()
            );
        }
        thread::sleep(Duration::from_secs(1));
    }

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
    fn the_manifest_names_the_native_activity_and_its_library() {
        let manifest = manifest("0.3.1");
        assert!(manifest.contains(r#"android:name="android.app.NativeActivity""#));
        assert!(manifest.contains(r#"android:value="vkrgss_android""#));
        assert!(manifest.contains(r#"android:hasCode="false""#));
        assert!(manifest.contains(r#"android:versionCode="301""#));
        assert!(FILES_DIR.contains(PACKAGE));
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
