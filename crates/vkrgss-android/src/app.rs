//! `android_main`: the steps of the crate's documentation.

use std::ffi::c_void;
use std::path::Path;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use vkrgss_guest::diagnostics::{self, Level};
use vkrgss_guest::settings::GuestSettings;
use vkrgss_guest_main::{builtin_scripts_name, game_succeeded, guest_inputs, guest_on_thread};
use vkrgss_host::{AudioOptions, InputSource, KeyMap, WindowedOptions, run_windowed_on_android};
use vkrgss_vfs::{GameIni, Runtime};
use winit::platform::android::activity::{AndroidApp, MainEvent, PollEvent};

use crate::input::ScriptedInput;
use crate::log;
use crate::options::Options;

/// The directory of the game in the files directory.
const GAME_DIR: &str = "game";

/// The file of the log in the files directory.
const LOG_FILE: &str = "vkrgss.log";

/// The fonts of the system.
const SYSTEM_FONTS: &str = "/system/fonts";

/// How long the app waits for the game's scripts to end after the host has.
const GUEST_WAIT: Duration = Duration::from_secs(5);

/// How long the app waits for the activity to be destroyed once it has finished it.
const FINISH_WAIT: Duration = Duration::from_secs(5);

#[link(name = "android")]
unsafe extern "C" {
    fn ANativeActivity_finish(activity: *mut c_void);
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let files = app.external_data_path();
    let log_file = files.as_deref().map(|files| files.join(LOG_FILE));
    // Without the pipe the lines are lost; the game runs all the same.
    let forwarding = log::start(log_file.as_deref()).ok();
    diagnostics::install(Level {
        filter: EnvFilter::default().add_directive(LevelFilter::INFO.into()),
        detailed: false,
    });
    tracing::info!("vkrgss {} starts on Android", env!("CARGO_PKG_VERSION"));
    let succeeded = match files {
        Some(files) => run(app.clone(), &files).unwrap_or_else(|error| {
            tracing::error!("{error:#}");
            false
        }),
        None => {
            tracing::error!("the app has no external files directory, where the game would be");
            false
        }
    };
    tracing::info!("the app ends");
    finish(&app);
    if let Some(forwarding) = forwarding {
        forwarding.finish();
    }
    // The interpreter cannot be started twice in one process, so the process ends with
    // the activity.
    std::process::exit(if succeeded { 0 } else { 1 });
}

/// Finishes the activity and waits until it has been destroyed: the system starts an
/// activity again whose process ends while it is still there.
fn finish(app: &AndroidApp) {
    // SAFETY: the pointer is that of the app's `ANativeActivity`, which lives until the
    // activity has been destroyed; the call may be made from any thread.
    unsafe { ANativeActivity_finish(app.activity_as_ptr()) };
    let deadline = Instant::now() + FINISH_WAIT;
    let mut destroyed = false;
    while !destroyed && Instant::now() < deadline {
        app.poll_events(Some(Duration::from_millis(100)), |event| {
            destroyed |= matches!(event, PollEvent::Main(MainEvent::Destroy));
        });
    }
}

/// Runs the game of the files directory `files`. `Ok(false)` is a run that failed and has
/// said why.
fn run(app: AndroidApp, files: &Path) -> Result<bool> {
    let game_dir = files.join(GAME_DIR);
    tracing::info!("the game is read from {}", game_dir.display());
    let options = Options::read(&game_dir)?;
    let runtime = Runtime::detect(&GameIni::read(&game_dir, None)?)?;
    if let Some(refusal) = crate::refusal(runtime) {
        tracing::error!("{refusal}");
        return Ok(false);
    }
    let mut font_dirs = options.font_dirs;
    font_dirs.push(SYSTEM_FONTS.into());
    let settings = GuestSettings {
        rtps: options.rtps,
        builtin_scripts: Some(
            options
                .builtin_scripts
                .unwrap_or_else(|| files.join(builtin_scripts_name(runtime))),
        ),
        font_dirs,
        debug: options.debug,
        log_p: options.log_p,
        ..GuestSettings::default()
    };
    let inputs = guest_inputs(&game_dir, &settings)?;
    let title = inputs.ini.title.clone();
    let seed = options.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            // The low 64 bits of the nanoseconds; a clock before 1970 gives 0.
            .map_or(0, |since| since.as_nanos() as u64)
    });
    tracing::info!("the guest is a thread and is not confined");
    let (guest, outcome) = guest_on_thread(&game_dir, &settings, inputs, seed)?;

    let host_options = WindowedOptions {
        title,
        logical_size: vkrgss_binding::frame_size(runtime),
        // The keyboard maps to the buttons as the original runtime of the game does.
        keys: match runtime {
            Runtime::Rgss1 => KeyMap::default(),
            Runtime::Rgss2 | Runtime::Rgss3 => KeyMap::vx(),
        },
        audio: (!options.no_audio).then_some(AudioOptions {
            soundfont: options.soundfont,
        }),
        input: options
            .input
            .map(|script| Box::new(ScriptedInput::new(script)) as Box<dyn InputSource>),
        frames: options.frames,
        dumps: options.dumps,
        ..WindowedOptions::new(Default::default())
    };
    let end = run_windowed_on_android(app, host_options, guest)?;
    let game_ok = match outcome.recv_timeout(GUEST_WAIT) {
        Ok(result) => game_succeeded(result),
        Err(RecvTimeoutError::Timeout) => {
            tracing::error!(
                "the game's scripts did not end within {} seconds of being asked to quit",
                GUEST_WAIT.as_secs()
            );
            false
        }
        Err(RecvTimeoutError::Disconnected) => {
            tracing::error!("the guest thread panicked");
            false
        }
    };
    if !end.is_success() {
        tracing::error!("the session ended without the game shutting down ({end:?})");
    }
    Ok(end.is_success() && game_ok)
}
