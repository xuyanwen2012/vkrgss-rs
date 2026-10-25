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
//! aapt2 compiles the resources of [`RESOURCES`] (the launcher icon, adaptive with a
//! monochrome layer for Android 8 and later, legacy and round per density) and links them
//! and the manifest against `android.jar`, `classes.dex` goes to the top of the APK
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
//! `cargo xtask apk --release` builds the same APK in the release profile, without the flag
//! `debuggable`, signs it with the key of the keystore that the environment names
//! ([`KEY_VARIABLES`]; the passwords reach apksigner as `env:` references, never in a
//! command line) with signatures of the schemes v2 and v3, and writes it as
//! `target/apk/vkrgss-<VERSION>-android-arm64.apk` with its SHA-256 checksum beside it, as
//! `sha256sum` writes one. Without the variables it refuses and names them. Both APKs carry
//! the workspace's version as `versionName` and the [`version_code`] made from it.
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

/// The resources of the app, below the repository: the launcher icon.
const RESOURCES: &str = "crates/vkrgss-android/res";

/// The app's label, which the launcher shows.
const LABEL: &str = "RGSS Player";

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

pub(crate) const USAGE: &str = "usage: cargo xtask apk [--release] | \
                                 cargo xtask android-run [--validation] <SERIAL> <GAME_DIR>";

/// The variable that names the Khronos validation layer for Android (arm64) that the APK of
/// [`Flavor::Validation`] carries.
pub(crate) const LAYER_VARIABLE: &str = "VKRGSS_ANDROID_VALIDATION_LAYER";

/// The file name of the validation layer in `lib/arm64-v8a/`, where the loader of Android
/// finds the layers of a debuggable app.
const LAYER_FILE: &str = "libVkLayer_khronos_validation.so";

/// The extra of the intent of the activity of games that runs the game under the layer.
const VALIDATION_EXTRA: &str = "validation";

/// The variables that name the key of the release: the keystore file, its password, the
/// alias of the key in it and the key's password.
pub(crate) const KEY_VARIABLES: [&str; 4] = [
    "VKRGSS_ANDROID_KEYSTORE",
    "VKRGSS_ANDROID_KEYSTORE_PASSWORD",
    "VKRGSS_ANDROID_KEY_ALIAS",
    "VKRGSS_ANDROID_KEY_PASSWORD",
];

/// What kind of APK is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flavor {
    /// The development profile, debuggable, signed with the debug key of this machine.
    Debug,
    /// The release profile, signed with the key the environment names.
    Release,
    /// [`Flavor::Debug`] with the validation layer that [`LAYER_VARIABLE`] names.
    Validation,
}

/// The key an APK is signed with.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Key {
    /// The debug keystore of this machine, whose passwords are known.
    Debug(PathBuf),
    /// The keystore of the release and the alias of its key; the passwords stay in the
    /// environment.
    Release { keystore: PathBuf, alias: String },
}

/// The key of the release, from the variables that `lookup` gives: an error that names them
/// all, and those missing, when one is unset or empty.
fn release_key(lookup: impl Fn(&str) -> Option<String>) -> Result<Key> {
    let missing: Vec<&str> = KEY_VARIABLES
        .iter()
        .copied()
        .filter(|name| lookup(name).is_none_or(|value| value.is_empty()))
        .collect();
    if !missing.is_empty() {
        bail!(
            "apk --release: the release is signed with the key that {} name; {} {} not set",
            KEY_VARIABLES.join(", "),
            missing.join(", "),
            if missing.len() == 1 { "is" } else { "are" }
        );
    }
    let keystore = PathBuf::from(lookup(KEY_VARIABLES[0]).unwrap_or_default());
    if !keystore.is_file() {
        bail!(
            "apk --release: {} names {}, which is no file",
            KEY_VARIABLES[0],
            keystore.display()
        );
    }
    Ok(Key::Release {
        keystore,
        alias: lookup(KEY_VARIABLES[2]).unwrap_or_default(),
    })
}

/// The command of apksigner that signs `aligned` into `apk` with `key`, with signatures of
/// the schemes v2 and v3 (v1 is left out by apksigner for API level 24 and above).
fn sign_command(apksigner: &Path, key: &Key, aligned: &Path, apk: &Path) -> Command {
    let mut command = Command::new(apksigner);
    command.arg("sign");
    match key {
        Key::Debug(keystore) => {
            command.arg("--ks").arg(keystore).args([
                "--ks-pass",
                "pass:android",
                "--key-pass",
                "pass:android",
            ]);
        }
        Key::Release { keystore, alias } => {
            command
                .arg("--ks")
                .arg(keystore)
                .args(["--ks-key-alias", alias])
                .args(["--ks-pass", &format!("env:{}", KEY_VARIABLES[1])])
                .args(["--key-pass", &format!("env:{}", KEY_VARIABLES[3])]);
        }
    }
    command
        .args([
            "--v2-signing-enabled",
            "true",
            "--v3-signing-enabled",
            "true",
        ])
        // No file `.idsig` beside it, which only an incremental installation reads.
        .args(["--v4-signing-enabled", "false"])
        .arg("--out")
        .arg(apk)
        .arg(aligned);
    command
}

/// The file name of the release APK of `version`, the asset of the GitHub release.
pub(crate) fn release_name(version: &str) -> String {
    format!("vkrgss-{version}-android-arm64.apk")
}

/// The manifest: two subclasses of `NativeActivity` with the library, the library's
/// activity started by the launcher and that of games in a process of its own, the native
/// libraries extracted at installation (the guest executables are run from there),
/// landscape on either side, fullscreen, not restarted by a change of the configuration (the
/// game would start again), and Vulkan 1.3. The application has the [`LABEL`], which the
/// activities inherit, and the launcher icon of [`RESOURCES`]. The activity of games is
/// exported, so that `android-run` can start it.
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
    <application android:label="{LABEL}" android:icon="@mipmap/ic_launcher"
        android:roundIcon="@mipmap/ic_launcher_round" android:hasCode="true"
        android:appCategory="game" android:extractNativeLibs="true">
{library}{game}    </application>
</manifest>
"#,
        code = version_code(version)
    )
}

/// The version code of a version `MAJOR.MINOR.PATCH`:
/// `MAJOR * 1_000_000 + MINOR * 1_000 + PATCH`. A suffix such as `-rc.1` is left out.
fn version_code(version: &str) -> u32 {
    let core = version.split(['-', '+']).next().unwrap_or_default();
    core.split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0))
        .chain(std::iter::repeat(0))
        .take(3)
        .fold(0, |code, part| code * 1_000 + part)
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

/// Builds the APK of `flavor` and returns its path.
pub(crate) fn build(cargo: &str, root: &Path, flavor: Flavor) -> Result<PathBuf> {
    // Before the build, which takes long.
    let release_key = match flavor {
        Flavor::Release => Some(release_key(|name| std::env::var(name).ok())?),
        Flavor::Debug | Flavor::Validation => None,
    };
    let layer = match flavor {
        Flavor::Validation => Some(validation_layer(std::env::var_os(LAYER_VARIABLE))?),
        Flavor::Debug | Flavor::Release => None,
    };
    let (clang, ar) = ndk_tools()?;
    let strip = ar.with_file_name("llvm-strip");
    let sdk = sdk()?;
    let tools = build_tools(&sdk)?;
    let platform = sdk.join(format!("platforms/android-{API_LEVEL}/android.jar"));
    if !platform.is_file() {
        bail!("the SDK has no {}", platform.display());
    }

    let profile = match flavor {
        Flavor::Release => "release",
        Flavor::Debug | Flavor::Validation => "debug",
    };
    eprintln!("apk: building {LIBRARY} and the guest executables for {TARGET}, {profile}");
    let mut packages = vec!["-p", "vkrgss-android"];
    for (package, _) in GUEST_PROGRAMS {
        packages.extend(["-p", package]);
    }
    run_host(
        Command::new(cargo)
            .arg("build")
            .args(packages)
            .args(["--locked", "--target", TARGET])
            .args((flavor == Flavor::Release).then_some("--release"))
            .env("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", &clang)
            .env("CC_aarch64_linux_android", &clang)
            .env("AR_aarch64_linux_android", &ar)
            .current_dir(root),
    )?;
    let built = root.join(format!("target/{TARGET}/{profile}"));
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
    // As it is: the layer of a release of the SDK.
    if let Some(layer) = &layer {
        fs::copy(layer, lib_dir.join(LAYER_FILE)).with_context(|| layer.display().to_string())?;
        files.push((layer.clone(), LAYER_FILE.to_owned()));
    }
    let version = env!("CARGO_PKG_VERSION");
    fs::write(work.join("AndroidManifest.xml"), manifest(version)).context("apk: the manifest")?;
    compile_java(root, &tools, &platform, &work)?;
    let resources = work.join("res.zip");
    run_host(
        Command::new(tools.join("aapt2"))
            .arg("compile")
            .arg("--dir")
            .arg(root.join(RESOURCES))
            .arg("-o")
            .arg(&resources),
    )?;

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
            .args((flavor != Flavor::Release).then_some("--debug-mode"))
            .arg(&resources),
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
    let (key, name) = match release_key {
        Some(key) => (key, release_name(version)),
        None => (
            Key::Debug(debug_key(&out)?),
            match flavor {
                Flavor::Validation => "vkrgss-validation.apk",
                _ => "vkrgss-debug.apk",
            }
            .to_owned(),
        ),
    };
    let apk = out.join(&name);
    run_host(&mut sign_command(
        &tools.join("apksigner"),
        &key,
        &aligned,
        &apk,
    ))?;
    if flavor == Flavor::Release {
        let checksum = Command::new("sha256sum")
            .arg(&name)
            .current_dir(&out)
            .output()
            .context("apk: sha256sum could not be started")?;
        if !checksum.status.success() {
            bail!("apk: sha256sum ended with {}", checksum.status);
        }
        let file = out.join(format!("{name}.sha256"));
        fs::write(&file, checksum.stdout).with_context(|| file.display().to_string())?;
        eprintln!("apk: {} and its .sha256", apk.display());
    } else {
        eprintln!("apk: {}", apk.display());
    }
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

/// The validation layer that the variable [`LAYER_VARIABLE`] gives as `value`, which must
/// name the file.
fn validation_layer(value: Option<std::ffi::OsString>) -> Result<PathBuf> {
    let Some(path) = value.filter(|value| !value.is_empty()).map(PathBuf::from) else {
        bail!(
            "--validation: {LAYER_VARIABLE} names no file; it names {LAYER_FILE} of the \
             Android binaries (arm64-v8a) of a release of the Vulkan validation layers"
        );
    };
    if !path.is_file() {
        bail!(
            "--validation: {LAYER_VARIABLE} names {}, which is no file",
            path.display()
        );
    }
    Ok(path)
}

/// What a run under the validation layer wrote to the lines of logcat `logcat`: an error
/// unless the app said that the layer reported no error, and then one that names how many
/// errors were reported.
pub(crate) fn validation_outcome(logcat: &str) -> Result<()> {
    let errors = logcat.matches("validation error:").count();
    if errors > 0 {
        bail!("the validation layer reported {errors} errors");
    }
    if !logcat.contains("validation: the layer reported no error") {
        bail!("the app did not say that the validation layer reported no error");
    }
    Ok(())
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
/// the files directory, under the validation layer with `validation`, and waits until its
/// process has ended, at most `timeout`; after that the app is stopped and the run fails.
pub(crate) fn start_and_wait(device: &str, timeout: Duration, validation: bool) -> Result<()> {
    let component = game_component();
    let mut start = vec!["shell", "am", "start", "-n", &component];
    if validation {
        start.extend(["--ez", VALIDATION_EXTRA, "true"]);
    }
    output(device, &start)?;
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

/// Builds the APK of `flavor` and installs it on the phone `device`, stops the app, and
/// starts it once if it has no files directory yet, so that the app makes it.
pub(crate) fn install(cargo: &str, root: &Path, device: &str, flavor: Flavor) -> Result<()> {
    let apk = build(cargo, root, flavor)?;
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
        start_and_wait(device, RUN_TIMEOUT, false)?;
    }
    Ok(())
}

/// Runs the game `game` on the phone `device`, under the validation layer with
/// `validation`; see the module documentation.
pub(crate) fn run(
    cargo: &str,
    root: &Path,
    device: &str,
    game: &Path,
    validation: bool,
) -> Result<()> {
    if !game.join("Game.ini").is_file() {
        bail!("android-run: {} has no Game.ini", game.display());
    }
    let dumps = dumped_files(game)?;
    let flavor = match validation {
        true => Flavor::Validation,
        false => Flavor::Debug,
    };
    install(cargo, root, device, flavor)?;
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
    start_and_wait(device, RUN_TIMEOUT, validation)?;

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
    if validation {
        validation_outcome(&logcat).context("android-run")?;
        eprintln!("android-run: the validation layer reported no error");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_code_has_three_digits_for_the_minor_version_and_the_patch() {
        assert_eq!(version_code("0.3.1"), 3_001);
        assert_eq!(version_code("1.12.5"), 1_012_005);
        assert_eq!(version_code("2"), 2_000_000);
        assert_eq!(version_code("0.4.0-rc.1"), 4_000);
        // Above every code of the former formula, `MAJOR * 10000 + MINOR * 100 + PATCH`,
        // that a version up to 0.3.1 had, so that an update installs over it.
        assert!(version_code("0.3.1") > 301);
    }

    /// Scenario "No keystore": the command fails before it builds and names the variables.
    #[test]
    fn the_release_without_the_variables_names_them() {
        let message = format!("{:#}", release_key(|_| None).unwrap_err());
        for name in KEY_VARIABLES {
            assert!(message.contains(name), "{message}");
        }
        // One missing, the others given.
        let dir = tempfile::tempdir().unwrap();
        let keystore = dir.path().join("release.keystore");
        fs::write(&keystore, b"x").unwrap();
        let given = |missing: &'static str| {
            let keystore = keystore.display().to_string();
            move |name: &str| match name {
                _ if name == missing => Some(String::new()),
                "VKRGSS_ANDROID_KEYSTORE" => Some(keystore.clone()),
                _ => Some("secret".to_owned()),
            }
        };
        let message = format!(
            "{:#}",
            release_key(given("VKRGSS_ANDROID_KEY_PASSWORD")).unwrap_err()
        );
        assert!(
            message.ends_with("VKRGSS_ANDROID_KEY_PASSWORD is not set"),
            "{message}"
        );
        assert_eq!(
            release_key(given("NONE")).unwrap(),
            Key::Release {
                keystore: keystore.clone(),
                alias: "secret".to_owned()
            }
        );
        // A keystore that is not there.
        let message = format!(
            "{:#}",
            release_key(|name| Some(match name {
                "VKRGSS_ANDROID_KEYSTORE" => "/nonexistent/release.keystore".to_owned(),
                _ => "secret".to_owned(),
            }))
            .unwrap_err()
        );
        assert!(
            message.contains("/nonexistent/release.keystore"),
            "{message}"
        );
    }

    /// The passwords of the release reach apksigner as references to the environment.
    #[test]
    fn the_release_is_signed_without_a_password_in_the_command_line() {
        let key = Key::Release {
            keystore: PathBuf::from("/keys/release.keystore"),
            alias: "vkrgss".to_owned(),
        };
        let command = sign_command(
            Path::new("apksigner"),
            &key,
            Path::new("aligned.apk"),
            Path::new("out.apk"),
        );
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "sign",
                "--ks",
                "/keys/release.keystore",
                "--ks-key-alias",
                "vkrgss",
                "--ks-pass",
                "env:VKRGSS_ANDROID_KEYSTORE_PASSWORD",
                "--key-pass",
                "env:VKRGSS_ANDROID_KEY_PASSWORD",
                "--v2-signing-enabled",
                "true",
                "--v3-signing-enabled",
                "true",
                "--v4-signing-enabled",
                "false",
                "--out",
                "out.apk",
                "aligned.apk",
            ]
        );
        assert_eq!(release_name("0.4.0"), "vkrgss-0.4.0-android-arm64.apk");
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
        assert!(manifest.contains(r#"android:versionCode="3001""#));
        assert!(manifest.contains(r#"android:versionName="0.3.1""#));
        assert!(manifest.contains(r#"android:extractNativeLibs="true""#));
        assert!(manifest.contains(&format!("package=\"{PACKAGE}\"")));
        assert_eq!(PACKAGE, "io.github.xuyanwen2012.vkrgss");
        assert!(FILES_DIR.contains(PACKAGE));
        assert_eq!(
            game_component(),
            "io.github.xuyanwen2012.vkrgss/.VkrgssActivity\\$Game"
        );
    }

    /// Scenario "Name and icon", for the manifest and the resources it names; `aapt2 dump
    /// badging` of a built APK is checked by hand (`docs/building.md`).
    #[test]
    fn the_app_is_labelled_rgss_player_and_has_the_launcher_icon() {
        let manifest = manifest("0.4.0");
        assert!(manifest.contains(r#"<application android:label="RGSS Player""#));
        assert_eq!(manifest.matches("android:label=").count(), 1);
        assert!(manifest.contains(r#"android:icon="@mipmap/ic_launcher""#));
        assert!(manifest.contains(r#"android:roundIcon="@mipmap/ic_launcher_round""#));
        let res = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(RESOURCES);
        for icon in ["ic_launcher", "ic_launcher_round"] {
            let adaptive =
                fs::read_to_string(res.join(format!("mipmap-anydpi-v26/{icon}.xml"))).unwrap();
            for layer in ["background", "foreground", "monochrome"] {
                assert!(
                    adaptive.contains(&format!(
                        "<{layer} android:drawable=\"@drawable/ic_launcher_{layer}\""
                    )),
                    "{icon}: {layer}"
                );
                assert!(
                    res.join(format!("drawable-nodpi/ic_launcher_{layer}.png"))
                        .is_file()
                );
            }
            for density in ["mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"] {
                let png = res.join(format!("mipmap-{density}/{icon}.png"));
                assert!(png.is_file(), "{}", png.display());
            }
        }
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
        assert!(source.contains("private static native void nativeImport(String uri, int what)"));
        // The kinds of the picker, numbered alike on both sides.
        for (java, rust) in [
            ("PICK_GAME = 1;", "Game = 1,"),
            ("PICK_RTP = 2;", "Rtp = 2,"),
            ("PICK_BUILTIN = 3;", "BuiltinScripts = 3,"),
        ] {
            assert!(source.contains(java), "{java}");
            assert!(native.contains(rust), "{rust}");
        }
        for method in [
            "gameDir",
            "validation",
            "openPicker",
            "copyToCache",
            "startGame",
        ] {
            assert!(source.contains(&format!(" {method}(")), "{method}");
            assert!(
                native.contains(&format!("jni_str!(\"{method}\")")),
                "{method}"
            );
        }
    }

    /// Scenario "Validation on the phones", for the lines of logcat a run leaves.
    #[test]
    fn a_run_under_the_validation_layer_fails_on_an_error_or_without_its_report() {
        let clean = "I vkrgss  : vkrgss: the app ends\n\
                     I vkrgss  : vkrgss: validation: the layer reported no error\n";
        validation_outcome(clean).unwrap();
        let failed = "E vkrgss  : vkrgss: validation error: [ VUID-x ] y\n\
                      E vkrgss  : vkrgss: validation error: [ VUID-z ] w\n\
                      E vkrgss  : vkrgss: validation: the layer reported 2 errors\n";
        let message = format!("{:#}", validation_outcome(failed).unwrap_err());
        assert!(message.contains("2 errors"), "{message}");
        let silent = "E vkrgss  : vkrgss: validation: the layer \
                      VK_LAYER_KHRONOS_validation was not loaded\n";
        assert!(validation_outcome(silent).is_err());
        assert!(validation_outcome("").is_err());

        let message = format!("{:#}", validation_layer(None).unwrap_err());
        assert!(message.contains(LAYER_VARIABLE), "{message}");
        let message = format!(
            "{:#}",
            validation_layer(Some("/nonexistent/layer.so".into())).unwrap_err()
        );
        assert!(message.contains("/nonexistent/layer.so"), "{message}");
        assert!(LAYER_FILE.starts_with("lib") && LAYER_FILE.ends_with(".so"));
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
