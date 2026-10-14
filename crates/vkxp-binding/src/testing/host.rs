//! A scripted host for the tests of what crosses the link, built on `vkxp_proto::link` alone.
//!
//! [`scripted_host`] starts a thread that holds the host end of a new link. It answers every
//! frame with a tick and every modal message with its dismissal, at once unless its
//! [`Script`] says after how many frames, records what it receives, and sends what its
//! [`Script`] tells it to. [`play`] evaluates a Ruby snippet on the
//! harness's guest against such a host, and [`run_game`] plays a [`Game`], a game directory
//! made for one test, on it.
//!
//! The first use of [`play`] adds these files to the generated game directory and evaluates
//! the prelude of the binding, with the default [`RunConfig`] unless [`configure`] came
//! first:
//!
//! | File | Contents |
//! |---|---|
//! | `game/Extra.rb` | Ruby source that defines `VKXP_EXTRA` and counts its evaluations in `$vkxp_extra_loads` |
//! | `game/data/thing.rxdata` | the `Marshal` dump of `[1, "two"]` |
//! | `game/Graphics/Transitions/004-Blind04.png` | 2 by 2 pixels, [`DOT_PIXELS`](super::DOT_PIXELS) |
//! | `rtp/Audio/SE/001-System01.ogg` | [`SOUND_BYTES`], which is not sound |

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Mutex, Once, PoisonError};
use std::thread::{self, JoinHandle};

use vkxp_proto::link::{GuestEnd, link};
use vkxp_proto::{FramePacket, GuestToHost, HostToGuest};
use vkxp_ruby_sys::{Ruby, RubyError};
use vkxp_vfs::{GameFiles, GameIni, Runtime};

use crate::{Guest, RunConfig, RunError, runtime};

/// What a scripted host sends besides its answers.
#[derive(Debug, Clone, Default)]
pub struct Script {
    at_start: Vec<HostToGuest>,
    after_frame: Vec<(usize, HostToGuest)>,
    dismiss_after: Option<usize>,
}

impl Script {
    /// A host that only answers.
    pub fn new() -> Script {
        Script::default()
    }

    /// Sends `message` before the guest does anything.
    #[must_use]
    pub fn at_start(mut self, message: HostToGuest) -> Script {
        self.at_start.push(message);
        self
    }

    /// Sends `message` when the frame with this number has arrived, counted from 1, before
    /// the tick that answers it.
    #[must_use]
    pub fn after_frame(mut self, frame: usize, message: HostToGuest) -> Script {
        self.after_frame.push((frame, message));
        self
    }

    /// Dismisses a modal message when this many frames have arrived after it, before the
    /// tick of the last of them, instead of at once.
    #[must_use]
    pub fn dismiss_after(mut self, frames: usize) -> Script {
        self.dismiss_after = Some(frames);
        self
    }
}

/// What a scripted host saw.
#[derive(Debug, Clone, Default)]
pub struct Record {
    /// Every message of the guest, in the order received.
    pub received: Vec<GuestToHost>,
    /// The number of ticks sent.
    pub ticks: usize,
}

impl Record {
    /// The frame packets received, in order.
    pub fn frames(&self) -> Vec<&FramePacket> {
        self.received
            .iter()
            .filter_map(|message| match message {
                GuestToHost::Frame(packet) => Some(packet),
                _ => None,
            })
            .collect()
    }

    /// The messages received that are not frames, in order.
    pub fn others(&self) -> Vec<&GuestToHost> {
        self.received
            .iter()
            .filter(|message| !matches!(message, GuestToHost::Frame(_)))
            .collect()
    }
}

/// A running scripted host.
#[derive(Debug)]
pub struct ScriptedHost {
    thread: JoinHandle<Record>,
}

impl ScriptedHost {
    /// Waits until the guest end of the link is dropped and returns what the host saw.
    pub fn finish(self) -> Record {
        self.thread
            .join()
            .expect("the scripted host does not panic")
    }
}

/// Starts a scripted host and returns the guest end of its link.
pub fn scripted_host(script: Script) -> (GuestEnd, ScriptedHost) {
    let (wake, woken) = mpsc::channel();
    let (guest, host) = link(move || {
        let _ = wake.send(());
    });
    for message in &script.at_start {
        host.send(message.clone()).expect("the guest end is alive");
    }
    let thread = thread::spawn(move || {
        let mut record = Record::default();
        let mut frames = 0;
        // The frames still to come before the modal message being shown is dismissed.
        let mut modal_frames = None;
        // The wake hook is dropped with the guest end, which ends the outer loop.
        while woken.recv().is_ok() {
            while let Ok(Some(message)) = host.try_recv() {
                let answer = match &message {
                    GuestToHost::Frame(_) => {
                        frames += 1;
                        for (frame, extra) in &script.after_frame {
                            if *frame == frames {
                                let _ = host.send(extra.clone());
                            }
                        }
                        if let Some(left) = &mut modal_frames {
                            *left -= 1;
                            if *left == 0 {
                                modal_frames = None;
                                let _ = host.send(HostToGuest::ModalDismissed);
                            }
                        }
                        Some(HostToGuest::Tick)
                    }
                    GuestToHost::Modal { .. } => match script.dismiss_after {
                        Some(frames) if frames > 0 => {
                            modal_frames = Some(frames);
                            None
                        }
                        _ => Some(HostToGuest::ModalDismissed),
                    },
                    _ => None,
                };
                record.received.push(message);
                if let Some(answer) = answer {
                    let is_tick = answer == HostToGuest::Tick;
                    if host.send(answer).is_ok() && is_tick {
                        record.ticks += 1;
                    }
                }
            }
        }
        record
    });
    (guest, ScriptedHost { thread })
}

/// The Ruby source of `Extra.rb` in the game directory.
const EXTRA_RB: &str = "VKXP_EXTRA = :extra unless defined?(VKXP_EXTRA)\n\
                        $vkxp_extra_loads = ($vkxp_extra_loads || 0) + 1\n";

/// The contents of `Audio/SE/001-System01.ogg` in the run-time package directory.
pub const SOUND_BYTES: &[u8] = b"not really Ogg Vorbis";

/// The settings [`configure`] asked for, until the preparation takes them.
static CONFIGURED: Mutex<Option<RunConfig>> = Mutex::new(None);

/// Makes the one preparation of this process use `run` in place of the default settings. A
/// test binary calls this before anything else of this module, because what the prelude
/// defines cannot be taken back: the process functions stay off once they were turned off.
pub fn configure(run: RunConfig) {
    *CONFIGURED.lock().unwrap_or_else(PoisonError::into_inner) = Some(run);
    prepare();
}

/// Adds the files of this module to the game directory and evaluates the prelude, once.
fn prepare() {
    static PREPARED: Once = Once::new();
    PREPARED.call_once(|| {
        let game = super::game_dir();
        super::write(&game.join("Extra.rb"), EXTRA_RB.as_bytes());
        super::write(
            &game.join("Graphics/Transitions/004-Blind04.png"),
            &super::png(2, &super::DOT_PIXELS),
        );
        super::write(
            &game.with_file_name("rtp").join("Audio/SE/001-System01.ogg"),
            SOUND_BYTES,
        );
        // Written by the interpreter below; the directory has to exist.
        let thing = game.join("data/thing.rxdata");
        super::write(&thing, b"");
        let dump_thing = format!(
            "File.open({}, 'wb') {{ |file| Marshal.dump([1, 'two'], file) }}",
            ruby_literal(&thing.to_string_lossy())
        );
        let run = CONFIGURED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        super::run(move |ruby| {
            Guest::with(|guest| {
                guest.files.refresh();
                if let Some(run) = run {
                    guest.runtime.run = run;
                }
            });
            runtime::prepare(ruby).expect("the prelude evaluates");
            ruby.eval_protected(&dump_thing, "testing", 1)
                .expect("the data file is written");
        });
    });
}

/// A Ruby string literal of `text`.
pub fn ruby_literal(text: &str) -> String {
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// A snippet whose value is the value of `source` together with what `source` wrote to the
/// error stream, as an array of the two.
pub fn with_error_stream(source: &str) -> String {
    format!(
        "vkxp_capture = Object.new
         def vkxp_capture.write(text)
           (@text ||= '') << text.to_s
           text.to_s.size
         end
         def vkxp_capture.text
           @text || ''
         end
         vkxp_stderr = $stderr
         begin
           $stderr = vkxp_capture
           vkxp_value = begin
             {source}
           end
         ensure
           $stderr = vkxp_stderr
         end
         [vkxp_value, vkxp_capture.text]"
    )
}

/// `inspect` of the value of `source`, evaluated as `test`, line 1.
fn inspect(ruby: &Ruby, source: &str) -> Result<String, RubyError> {
    let value = ruby.eval_protected(source, "test", 1)?;
    // SAFETY: `value` was just returned by the interpreter and is held in a local; so is the
    // exception of a failed `inspect`.
    unsafe { ruby.inspect(value).map_err(|raised| ruby.describe(raised)) }
}

/// Evaluates Ruby source on the harness's guest, connected to a new scripted host, with the
/// prelude in place. Returns what [`eval`](super::eval) returns and what the host saw.
pub fn play(source: &str, script: Script) -> (Result<String, RubyError>, Record) {
    prepare();
    let (link, host) = scripted_host(script);
    let source = source.to_owned();
    let outcome = super::run(move |ruby| {
        Guest::with(|guest| guest.connect(link));
        let outcome = inspect(ruby, &source);
        Guest::with(Guest::disconnect);
        outcome
    });
    (outcome, host.finish())
}

/// A stand-in for the built-in scripts of the original runtime, written for these tests: a
/// minimal `RPG` module.
pub const RPG_STAND_IN: &str = "\
module RPG
  module Cache
  end

  class Sprite
  end

  class Weather
  end
end
";

/// A game directory made for one test, with a `Game.ini`, a script archive and, next to it,
/// a built-in scripts file holding [`RPG_STAND_IN`].
#[derive(Debug, Clone)]
pub struct Game {
    dir: PathBuf,
}

impl Game {
    /// Creates the game `name` afresh below the harness's directory. Its window title is
    /// `title` and its archive, `Data/Scripts.rxdata`, holds `scripts` as pairs of name and
    /// source, assembled with `Marshal` and `Zlib` of the interpreter.
    pub fn new(name: &str, title: &str, scripts: &[(&str, &str)]) -> Game {
        Game::of(Runtime::Rgss1, name, title, scripts)
    }

    /// Creates a game as [`Game::new`] does, as the editor of `runtime` lays it out: the
    /// library and the name of the archive in `Game.ini` are those of that runtime.
    pub fn of(runtime: Runtime, name: &str, title: &str, scripts: &[(&str, &str)]) -> Game {
        let (library, archive) = match runtime {
            Runtime::Rgss1 => ("RGSS104E.dll", "Scripts.rxdata"),
            Runtime::Rgss2 => ("RGSS202E.dll", "Scripts.rvdata"),
            Runtime::Rgss3 => ("System\\RGSS301.dll", "Scripts.rvdata2"),
        };
        let dir = super::game_dir().with_file_name("games").join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
        }
        let game = Game { dir };
        let ini =
            format!("[Game]\r\nLibrary={library}\r\nScripts=Data\\{archive}\r\nTitle={title}\r\n");
        super::write(&game.dir.join("Game.ini"), ini.as_bytes());
        super::write(&game.builtin_scripts(), RPG_STAND_IN.as_bytes());
        let archive = game.dir.join("Data").join(archive);
        super::write(&archive, b"");
        let entries: Vec<String> = scripts
            .iter()
            .enumerate()
            .map(|(id, (name, source))| {
                format!(
                    "[{id}, {}, Zlib::Deflate.deflate({})]",
                    ruby_literal(name),
                    ruby_literal(source)
                )
            })
            .collect();
        let assemble = format!(
            "File.open({}, 'wb') {{ |file| Marshal.dump([{}], file) }}",
            ruby_literal(&archive.to_string_lossy()),
            entries.join(", ")
        );
        super::run(move |ruby| ruby.eval_protected(&assemble, "testing", 1).map(|_| ()))
            .expect("the script archive is written");
        game
    }

    /// The game directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The built-in scripts file of this game, outside its directory.
    pub fn builtin_scripts(&self) -> PathBuf {
        self.dir.with_extension("rpg.rb")
    }

    /// What running this game needs, as a caller of `run_game` puts it together: the
    /// configuration file read from the directory, the runtime it tells, the built-in
    /// scripts file, and every switch off.
    pub fn run_config(&self) -> RunConfig {
        let ini = GameIni::read(&self.dir, None).expect("the game has a Game.ini");
        RunConfig {
            runtime: Runtime::detect(&ini).expect("the game tells its runtime"),
            ini,
            builtin_scripts: self.builtin_scripts(),
            ..RunConfig::default()
        }
    }
}

/// How a game went.
#[derive(Debug)]
pub struct Played {
    /// What running it returned.
    pub outcome: Result<(), RunError>,
    /// What the host saw.
    pub record: Record,
    /// `inspect` of the snippet evaluated after the run.
    pub after: Result<String, RubyError>,
    /// The number of live bitmaps in the guest's arena after the run, counted before
    /// another test can use the interpreter.
    pub bitmaps: usize,
}

/// Plays `game` with the settings `run` against a scripted host, then evaluates the snippet
/// `after`, which is how a test reads what the scripts left in global variables.
///
/// The process has one interpreter and the harness's guest is installed on it, so this is
/// `run_game` from its second step on, on that guest: what an earlier game left is disposed,
/// and the guest gets the files of `game` (without run-time packages), an empty scene and
/// `run`, and the current error is cleared. Classes, constants and global variables of
/// earlier games stay, and so do the functions of `Graphics` and the frame, which are those
/// of RGSS1 whatever the runtime of `run` is: what a game of another runtime starts with is
/// tested through `run_game` of the crate, in a process of its own.
pub fn run_game(game: &Game, run: RunConfig, script: Script, after: &str) -> Played {
    let (link, host) = scripted_host(script);
    let files = GameFiles::new(game.dir(), []);
    let after = after.to_owned();
    let (outcome, after, bitmaps) = super::run(move |ruby| {
        ruby.eval_protected("Graphics._reset", "testing", 1)
            .expect("what the previous game left is disposed");
        // SAFETY: `nil` is a special constant.
        unsafe { ruby.set_current_error(vkxp_ruby_sys::ffi::Qnil) };
        Guest::with(|guest| {
            guest.files = files;
            // Not a new scene: the element objects of earlier games are collected while
            // this one plays, and their handles must not be those of its elements.
            guest.scene.clear();
            guest.runtime.run = run;
        });
        let outcome = runtime::play(ruby, link);
        let after = inspect(ruby, &after);
        (outcome, after, Guest::with(|guest| guest.bitmaps().len()))
    });
    Played {
        outcome,
        record: host.finish(),
        after,
        bitmaps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Await;

    #[test]
    fn three_frames_through_the_pump_are_three_frames_and_three_ticks() {
        let (link, host) = scripted_host(Script::new());
        super::super::run(move |_ruby| {
            Guest::with(|guest| {
                guest.connect(link);
                for _ in 0..3 {
                    let packet = guest.scene.extract(&mut guest.bitmaps);
                    guest.send(GuestToHost::Frame(packet));
                    guest.pump(Await::Tick);
                }
                guest.disconnect();
            });
        });
        let record = host.finish();
        assert_eq!(record.frames().len(), 3);
        assert_eq!(record.received.len(), 3);
        assert_eq!(record.ticks, 3);
    }

    /// The objects of an earlier game are collected while a later one plays. Their handles
    /// name nothing in its scene: none of its elements is disposed with them.
    #[test]
    fn the_collected_objects_of_an_earlier_game_dispose_nothing_of_a_later_one() {
        let earlier = "sprites = (0...200).map { Sprite.new }
                       Graphics.update";
        let later = "sprites = (0...200).map { Sprite.new }
                     GC.start
                     Graphics.update
                     $vkxp_disposed = sprites.select { |sprite| sprite.disposed? }.size";
        for (name, main) in [("host_earlier", earlier), ("host_later", later)] {
            let game = Game::new(name, "Collected", &[("Main", main)]);
            let played = run_game(&game, game.run_config(), Script::new(), "$vkxp_disposed");
            assert!(played.outcome.is_ok(), "{:?}", played.outcome);
            if name == "host_later" {
                assert_eq!(played.after.as_deref(), Ok("0"));
            }
        }
    }

    #[test]
    fn a_guest_whose_host_has_gone_stops_waiting_and_quits() {
        let (link, host) = scripted_host(Script::new());
        let quit = super::super::run(move |ruby| {
            Guest::with(|guest| {
                guest.connect(link);
                guest.disconnect();
                guest.pump(Await::Tick);
            });
            runtime::raise_pending(ruby).is_err()
        });
        assert!(quit);
        assert!(host.finish().received.is_empty());
    }
}
