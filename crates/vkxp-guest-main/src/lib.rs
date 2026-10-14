//! The main of a guest: what a guest does with its binding and its interpreter.
//!
//! Side: guest.
//!
//! The guest role itself, which is the same for an executable of any interpreter, is the
//! library `vkxp-guest`. This crate is the part that names the binding and the sys crate:
//! what a guest is set up with ([`guest_inputs`], which the program also calls as a check
//! before anything starts), the text backend over the font engine (`font`), the guest on a
//! thread ([`guest_on_thread`]) and the guest process ([`guest_process`]), which call the
//! library in the order its documentation gives.
//!
//! It is the body of the `main` of a guest executable, kept in a library so that every such
//! executable has it: the first statement of `main` is a call of [`guest_process`]. The
//! crate names nothing of the host and nothing of the command line or the configuration
//! files of the program, and it names the binding and the sys crate only as `vkxp_binding`
//! and `vkxp_ruby_sys`: an executable of another interpreter compiles the same source
//! against its own. What a guest reads of the settled options is `GuestSettings` of
//! `vkxp-guest`. Its errors are written to the error stream with their causes and matched
//! by nobody, so it uses `anyhow`.

#![cfg(not(vkxp_interpreter_absent))]
#![forbid(unsafe_code)]

mod font;

use std::path::{Path, PathBuf, absolute};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use anyhow::{Context, Result, anyhow, bail};
use vkxp_binding::{Config, Copied, Copier, CopyFailed, RunConfig, RunError, run_game};
use vkxp_guest::copier::{ForkCopier, Parked};
use vkxp_guest::record::{Diagnostics, GuestRecord};
use vkxp_guest::role::{self, GuestRole};
use vkxp_guest::settings::GuestSettings;
use vkxp_guest::{Attached, diagnostics, play_in_process, play_on_thread, signals};
use vkxp_proto::link::GuestEnd;
use vkxp_ruby_sys::{Interpreter, Options, RUBY_API, RubyApi};
use vkxp_vfs::{FileKind, GameFiles, GameIni, LibraryVersion, Runtime, VfsError, resolve_rtps};

use crate::font::FontBackend;

/// The name of the tool that creates the built-in scripts file.
const EXTRACTION_TOOL: &str = "cargo xtask extract-rpg";

/// Where it is written how the built-in scripts file of RGSS2 is made.
const RGSS2_BUILTIN_GUIDE: &str =
    "docs/running.md, section \"The built-in scripts of RPG Maker VX\"";

/// Where it is written how the built-in scripts file of RGSS3 is made.
const RGSS3_BUILTIN_GUIDE: &str =
    "docs/running.md, section \"The built-in scripts of RPG Maker VX Ace\"";

/// The name of the built-in scripts file of `runtime` at the default location.
pub fn builtin_scripts_name(runtime: Runtime) -> &'static str {
    match runtime {
        Runtime::Rgss1 => "rpg_builtin.rb",
        Runtime::Rgss2 => "rpg_builtin_rgss2.rb",
        Runtime::Rgss3 => "rpg_builtin_rgss3.rb",
    }
}

/// Starts the interpreter of a game of `runtime`, with the options of its interpreter crate
/// for that runtime: over Ruby 1.8.7 the start-up symbol table, which numbers the symbols as
/// the original does.
fn start_interpreter(runtime: Runtime) -> Result<Interpreter> {
    let generation = match runtime {
        Runtime::Rgss1 => 1,
        Runtime::Rgss2 => 2,
        Runtime::Rgss3 => 3,
    };
    Ok(Interpreter::start(Options::for_rgss(generation))?)
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
    if path.is_file() {
        return Ok(path);
    }
    let guide = match runtime {
        Runtime::Rgss1 => bail!(
            "the built-in scripts file {path} does not exist. It holds the RPG scripts of the \
             original runtime, which are not part of vkxp: create it from your own RGSS DLL \
             with `{EXTRACTION_TOOL} <DLL> {path}`, or name another file with \
             --builtin-scripts",
            path = path.display()
        ),
        Runtime::Rgss2 => RGSS2_BUILTIN_GUIDE,
        Runtime::Rgss3 => RGSS3_BUILTIN_GUIDE,
    };
    bail!(
        "the built-in scripts file {path} does not exist. It holds the RPG scripts of the \
         original runtime of {product}, which are not part of vkxp: {guide}, says how it is \
         made from your own copy of that runtime; or name another file with \
         --builtin-scripts",
        path = path.display(),
        product = runtime.product()
    )
}

/// The encrypted archive of the game in `game_dir`, which the game's files are opened with:
/// a file with the archive name of `runtime` under any letter case.
fn archive(game_dir: &Path, runtime: Runtime) -> Option<PathBuf> {
    let archive_name = runtime.archive_name();
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

/// The version of the DLL that `library`, the `Library` of the game's configuration file,
/// names, found in the game directory as a data file is (letter case and `\` accepted,
/// never in the archive), and the line of the log that says which file gave which version,
/// or why there is none: an empty `Library`, a file that is not found or cannot be read,
/// and bytes without a version record. The file's name does not count.
fn library_version(files: &GameFiles, library: &str) -> (Option<LibraryVersion>, String) {
    if library.trim().is_empty() {
        return (
            None,
            "no library version: Game.ini names no Library".to_owned(),
        );
    }
    let path = match files.resolve(FileKind::Data, library) {
        Ok(path) => path,
        Err(_) => {
            let line = format!("no library version: {library} is not in the game directory");
            return (None, line);
        }
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let line = format!(
                "no library version: {} cannot be read: {error}",
                path.display()
            );
            return (None, line);
        }
    };
    match LibraryVersion::read(&bytes) {
        Some(version) => {
            let line = format!("library version: {version} ({})", path.display());
            (Some(version), line)
        }
        None => {
            let line = format!(
                "no library version: {} has no version record",
                path.display()
            );
            (None, line)
        }
    }
}

/// What the guest of a run is set up with, as far as it is looked up before any game file
/// is opened.
pub struct GuestInputs {
    /// The configuration file of the game.
    pub ini: GameIni,
    /// The runtime of the game.
    pub runtime: Runtime,
    rtp_dirs: Vec<PathBuf>,
    builtin_scripts: PathBuf,
    font_dirs: Vec<PathBuf>,
    preload: Vec<PathBuf>,
}

/// The runtime of the game whose configuration file is `ini`: the one the option names, and
/// without the option what the file says.
fn runtime_of(ini: &GameIni, named: Option<Runtime>) -> Result<Runtime> {
    match named {
        Some(runtime) => Ok(runtime),
        None => Runtime::detect(ini).map_err(|error| {
            anyhow!("{error}; name the runtime with --runtime rgss1, rgss2 or rgss3")
        }),
    }
}

/// Whether the interpreter of this executable runs the games of `runtime`: Ruby 1.8.7 those
/// of RGSS1 and RGSS2, Ruby 1.9.2 those of RGSS3.
pub fn interpreter_runs(runtime: Runtime) -> bool {
    let needed = match runtime {
        Runtime::Rgss1 | Runtime::Rgss2 => RubyApi::Ruby18,
        Runtime::Rgss3 => RubyApi::Ruby19,
    };
    RUBY_API == needed
}

/// The name of the interpreter of `api`.
fn interpreter_name(api: RubyApi) -> &'static str {
    match api {
        RubyApi::Ruby18 => "Ruby 1.8.7",
        RubyApi::Ruby19 => "Ruby 1.9.2",
    }
}

/// Ends a run of a game whose runtime the interpreter of this executable does not run, and
/// does nothing for another.
fn refuse_other_interpreter(runtime: Runtime) -> Result<()> {
    if interpreter_runs(runtime) {
        return Ok(());
    }
    let needed = match RUBY_API {
        RubyApi::Ruby18 => RubyApi::Ruby19,
        RubyApi::Ruby19 => RubyApi::Ruby18,
    };
    bail!(
        "a game of {} ({}) runs over {}, and this executable has {}",
        runtime.product(),
        runtime.name(),
        interpreter_name(needed),
        interpreter_name(RUBY_API)
    )
}

/// The title of a game of RGSS3 whose `Game.ini` has no `Title` or an empty one, measured
/// on the original runtime (`ace-v1-api-REPORT.md` section 1.1).
const UNTITLED_RGSS3: &str = "Untitled";

/// Reads `Game.ini`, settles the runtime of the game, looks the run-time packages it names
/// up in the table of the settings and finds the built-in scripts file. A game of RGSS3
/// without a title gets the title [`UNTITLED_RGSS3`], which the window and the guest take;
/// the file itself is not changed.
pub fn guest_inputs(game_dir: &Path, settings: &GuestSettings) -> Result<GuestInputs> {
    let mut ini = GameIni::read(game_dir, settings.encoding)?;
    let runtime = runtime_of(&ini, settings.runtime)?;
    if runtime == Runtime::Rgss3 && ini.title.is_empty() {
        ini.title = UNTITLED_RGSS3.to_owned();
    }
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

/// Opens the game's files, reads the version of the DLL of a game of RGSS3
/// ([`library_version`]), enters the game directory, blocks SIGVTALRM in the calling
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
    // Only RGSS3 has a rule by the version: the glyph rule and `RGSS_VERSION`.
    let version = if runtime == Runtime::Rgss3 {
        let (version, line) = library_version(&files, &ini.library);
        tracing::info!("{line}");
        version
    } else {
        None
    };
    let text = FontBackend::for_game(
        runtime,
        game_dir,
        files.rtp_dirs(),
        &font_dirs,
        settings.font_own_metrics,
        version,
    );

    std::env::set_current_dir(game_dir)
        .with_context(|| format!("cannot enter the game directory {}", game_dir.display()))?;
    signals::block_virtual_timer();

    Ok(Config {
        files,
        font_names: default_font_names(&ini.library),
        text: Box::new(text),
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
            library_version: version,
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
pub type Outcome = Receiver<Result<(), RunError>>;

/// The copier of the library as the copier of the binding.
#[derive(Debug)]
struct ProcessCopier(ForkCopier);

impl Copier for ProcessCopier {
    fn park(&self, slot: u8) -> Result<Copied, CopyFailed> {
        // A thread of the interpreter that is a thread of the system is not in a copy.
        if vkxp_ruby_sys::threads_lost_in_a_copy() {
            tracing::warn!(
                "the state of slot {slot} is not saved: a script thread is alive, and a copy \
                 of the process would not have it"
            );
            return Err(CopyFailed);
        }
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
pub fn guest_on_thread(
    game_dir: &Path,
    settings: &GuestSettings,
    inputs: GuestInputs,
    seed: u64,
) -> Result<(impl FnOnce(GuestEnd) + Send + 'static, Outcome)> {
    let runtime = inputs.runtime;
    refuse_other_interpreter(runtime)?;
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

/// Makes the executable that calls it the guest process of a program, when the first of its
/// arguments is the hidden option of the guest role; `None` otherwise, and the executable
/// goes on as what it is. `version` is the package version of the executable.
///
/// A guest process reads the guest record the option names, installs the subscriber of the
/// diagnostics from it and plays the game of the record over the socket of the program. It
/// reads nothing else of its command line and no configuration file. The status is 0 for a
/// game that ended without a failure and 1 otherwise; a record that cannot be used and a
/// start that failed are a line on the error stream first.
pub fn guest_process(version: &str) -> Option<ExitCode> {
    let record = role::record_path(std::env::args_os().skip(1))?
        .and_then(|path| GuestRecord::read(&path, version));
    let fatal = |message: &dyn std::fmt::Display| {
        eprintln!("vkxp: {message}");
        Some(ExitCode::FAILURE)
    };
    let GuestRecord {
        role,
        game_dir,
        settings,
        diagnostics: Diagnostics { variable, verbose },
    } = match record {
        Ok(record) => record,
        Err(error) => return fatal(&error),
    };
    // Nothing before this emits an event.
    match diagnostics::level(variable.as_deref(), verbose) {
        Ok(level) => diagnostics::install(level),
        Err(message) => return fatal(&message),
    }
    match run_guest(game_dir, &settings, &role, version) {
        Ok(true) => Some(ExitCode::SUCCESS),
        Ok(false) => Some(ExitCode::FAILURE),
        Err(error) => fatal(&format_args!("{error:#}")),
    }
}

/// The guest role: plays the game in `game_dir` over the socket of the parent process, as
/// an executable of `version`. `Ok(false)` is a game that failed and has said why.
fn run_guest(
    game_dir: PathBuf,
    settings: &GuestSettings,
    role: &GuestRole,
    version: &str,
) -> Result<bool> {
    signals::die_with_parent();
    let inputs = guest_inputs(&game_dir, settings)?;
    refuse_other_interpreter(inputs.runtime)?;
    let runtime = inputs.runtime;
    let mut config = guest_config(&game_dir, settings, inputs, role.seed)?;
    let interpreter = start_interpreter(runtime)?;
    let attached = Attached::connect(role, version)?;
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
pub fn game_succeeded(result: Result<(), RunError>) -> bool {
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
        assert!(message.contains("of RPG Maker VX\""), "{message}");
        // Nor is that of RGSS3, which has a section of its own.
        let absent = dir.path().join("absent.rb");
        let error = builtin_scripts(Some(absent), Runtime::Rgss3).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("docs/running.md"), "{message}");
        assert!(message.contains("absent.rb"), "{message}");
        assert!(message.contains("of RPG Maker VX Ace\""), "{message}");
        assert!(!message.contains("planned"), "{message}");
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

        // RGSS3 is a runtime like the others here.
        for library in ["System\\RGSS300.dll", "System\\RGSS301.dll"] {
            let ace = ini(library, "Data\\Scripts.rvdata2");
            assert_eq!(runtime_of(&ace, None).unwrap(), Runtime::Rgss3);
        }
        assert_eq!(
            runtime_of(&xp, Some(Runtime::Rgss3)).unwrap(),
            Runtime::Rgss3
        );
    }

    /// The bytes of a file that holds the version record of `version` after other bytes.
    fn dll(version: [u16; 4]) -> Vec<u8> {
        let mut bytes = vec![0x4D, 0x5A];
        bytes.extend_from_slice(&[7; 300]);
        for word in [
            0xFEEF_04BD,
            0x0001_0000,
            u32::from(version[0]) << 16 | u32::from(version[1]),
            u32::from(version[2]) << 16 | u32::from(version[3]),
        ] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend_from_slice(&[9; 40]);
        bytes
    }

    /// The version of the DLL of a game directory with the file `file` of `contents`, read
    /// by the `Library` of its `Game.ini`.
    fn version_in_game(
        library: &str,
        file: Option<(&str, Vec<u8>)>,
    ) -> (Option<LibraryVersion>, String) {
        let game = tempfile::tempdir().unwrap();
        if let Some((name, contents)) = file {
            let path = game.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
        let ini = ini(library, "Data\\Scripts.rvdata2");
        let files = GameFiles::new(game.path(), []);
        library_version(&files, &ini.library)
    }

    /// Scenarios "The file named by the configuration" and "The name of the file does not
    /// count" of "Glyph rule by the library of a game of RGSS3".
    #[test]
    fn the_version_is_read_from_the_file_the_library_names() {
        let (version, line) = version_in_game(
            "System\\RGSS300.dll",
            Some(("system/rgss300.dll", dll([3, 0, 0, 1]))),
        );
        assert_eq!(version, Some(LibraryVersion([3, 0, 0, 1])));
        assert!(
            line.contains("3.0.0.1") && line.contains("rgss300.dll"),
            "{line}"
        );
        let (version, _) = version_in_game(
            "System\\RGSS300.dll",
            Some(("System/RGSS300.dll", dll([3, 0, 1, 1]))),
        );
        assert_eq!(version, Some(LibraryVersion([3, 0, 1, 1])));
        assert_eq!(
            font::mask_rules(Runtime::Rgss3, version),
            vkxp_text::MaskRules::Rgss3
        );
    }

    /// Scenario "No version": each is one line of the log that names the reason.
    #[test]
    fn a_game_without_a_version_says_why() {
        for (library, file, reason) in [
            ("", None, "names no Library"),
            ("System\\RGSS300.dll", None, "is not in the game directory"),
            (
                "System\\RGSS300.dll",
                Some(("System/RGSS300.dll", vec![0; 1000])),
                "has no version record",
            ),
        ] {
            let (version, line) = version_in_game(library, file);
            assert_eq!(version, None, "{library}");
            assert!(line.contains(reason) && !line.contains('\n'), "{line}");
        }
    }

    /// The title of the inputs of a game directory whose `Game.ini` is `ini`, and whether
    /// the file is unchanged afterwards.
    fn title_of_inputs(ini: &str) -> (String, bool) {
        let game = tempfile::tempdir().unwrap();
        let path = game.path().join("Game.ini");
        std::fs::write(&path, ini).unwrap();
        let builtin = game.path().join("rpg.rb");
        std::fs::write(&builtin, "module RPG;end\n").unwrap();
        let settings = GuestSettings {
            builtin_scripts: Some(builtin),
            ..GuestSettings::default()
        };
        let inputs = guest_inputs(game.path(), &settings).unwrap();
        let unchanged = std::fs::read_to_string(&path).unwrap() == ini;
        (inputs.ini.title, unchanged)
    }

    /// Requirement "Title of a game of RGSS3".
    #[test]
    fn a_game_of_rgss3_without_a_title_is_untitled() {
        let ace = "[Game]\r\nLibrary=System\\RGSS301.dll\r\nScripts=Data\\Scripts.rvdata2\r\n";
        assert_eq!(title_of_inputs(ace), ("Untitled".to_owned(), true));
        let empty = format!("{ace}Title=\r\n");
        assert_eq!(title_of_inputs(&empty), ("Untitled".to_owned(), true));
        let named = format!("{ace}Title=Ace\r\n");
        assert_eq!(title_of_inputs(&named).0, "Ace");
        // XP and VX keep an empty title.
        for (library, scripts) in [
            ("RGSS104E.dll", "Data\\Scripts.rxdata"),
            ("RGSS202E.dll", "Data\\Scripts.rvdata"),
        ] {
            let ini = format!("[Game]\r\nLibrary={library}\r\nScripts={scripts}\r\n");
            assert_eq!(title_of_inputs(&ini), (String::new(), true));
        }
    }

    #[test]
    fn an_existing_built_in_scripts_file_is_taken() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rpg.rb");
        std::fs::write(&file, "module RPG;end\n").unwrap();
        for runtime in Runtime::ALL {
            assert_eq!(builtin_scripts(Some(file.clone()), runtime).unwrap(), file);
        }
    }
}
