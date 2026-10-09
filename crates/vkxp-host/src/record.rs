//! Input recording and replay.
//!
//! A recording is a text file. Its first line is `vkxp-input 1 seed=<n>`, with the random
//! seed of the recorded run. Every other line stands for a run of frames with the same
//! buttons held: `<count>x`, followed by a blank and the `+`-joined names of the held buttons
//! when there are any. For example
//!
//! ```text
//! vkxp-input 1 seed=7
//! 60x
//! 2x C
//! 58x
//! 4x DOWN+SHIFT
//! ```
//!
//! holds nothing for 60 frames, C for two, nothing for 58 and DOWN and SHIFT for four. Button
//! names are those of the input script (see [`button_name`]).
//!
//! A session that has an [`InputSource`] asks it once per frame, in order, for the snapshot
//! to deliver, and sends that snapshot ahead of the frame's tick. [`Recorder`] passes the
//! live input through and writes it down; [`Replayer`] delivers what a [`Recording`] holds.

use std::fmt::Debug;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;
use vkxp_proto::{Button, InputSnapshot};

use crate::button_name;
use crate::script::button_named;

/// What the first line of a recording starts with: the format's name and version.
const MAGIC: &str = "vkxp-input 1 seed=";

/// Where a session takes the input snapshot of each frame from.
pub trait InputSource: Debug + Send {
    /// The snapshot to deliver for the next frame. `live` is what the keyboard, or the input
    /// script of a headless run, holds at that frame.
    fn next(&mut self, live: InputSnapshot) -> InputSnapshot;
}

/// Why a file could not be used as a recording.
#[derive(Debug, Error)]
pub enum RecordingError {
    #[error("cannot read the input recording {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot write the input recording {}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{} is not an input recording: {reason}", path.display())]
    Malformed { path: PathBuf, reason: String },
}

/// The input of a run: its seed and the snapshot of every frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recording {
    /// The random seed of the recorded run.
    pub seed: u64,
    /// Runs of frames with the same snapshot, in order: the number of frames and the
    /// snapshot.
    runs: Vec<(u64, InputSnapshot)>,
}

impl Recording {
    /// A recording of no frames of a run with `seed`.
    pub fn new(seed: u64) -> Recording {
        Recording {
            seed,
            runs: Vec::new(),
        }
    }

    /// Adds a frame at the end.
    pub fn push(&mut self, snapshot: InputSnapshot) {
        match self.runs.last_mut() {
            Some((count, last)) if *last == snapshot => *count += 1,
            _ => self.runs.push((1, snapshot)),
        }
    }

    /// The number of frames.
    pub fn frames(&self) -> u64 {
        self.runs.iter().map(|(count, _)| count).sum()
    }

    /// The snapshot of every frame, in order.
    pub fn snapshots(&self) -> impl Iterator<Item = InputSnapshot> + '_ {
        self.runs
            .iter()
            .flat_map(|(count, snapshot)| (0..*count).map(|_| *snapshot))
    }

    /// Reads the text of a recording; see the module documentation for the format. The error
    /// says what is wrong with it.
    pub fn parse(text: &str) -> Result<Recording, String> {
        let mut lines = text.lines();
        let first = lines.next().unwrap_or_default();
        let seed = first
            .strip_prefix(MAGIC)
            .and_then(|seed| seed.trim_end().parse().ok())
            .ok_or_else(|| format!("its first line is not `{MAGIC}<n>`"))?;
        let mut recording = Recording::new(seed);
        for (index, line) in lines.enumerate() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            let wrong = |what: &str| format!("line {}: {what}", index + 2);
            let (count, buttons) = line
                .split_once('x')
                .ok_or_else(|| wrong("no `x` after the number of frames"))?;
            let count: u64 = count
                .parse()
                .map_err(|_| wrong("it does not start with a number of frames"))?;
            let mut snapshot = InputSnapshot::EMPTY;
            if !buttons.trim().is_empty() {
                for name in buttons.split('+').map(str::trim) {
                    let button = button_named(name)
                        .ok_or_else(|| wrong(&format!("`{name}` is not the name of a button")))?;
                    snapshot = snapshot.with(button);
                }
            }
            if count > 0 {
                match recording.runs.last_mut() {
                    Some((last_count, last)) if *last == snapshot => *last_count += count,
                    _ => recording.runs.push((count, snapshot)),
                }
            }
        }
        Ok(recording)
    }

    /// Reads the recording in the file `path`. The error names the file.
    pub fn read(path: &Path) -> Result<Recording, RecordingError> {
        let bytes = std::fs::read(path).map_err(|source| RecordingError::Read {
            path: path.to_owned(),
            source,
        })?;
        let malformed = |reason: String| RecordingError::Malformed {
            path: path.to_owned(),
            reason,
        };
        let text = str::from_utf8(&bytes).map_err(|_| malformed("it is not text".to_owned()))?;
        Recording::parse(text).map_err(malformed)
    }
}

/// The first line of a recording of a run with `seed`.
fn header(seed: u64) -> String {
    format!("{MAGIC}{seed}\n")
}

/// The line of a run of `count` frames with `snapshot` held.
fn run_line(count: u64, snapshot: InputSnapshot) -> String {
    let names: Vec<&str> = Button::ALL
        .into_iter()
        .filter(|button| snapshot.is_down(*button))
        .map(button_name)
        .collect();
    if names.is_empty() {
        format!("{count}x\n")
    } else {
        format!("{count}x {}\n", names.join("+"))
    }
}

impl std::fmt::Display for Recording {
    /// The text of the recording, as [`Recording::parse`] reads it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&header(self.seed))?;
        self.runs
            .iter()
            .try_for_each(|(count, snapshot)| f.write_str(&run_line(*count, *snapshot)))
    }
}

/// An input source that delivers the live input and writes it to a file as a recording.
///
/// A run of frames is written when the frame after it arrives, and the last one when the
/// recorder is dropped. A failure to write is reported on the error stream once, and the
/// session goes on without recording.
#[derive(Debug)]
pub struct Recorder {
    path: PathBuf,
    /// The file, until writing to it has failed.
    file: Option<File>,
    /// The run of frames that is not written yet.
    run: Option<(u64, InputSnapshot)>,
}

impl Recorder {
    /// Creates the file `path`, replacing what is there, and writes the first line of the
    /// recording of a run with `seed`.
    pub fn create(path: &Path, seed: u64) -> Result<Recorder, RecordingError> {
        File::create(path)
            .and_then(|mut file| file.write_all(header(seed).as_bytes()).map(|()| file))
            .map(|file| Recorder {
                path: path.to_owned(),
                file: Some(file),
                run: None,
            })
            .map_err(|source| RecordingError::Write {
                path: path.to_owned(),
                source,
            })
    }

    /// Writes the run of frames collected so far.
    fn write_run(&mut self) {
        let (Some((count, snapshot)), Some(file)) = (self.run.take(), &mut self.file) else {
            return;
        };
        if let Err(source) = file.write_all(run_line(count, snapshot).as_bytes()) {
            let error = RecordingError::Write {
                path: self.path.clone(),
                source,
            };
            eprintln!("vkxp: {error}");
            self.file = None;
        }
    }
}

impl InputSource for Recorder {
    fn next(&mut self, live: InputSnapshot) -> InputSnapshot {
        match &mut self.run {
            Some((count, snapshot)) if *snapshot == live => *count += 1,
            _ => {
                self.write_run();
                self.run = Some((1, live));
            }
        }
        live
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.write_run();
    }
}

/// An input source that delivers the frames of a recording and, after its last frame, the
/// live input: the keyboard in a window, and nothing in a headless run whose input script is
/// empty.
#[derive(Debug, Clone)]
pub struct Replayer {
    /// The runs that are left, the next one last.
    runs: Vec<(u64, InputSnapshot)>,
}

impl Replayer {
    pub fn new(recording: Recording) -> Replayer {
        let mut runs = recording.runs;
        runs.reverse();
        Replayer { runs }
    }
}

impl InputSource for Replayer {
    fn next(&mut self, live: InputSnapshot) -> InputSnapshot {
        let Some((count, snapshot)) = self.runs.last_mut() else {
            return live;
        };
        let snapshot = *snapshot;
        *count -= 1;
        if *count == 0 {
            self.runs.pop();
        }
        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(buttons: &[Button]) -> InputSnapshot {
        buttons
            .iter()
            .fold(InputSnapshot::EMPTY, |snapshot, button| {
                snapshot.with(*button)
            })
    }

    /// Three frames of nothing, two of C, one of DOWN and SHIFT.
    fn example() -> Vec<InputSnapshot> {
        let c = held(&[Button::C]);
        let down_shift = held(&[Button::Shift, Button::Down]);
        let empty = InputSnapshot::EMPTY;
        vec![empty, empty, empty, c, c, down_shift]
    }

    const EXAMPLE_TEXT: &str = "vkxp-input 1 seed=7\n3x\n2x C\n1x DOWN+SHIFT\n";

    #[test]
    fn a_recorder_writes_the_seed_and_run_length_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input.txt");
        let mut recorder = Recorder::create(&path, 7).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "vkxp-input 1 seed=7\n"
        );
        for snapshot in example() {
            assert_eq!(recorder.next(snapshot), snapshot);
        }
        // The runs that have ended are in the file while the recorder lives.
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "vkxp-input 1 seed=7\n3x\n2x C\n"
        );
        drop(recorder);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), EXAMPLE_TEXT);
    }

    #[test]
    fn what_a_recorder_wrote_reads_back_frame_by_frame() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input.txt");
        let mut recorder = Recorder::create(&path, u64::MAX).unwrap();
        let frames: Vec<InputSnapshot> = Button::ALL
            .into_iter()
            .map(|button| held(&[button]))
            .chain([held(&Button::ALL), InputSnapshot::EMPTY])
            .collect();
        for snapshot in &frames {
            recorder.next(*snapshot);
        }
        drop(recorder);
        let recording = Recording::read(&path).unwrap();
        assert_eq!(recording.seed, u64::MAX);
        assert_eq!(recording.frames(), frames.len() as u64);
        assert_eq!(recording.snapshots().collect::<Vec<_>>(), frames);
    }

    #[test]
    fn the_text_of_a_recording_parses_back() {
        let recording = Recording::parse(EXAMPLE_TEXT).unwrap();
        assert_eq!(recording.seed, 7);
        assert_eq!(recording.snapshots().collect::<Vec<_>>(), example());
        assert_eq!(recording.to_string(), EXAMPLE_TEXT);

        let mut pushed = Recording::new(7);
        example().into_iter().for_each(|frame| pushed.push(frame));
        assert_eq!(pushed, recording);
    }

    #[test]
    fn line_ends_blank_lines_and_split_runs_are_accepted() {
        let text = "vkxp-input 1 seed=0\r\n2x\r\n1x\n\n0x A\n1x shift + down\n";
        let recording = Recording::parse(text).unwrap();
        assert_eq!(recording.seed, 0);
        assert_eq!(
            recording.to_string(),
            "vkxp-input 1 seed=0\n3x\n1x DOWN+SHIFT\n"
        );
    }

    #[test]
    fn text_that_is_no_recording_is_refused() {
        for (text, reason) in [
            ("", "first line"),
            ("2:RIGHT, 6:", "first line"),
            ("vkxp-input 2 seed=1\n", "first line"),
            ("vkxp-input 1 seed=one\n", "first line"),
            ("vkxp-input 1 seed=1\n3\n", "line 2: no `x`"),
            ("vkxp-input 1 seed=1\n1x\nmanyx C\n", "line 3"),
            ("vkxp-input 1 seed=1\n1x JUMP\n", "`JUMP`"),
            ("vkxp-input 1 seed=1\n1x A+\n", "line 2"),
        ] {
            let error = Recording::parse(text).unwrap_err();
            assert!(error.contains(reason), "{text:?}: {error}");
        }
    }

    /// Scenario "Not a recording", as far as the file goes.
    #[test]
    fn a_file_of_other_content_is_refused_with_its_name() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, "2:RIGHT, 6:\n").unwrap();
        let error = Recording::read(&path).unwrap_err();
        assert!(
            matches!(error, RecordingError::Malformed { .. }),
            "{error:?}"
        );
        assert!(error.to_string().contains("notes.txt"), "{error}");

        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        let error = Recording::read(&path).unwrap_err();
        assert!(error.to_string().contains("notes.txt"), "{error}");

        let absent = dir.path().join("absent.txt");
        let error = Recording::read(&absent).unwrap_err();
        assert!(matches!(error, RecordingError::Read { .. }), "{error:?}");
        assert!(error.to_string().contains("absent.txt"), "{error}");
    }

    #[test]
    fn a_recording_that_cannot_be_created_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nowhere").join("input.txt");
        let error = Recorder::create(&path, 0).unwrap_err();
        assert!(matches!(error, RecordingError::Write { .. }), "{error:?}");
        assert!(error.to_string().contains("input.txt"), "{error}");
    }

    #[test]
    fn a_replayer_delivers_the_recording_and_then_the_live_input() {
        let mut replayer = Replayer::new(Recording::parse(EXAMPLE_TEXT).unwrap());
        let live = held(&[Button::Left]);
        let delivered: Vec<InputSnapshot> = (0..6).map(|_| replayer.next(live)).collect();
        assert_eq!(delivered, example());
        assert_eq!(replayer.next(live), live);
        assert_eq!(replayer.next(InputSnapshot::EMPTY), InputSnapshot::EMPTY);
    }
}
