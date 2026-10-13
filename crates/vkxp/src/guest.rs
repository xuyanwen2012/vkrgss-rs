//! The guest of the program: what it needs of the binding and of the interpreter.
//!
//! The guest role itself, which is the same for an executable of any interpreter, is the
//! library `vkxp-guest`. This module is the part that names the binding and the sys crate:
//! what a guest is set up with ([`guest_inputs`], which the program also calls as a check
//! before anything starts), the guest on a thread ([`guest_on_thread`]) and the guest
//! process ([`run_guest`]), which call the library in the order its documentation gives.
//!
//! The module and `font` name nothing of the host and nothing of the command line, the
//! configuration files or the diagnostics of the program: they are what an executable of
//! another interpreter compiles against its own binding. What a guest reads of the settled
//! options is [`GuestSettings`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf, absolute};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use anyhow::{Context, Result, anyhow, bail};
use vkxp_binding::{Config, Copied, Copier, CopyFailed, RunConfig, RunError, run_game};
use vkxp_guest::copier::{ForkCopier, Parked};
use vkxp_guest::role::GuestRole;
use vkxp_guest::{Attached, play_in_process, play_on_thread, signals};
use vkxp_proto::link::GuestEnd;
use vkxp_ruby_sys::{Interpreter, SymbolTable};
use vkxp_vfs::{Encoding, GameFiles, GameIni, Runtime, VfsError, resolve_rtps};

use crate::font::FontBackend;

/// The name of the tool that creates the built-in scripts file.
const EXTRACTION_TOOL: &str = "cargo xtask extract-rpg";

/// Where it is written how the built-in scripts file of RGSS2 is made.
const RGSS2_BUILTIN_GUIDE: &str =
    "docs/running.md, section \"The built-in scripts of RPG Maker VX\"";

/// What a guest reads of the settled options of a run.
#[derive(Debug, Clone, Default)]
pub(crate) struct GuestSettings {
    /// The encoding of `Game.ini`, if one was given.
    pub(crate) encoding: Option<Encoding>,
    /// The runtime of the game, if one was given; otherwise `Game.ini` tells it.
    pub(crate) runtime: Option<Runtime>,
    /// The directory of each run-time package, by its name.
    pub(crate) rtps: BTreeMap<String, PathBuf>,
    /// The built-in scripts file, instead of the default location.
    pub(crate) builtin_scripts: Option<PathBuf>,
    /// Font directories, in the order given.
    pub(crate) font_dirs: Vec<PathBuf>,
    /// Whether a font that replaces another keeps its own line box.
    pub(crate) font_own_metrics: bool,
    /// Preload scripts, in the order given.
    pub(crate) preload: Vec<PathBuf>,
    pub(crate) debug: bool,
    /// Whether the functions that start processes do so.
    pub(crate) process_functions: bool,
    /// Whether `p` is log-only.
    pub(crate) log_p: bool,
    /// Whether the frame time statistics are reported.
    pub(crate) frame_stats: bool,
    /// Whether a guest process confines itself, if that was given.
    pub(crate) sandbox: Option<bool>,
}

/// The name of the built-in scripts file of `runtime` at the default location.
fn builtin_scripts_name(runtime: Runtime) -> &'static str {
    match runtime {
        Runtime::Rgss1 => "rpg_builtin.rb",
        Runtime::Rgss2 => "rpg_builtin_rgss2.rb",
        Runtime::Rgss3 => "rpg_builtin_rgss3.rb",
    }
}

/// The start-up symbol table of `runtime`, which numbers the symbols as its original does.
/// A game of RGSS3 ends before an interpreter is started.
fn symbol_table(runtime: Runtime) -> SymbolTable {
    match runtime {
        Runtime::Rgss1 => SymbolTable::Rgss1,
        Runtime::Rgss2 | Runtime::Rgss3 => SymbolTable::Rgss2,
    }
}

/// Starts the interpreter of a game of `runtime`.
fn start_interpreter(runtime: Runtime) -> Result<Interpreter> {
    let options = vkxp_ruby_sys::Options {
        symbols: symbol_table(runtime),
        ..vkxp_ruby_sys::Options::default()
    };
    Ok(Interpreter::start(options)?)
}

/// The built-in scripts file of `runtime`: the one named on the command line, or the default
/// under the user's data directory. Fails when it does not exist.
fn builtin_scripts(named: Option<PathBuf>, runtime: Runtime) -> Result<PathBuf> {
    let path = match named {
        Some(path) => absolute(&path)?,
        None => {
            let variable = |name| std::env::var_os(name).filter(|value| !value.is_empty());
            let data = match (variable("XDG_DATA_HOME"), variable("HOME")) {
                (Some(data), _) => PathBuf::from(data),
                (None, Some(home)) => Path::new(&home).join(".local").join("share"),
                (None, None) => bail!(
                    "cannot tell where the built-in scripts file is: neither XDG_DATA_HOME nor \
                     HOME is set. Name it with --builtin-scripts; `{EXTRACTION_TOOL} <DLL> \
                     <OUTPUT>` creates it from your own RGSS DLL"
                ),
            };
            data.join("vkxp").join(builtin_scripts_name(runtime))
        }
    };
    if !path.is_file() && runtime != Runtime::Rgss1 {
        bail!(
            "the built-in scripts file {path} does not exist. It holds the RPG scripts of the \
             original runtime of {product}, which are not part of vkxp: \
             {RGSS2_BUILTIN_GUIDE}, says how it is made from your own copy of that runtime; \
             or name another file with --builtin-scripts",
            path = path.display(),
            product = runtime.product()
        );
    }
    if !path.is_file() {
        bail!(
            "the built-in scripts file {path} does not exist. It holds the RPG scripts of the \
             original runtime, which are not part of vkxp: create it from your own RGSS DLL \
             with `{EXTRACTION_TOOL} <DLL> {path}`, or name another file with \
             --builtin-scripts",
            path = path.display()
        );
    }
    Ok(path)
}

/// The encrypted archive of the game in `game_dir`, which the game's files are opened with:
/// a file with the archive name of `runtime` under any letter case.
fn archive(game_dir: &Path, runtime: Runtime) -> Option<PathBuf> {
    let archive_name = runtime.archive_name()?;
    std::fs::read_dir(game_dir)
        .ok()?
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.eq_ignore_ascii_case(archive_name))
        })
        .map(|entry| entry.path())
        .find(|path| path.is_file())
}

/// The names `Font.default_name` starts with. The original runtime's Japanese edition, which
/// a trailing `J` in the name of the game's library stands for, starts with another font.
fn default_font_names(library: &str) -> Vec<String> {
    let stem = library.rsplit_once('.').map_or(library, |(stem, _)| stem);
    let name = if stem.ends_with(['J', 'j']) {
        "ＭＳ Ｐゴシック"
    } else {
        "Arial"
    };
    vec![name.to_owned()]
}

/// What the guest of a run is set up with, as far as it is looked up before any game file
/// is opened.
pub(crate) struct GuestInputs {
    pub(crate) ini: GameIni,
    pub(crate) runtime: Runtime,
    rtp_dirs: Vec<PathBuf>,
    builtin_scripts: PathBuf,
    font_dirs: Vec<PathBuf>,
    preload: Vec<PathBuf>,
}

/// The runtime of the game whose configuration file is `ini`: the one the option names, and
/// without the option what the file says. A game of RGSS3 is refused here.
fn runtime_of(ini: &GameIni, named: Option<Runtime>) -> Result<Runtime> {
    let runtime = match named {
        Some(runtime) => runtime,
        None => Runtime::detect(ini).map_err(|error| {
            anyhow!("{error}; name the runtime with --runtime rgss1, rgss2 or rgss3")
        })?,
    };
    if runtime == Runtime::Rgss3 {
        bail!(
            "this is a game of {} ({}), which is planned and not supported yet",
            runtime.product(),
            runtime.name()
        );
    }
    Ok(runtime)
}

/// Reads `Game.ini`, settles the runtime of the game, looks the run-time packages it names
/// up in the table of the settings and finds the built-in scripts file.
pub(crate) fn guest_inputs(game_dir: &Path, settings: &GuestSettings) -> Result<GuestInputs> {
    let ini = GameIni::read(game_dir, settings.encoding)?;
    let runtime = runtime_of(&ini, settings.runtime)?;
    tracing::info!("runtime: {} ({})", runtime.name(), runtime.product());
    let rtp_table = settings
        .rtps
        .iter()
        .map(|(name, dir)| Ok((name.clone(), absolute(dir)?)))
        .collect::<Result<_>>()?;
    let rtp_dirs =
        resolve_rtps(ini.rtp_names(runtime), &rtp_table).map_err(|error| match error {
            VfsError::UnknownRtp { name } => anyhow!(
                "the game needs the run-time package `{name}`; pass its directory with --rtp \
             {name}=DIR or name it under [rtp] in a configuration file"
            ),
            other => other.into(),
        })?;
    for dir in &rtp_dirs {
        tracing::info!("run-time package directory: {}", dir.display());
    }
    let builtin_scripts = builtin_scripts(settings.builtin_scripts.clone(), runtime)?;
    tracing::info!("built-in scripts file: {}", builtin_scripts.display());
    let font_dirs = settings
        .font_dirs
        .iter()
        .map(absolute)
        .collect::<std::io::Result<Vec<_>>>()?;
    let preload = settings
        .preload
        .iter()
        .map(absolute)
        .collect::<std::io::Result<Vec<_>>>()?;
    Ok(GuestInputs {
        ini,
        runtime,
        rtp_dirs,
        builtin_scripts,
        font_dirs,
        preload,
    })
}

/// Opens the game's files, enters the game directory, blocks SIGVTALRM in the calling
/// thread, which has to be the only one still, and builds the binding's configuration.
fn guest_config(
    game_dir: &Path,
    settings: &GuestSettings,
    inputs: GuestInputs,
    seed: u64,
) -> Result<Config> {
    let GuestInputs {
        ini,
        runtime,
        rtp_dirs,
        builtin_scripts,
        font_dirs,
        preload,
    } = inputs;
    let files = GameFiles::open(game_dir.to_owned(), rtp_dirs, runtime)?;
    if let Some(archive) = archive(game_dir, runtime) {
        tracing::info!("encrypted archive: {}", archive.display());
    }

    std::env::set_current_dir(game_dir)
        .with_context(|| format!("cannot enter the game directory {}", game_dir.display()))?;
    signals::block_virtual_timer();

    Ok(Config {
        files,
        font_names: default_font_names(&ini.library),
        text: Box::new(FontBackend::for_game(
            game_dir,
            &font_dirs,
            settings.font_own_metrics,
        )),
        run: RunConfig {
            ini,
            runtime,
            builtin_scripts,
            debug: settings.debug,
            process_functions: settings.process_functions,
            log_p: settings.log_p,
            frame_stats: settings.frame_stats,
            seed: Some(seed),
            preload,
            ..RunConfig::default()
        },
    })
}

/// Whether a guest process confines itself: unless the setting says no, and never with the
/// functions that start processes.
fn confined(sandbox: Option<bool>, process_functions: bool) -> bool {
    sandbox.unwrap_or(true) && !process_functions
}

/// Where the guest on a thread reports how its game went.
pub(crate) type Outcome = Receiver<Result<(), RunError>>;

/// The copier of the library as the copier of the binding.
#[derive(Debug)]
struct ProcessCopier(ForkCopier);

impl Copier for ProcessCopier {
    fn park(&self, slot: u8) -> Result<Copied, CopyFailed> {
        match self.0.park(slot) {
            Ok(Parked::WentOn) => Ok(Copied::WentOn),
            Ok(Parked::Resumed(link)) => Ok(Copied::Resumed(link)),
            Err(_) => Err(CopyFailed),
        }
    }
}

/// The guest on a thread: starts the interpreter for the game in `game_dir` and returns
/// what the guest thread does with its link, and where that reports how the game went. It
/// sends the game to the interpreter thread, waits for it, reports and lets go of the
/// interpreter.
pub(crate) fn guest_on_thread(
    game_dir: &Path,
    settings: &GuestSettings,
    inputs: GuestInputs,
    seed: u64,
) -> Result<(impl FnOnce(GuestEnd) + Send + 'static, Outcome)> {
    let runtime = inputs.runtime;
    let config = guest_config(game_dir, settings, inputs, seed)?;
    let interpreter = start_interpreter(runtime)?;
    let (done, outcome) = mpsc::channel();
    let guest = move |link: GuestEnd| {
        let result = interpreter.run(move |ruby| play_on_thread(|| run_game(ruby, config, link)));
        // Nobody listens once the program has given up waiting.
        let _ = done.send(result);
        drop(interpreter);
    };
    Ok((guest, outcome))
}

/// The guest role: plays the game in `game_dir` over the socket of the parent process.
/// `Ok(false)` is a game that failed and has said why.
pub(crate) fn run_guest(
    game_dir: PathBuf,
    settings: &GuestSettings,
    role: &GuestRole,
) -> Result<bool> {
    signals::die_with_parent();
    let inputs = guest_inputs(&game_dir, settings)?;
    let runtime = inputs.runtime;
    let mut config = guest_config(&game_dir, settings, inputs, role.seed)?;
    let interpreter = start_interpreter(runtime)?;
    let attached = Attached::connect(role)?;
    // The functions that start processes need the children of this process to be waited
    // for, and states need them not to be: with those functions there are no states.
    if !settings.process_functions {
        let copier = attached.copier(vkxp_ruby_sys::restart_thread_timer);
        let copier: Arc<dyn Copier> = Arc::new(ProcessCopier(copier));
        config.run.copier = Some(copier);
    }
    let confined = confined(settings.sandbox, settings.process_functions);
    if settings.process_functions {
        // A program the game starts would inherit the restriction. Said whatever the
        // setting of the sandbox is.
        tracing::info!("the guest process is not confined: --allow-process-functions is given");
    }
    // The interpreter thread reads and writes the socket itself.
    let link = attached.into_link();
    let ok = interpreter.run(move |ruby| {
        play_in_process(confined.then_some(game_dir.as_path()), || {
            game_succeeded(run_game(ruby, config, link))
        })
    });
    drop(interpreter);
    Ok(ok)
}

/// Whether the game ended without a failure. A failure that is not on the error stream yet
/// is written there.
pub(crate) fn game_succeeded(result: Result<(), RunError>) -> bool {
    match result {
        Ok(()) => true,
        // The report of a script error is on the error stream already.
        Err(RunError::Script(_)) => false,
        Err(error) => {
            eprintln!("vkxp: {error}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_font_follows_the_edition_of_the_library() {
        assert_eq!(default_font_names("RGSS104E.dll"), ["Arial"]);
        assert_eq!(default_font_names("RGSS102J.dll"), ["ＭＳ Ｐゴシック"]);
        assert_eq!(default_font_names("rgss103j"), ["ＭＳ Ｐゴシック"]);
        assert_eq!(default_font_names(""), ["Arial"]);
    }

    /// Requirement "Sandbox option": the default, and the functions that start processes.
    #[test]
    fn a_guest_process_is_confined_unless_told_otherwise() {
        assert!(confined(None, false));
        assert!(confined(Some(true), false));
        assert!(!confined(Some(false), false));
        for sandbox in [None, Some(true), Some(false)] {
            assert!(!confined(sandbox, true));
        }
    }

    #[test]
    fn a_missing_built_in_scripts_file_names_the_tool() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("absent.rb");
        let error = builtin_scripts(Some(absent.clone()), Runtime::Rgss1).unwrap_err();
        let message = error.to_string();
        assert!(message.contains(EXTRACTION_TOOL), "{message}");
        assert!(message.contains("absent.rb"), "{message}");
        // The file of RGSS2 is not made from the DLL file.
        let error = builtin_scripts(Some(absent), Runtime::Rgss2).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("docs/running.md"), "{message}");
        assert!(message.contains("absent.rb"), "{message}");
    }

    fn ini(library: &str, scripts: &str) -> GameIni {
        let text = format!("[Game]\nLibrary={library}\nScripts={scripts}\n");
        GameIni::parse(text.as_bytes(), None).unwrap()
    }

    /// Requirement "Runtime of the run".
    #[test]
    fn the_runtime_is_the_one_named_or_the_one_the_file_tells() {
        let xp = ini("RGSS104E.dll", "Data\\Scripts.rxdata");
        assert_eq!(runtime_of(&xp, None).unwrap(), Runtime::Rgss1);
        assert_eq!(
            runtime_of(&xp, Some(Runtime::Rgss2)).unwrap(),
            Runtime::Rgss2
        );

        let conflict = ini("RGSS104E.dll", "Data\\Scripts.rvdata");
        let message = runtime_of(&conflict, None).unwrap_err().to_string();
        assert!(message.contains("--runtime"), "{message}");
        assert!(message.contains("RGSS104E.dll"), "{message}");
        assert!(message.contains("Data\\Scripts.rvdata"), "{message}");
        let named = runtime_of(&conflict, Some(Runtime::Rgss2));
        assert_eq!(named.unwrap(), Runtime::Rgss2);
    }

    /// Requirement "VX Ace is refused".
    #[test]
    fn a_game_of_vx_ace_is_refused_however_its_runtime_is_known() {
        let ace = ini("System\\RGSS301.dll", "Data\\Scripts.rvdata2");
        let xp = ini("RGSS104E.dll", "Data\\Scripts.rxdata");
        for (ini, named) in [(&ace, None), (&xp, Some(Runtime::Rgss3))] {
            let message = runtime_of(ini, named).unwrap_err().to_string();
            assert!(message.contains("RPG Maker VX Ace"), "{message}");
            assert!(message.contains("not supported"), "{message}");
            assert!(message.contains("planned"), "{message}");
        }
    }

    #[test]
    fn an_existing_built_in_scripts_file_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rpg.rb");
        std::fs::write(&file, "module RPG;end\n").unwrap();
        for runtime in [Runtime::Rgss1, Runtime::Rgss2] {
            assert_eq!(builtin_scripts(Some(file.clone()), runtime).unwrap(), file);
        }
    }
}
