//! `android_main`: the steps of the crate's documentation.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use vkrgss_guest::diagnostics::{self, Level};
use vkrgss_guest::record::Diagnostics;
use vkrgss_guest::settings::GuestSettings;
use vkrgss_guest_main::{builtin_scripts_name, guest_inputs};
use vkrgss_host::{AudioOptions, InputSource, KeyMap, WindowedOptions, run_windowed_on_android};
use vkrgss_proto::link::GuestEnd;
use vkrgss_vfs::{GameIni, Runtime};
use vkrgss_vk::ValidationLog;
use winit::platform::android::activity::{AndroidApp, MainEvent, PollEvent, WindowManagerFlags};

use crate::buttons::TouchButtons;
use crate::input::ScriptedInput;
use crate::java::Activity;
use crate::library::Library;
use crate::options::Options;
use crate::process::{GuestProcess, Handed};
use crate::{library_app, log};

/// The directory of the game in the files directory, for a game activity started without a
/// game.
const GAME_DIR: &str = "game";

/// The file of the log of a game in the files directory.
const LOG_FILE: &str = "vkrgss.log";

/// The file of the log of the library in the files directory.
const LIBRARY_LOG_FILE: &str = "library.log";

/// The fonts of the system.
const SYSTEM_FONTS: &str = "/system/fonts";

/// The level of the diagnostics of the guest process, the app's own.
const GUEST_LEVEL: &str = "info";

/// How long the app waits for the activity to be destroyed once it has finished it.
const FINISH_WAIT: Duration = Duration::from_secs(5);

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    // The activity, and the directory of its game: `None` for the library, empty for the
    // game of the files directory.
    let role = Activity::new(&app).and_then(|activity| {
        let game = activity.game_dir()?;
        Ok((activity, game))
    });
    let files = app.external_data_path();
    let log_name = match &role {
        Ok((_, Some(_))) => LOG_FILE,
        _ => LIBRARY_LOG_FILE,
    };
    let log_file = files.as_deref().map(|files| files.join(log_name));
    // Without the pipe the lines are lost; the app runs all the same.
    let forwarding = log::start(log_file.as_deref()).ok();
    diagnostics::install(Level {
        filter: EnvFilter::default().add_directive(LevelFilter::INFO.into()),
        detailed: false,
    });
    tracing::info!("vkrgss {} starts on Android", env!("CARGO_PKG_VERSION"));
    let succeeded = match (&role, app.internal_data_path()) {
        (Err(error), _) => {
            tracing::error!("cannot reach the activity: {error:#}");
            false
        }
        (_, None) => {
            tracing::error!(
                "the app has no files directory, where the library and the socket of the guest \
                 would be"
            );
            false
        }
        (Ok((activity, None)), Some(private)) => {
            tracing::info!("the library starts");
            library_app::run(app.clone(), activity.clone(), &private, SYSTEM_FONTS.into())
                .map(|()| true)
                .unwrap_or_else(|error| {
                    tracing::error!("{error:#}");
                    false
                })
        }
        (Ok((activity, Some(game))), Some(private)) => match files {
            Some(files) => {
                let game = match game.as_os_str().is_empty() {
                    true => files.join(GAME_DIR),
                    false => game.clone(),
                };
                run(app.clone(), activity, &game, &files, &private).unwrap_or_else(|error| {
                    tracing::error!("{error:#}");
                    false
                })
            }
            None => {
                tracing::error!(
                    "the app has no external files directory, where the built-in scripts would be"
                );
                false
            }
        },
    };
    tracing::info!("the app ends");
    if let Ok((activity, _)) = &role {
        finish(&app, activity);
    }
    if let Some(forwarding) = forwarding {
        forwarding.finish();
    }
    // The process ends with the activity, so that the next start of the activity starts
    // afresh.
    std::process::exit(if succeeded { 0 } else { 1 });
}

/// Finishes the activity and waits until it has been destroyed: the system starts an
/// activity again whose process ends while it is still there.
fn finish(app: &AndroidApp, activity: &Activity) {
    if let Err(error) = activity.finish() {
        tracing::error!("cannot finish the activity: {error:#}");
        return;
    }
    let deadline = Instant::now() + FINISH_WAIT;
    let mut destroyed = false;
    while !destroyed && Instant::now() < deadline {
        app.poll_events(Some(Duration::from_millis(100)), |event| {
            destroyed |= matches!(event, PollEvent::Main(MainEvent::Destroy));
        });
    }
}

/// Runs the game of `game_dir`, with the built-in scripts by default in the external files
/// directory `files` or imported into the library, and the socket of the guest process in
/// the app's private directory `private`, where the library is. `Ok(false)` is a run that failed and has said why.
fn run(
    app: AndroidApp,
    activity: &Activity,
    game_dir: &Path,
    files: &Path,
    private: &Path,
) -> Result<bool> {
    let game_dir = game_dir.to_owned();
    tracing::info!("the game is read from {}", game_dir.display());
    let options = Options::read(&game_dir)?;
    let runtime = Runtime::detect(&GameIni::read(&game_dir, None)?)?;
    let mut font_dirs = options.font_dirs;
    font_dirs.push(SYSTEM_FONTS.into());
    // The imported run-time packages, and those the file names, which win.
    let library = Library::new(private);
    let mut rtps = library.rtps();
    rtps.extend(options.rtps);
    // The file the options name, else that of the external files directory, which tests
    // fill, else the one imported into the library.
    let builtin_scripts = options.builtin_scripts.unwrap_or_else(|| {
        let pushed = files.join(builtin_scripts_name(runtime));
        match pushed.is_file() {
            true => pushed,
            false => library.builtin_scripts(runtime),
        }
    });
    tracing::info!(
        "the built-in scripts are read from {}",
        builtin_scripts.display()
    );
    let settings = GuestSettings {
        rtps,
        builtin_scripts: Some(builtin_scripts),
        font_dirs,
        font_own_metrics: options.font_own_metrics,
        debug: options.debug,
        log_p: options.log_p,
        ..GuestSettings::default()
    };
    // What the guest process checks again; a game that lacks something fails here, before
    // the guest process and the window exist.
    let inputs = guest_inputs(&game_dir, &settings)?;
    let title = inputs.ini.title.clone();
    let frame = vkrgss_binding::frame_size(runtime);
    let seed = options.seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            // The low 64 bits of the nanoseconds; a clock before 1970 gives 0.
            .map_or(0, |since| since.as_nanos() as u64)
    });
    let handed = Handed {
        seed,
        game_dir: &game_dir,
        settings: &settings,
        diagnostics: Diagnostics {
            variable: Some(GUEST_LEVEL.to_owned()),
            verbose: false,
        },
    };
    std::fs::create_dir_all(private)
        .with_context(|| format!("cannot create the directory {}", private.display()))?;
    let process = GuestProcess::start(runtime, private, handed)?;
    let (done, outcome) = mpsc::channel();
    let guest = move |link: GuestEnd| {
        let _ = done.send(process.relay(link));
    };

    // Under the validation layer when the intent asks for it, with what it reports written
    // to the log once the window has gone.
    let validation = activity.validation()?.then(ValidationLog::new);
    let insets = activity.clone();
    let buttons = TouchButtons::new(frame, vec![PathBuf::from(SYSTEM_FONTS)], move || {
        insets.insets().unwrap_or_default()
    });
    let host_options = WindowedOptions {
        title,
        logical_size: frame,
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
        states: true,
        frames: options.frames,
        dumps: options.dumps,
        overlay: Some(Box::new(buttons)),
        validation: validation.clone(),
        ..WindowedOptions::new(Default::default())
    };
    // A game is watched more than it is touched, and a screen that goes off sends the
    // activity to the background, where the game waits; the flag goes with the activity.
    app.set_window_flags(
        WindowManagerFlags::KEEP_SCREEN_ON,
        WindowManagerFlags::empty(),
    );
    // The windowed runner returns once the guest thread, the relay, has ended.
    let end = run_windowed_on_android(app, host_options, guest)?;
    let game_ok = outcome.recv().unwrap_or_else(|_| {
        tracing::error!("the guest thread panicked");
        false
    });
    if !end.is_success() {
        tracing::error!("the session ended without the game shutting down ({end:?})");
    }
    let valid = validation.as_ref().is_none_or(report_validation);
    Ok(end.is_success() && game_ok && valid)
}

/// Writes what the validation layer reported to the log, each error on a line that starts
/// with `validation error:`, and whether it found none. False when it reported one, or
/// when the layer was not loaded.
fn report_validation(log: &ValidationLog) -> bool {
    if !log.is_active() {
        tracing::error!(
            "validation: the layer VK_LAYER_KHRONOS_validation was not loaded; the APK of \
             `cargo xtask android-run --validation` carries it"
        );
        return false;
    }
    let errors = log.errors();
    for error in &errors {
        tracing::error!("validation error: {error}");
    }
    match errors.len() {
        0 => tracing::info!("validation: the layer reported no error"),
        count => tracing::error!("validation: the layer reported {count} errors"),
    }
    errors.is_empty()
}
