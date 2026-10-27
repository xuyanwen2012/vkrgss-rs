//! MIDI: Standard MIDI files rendered by `rustysynth` with a SoundFont.

use std::cell::Cell;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustysynth::{
    MidiFile, MidiFileLoopType, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings,
};

use crate::decode::{DecodeError, Source};

/// Where a SoundFont is looked for when none is named, in this order.
pub const SYSTEM_SOUNDFONTS: [&str; 3] = [
    "/usr/share/soundfonts/default.sf2",
    "/usr/share/soundfonts/FluidR3_GM.sf2",
    "/usr/share/sounds/sf2/FluidR3_GM.sf2",
];

/// How many frames the synthesiser renders at a time.
const BLOCK: usize = 1024;

/// The longest track whose loop is computed, in seconds: a start position in a longer one is
/// rendered up to, however far it is.
const LONGEST_LOOPED: f64 = 4.0 * 3600.0;

thread_local! {
    /// The frames the synthesisers of this thread have rendered.
    static SYNTHESISED: Cell<u64> = const { Cell::new(0) };
}

/// How many frames the MIDI synthesisers have rendered on the calling thread since it
/// started: what the tests measure the cost of a start position by.
#[doc(hidden)]
pub fn frames_synthesised_on_this_thread() -> u64 {
    SYNTHESISED.get()
}

/// Whether `data` is a Standard MIDI file.
pub(crate) fn is_midi(data: &[u8]) -> bool {
    data.starts_with(b"MThd")
}

enum State {
    /// Nothing has asked for the SoundFont yet.
    Unloaded,
    Loaded(Arc<SoundFont>),
    Failed(DecodeError),
}

/// The SoundFont for MIDI: the first file that exists among its locations, loaded when a
/// MIDI file is first opened and kept from then on.
pub struct SoundFonts {
    locations: Vec<PathBuf>,
    state: State,
}

impl SoundFonts {
    /// The SoundFont is the first of `locations` that exists.
    pub fn new(locations: Vec<PathBuf>) -> SoundFonts {
        SoundFonts {
            locations,
            state: State::Unloaded,
        }
    }

    /// The SoundFont is `named` or, without a name, the first of [`SYSTEM_SOUNDFONTS`] that
    /// exists.
    pub fn configured(named: Option<PathBuf>) -> SoundFonts {
        SoundFonts::new(match named {
            Some(path) => vec![path],
            None => SYSTEM_SOUNDFONTS.iter().map(PathBuf::from).collect(),
        })
    }

    pub(crate) fn get(&mut self) -> Result<Arc<SoundFont>, DecodeError> {
        if let State::Unloaded = self.state {
            self.state = match self.locations.iter().find(|path| path.is_file()) {
                Some(path) => match load(path) {
                    Ok(soundfont) => State::Loaded(Arc::new(soundfont)),
                    Err(reason) => State::Failed(DecodeError::SoundFont {
                        path: path.clone(),
                        reason,
                    }),
                },
                None => State::Failed(DecodeError::NoSoundFont {
                    searched: self.locations.clone(),
                }),
            };
        }
        match &self.state {
            State::Loaded(soundfont) => Ok(soundfont.clone()),
            State::Failed(error) => Err(error.clone()),
            State::Unloaded => unreachable!("the SoundFont state was settled above"),
        }
    }
}

fn load(path: &Path) -> Result<SoundFont, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    SoundFont::new(&mut BufReader::new(file)).map_err(|error| error.to_string())
}

/// Where the sequencer of `rustysynth` jumps back in a looping track, in frames since the
/// start: first at `end`, then every `period` frames.
#[derive(Clone, Copy, Debug)]
struct Loop {
    end: u64,
    period: u64,
}

impl Loop {
    /// Where the sequencer jumps back when it plays `data`, a MIDI file that `rustysynth`
    /// has read, at `sample_rate` in blocks of `block` frames. `None` when the file holds no
    /// event.
    ///
    /// The sequencer handles the events that are due at the start of each block, and its
    /// clock advances by a block at a time. At the end of the track it sets its clock back to
    /// the time of the last loop marker, or of the first event without one, and from there it
    /// steps through the same times in every pass. The times are computed here as the
    /// library computes them, so that the blocks are the same.
    fn of(data: &[u8], sample_rate: u32, block: usize) -> Option<Loop> {
        let (start, length) = loop_times(data)?;
        if !(start.is_finite() && length.is_finite() && length <= LONGEST_LOOPED) {
            return None;
        }
        let step = block as f64 / f64::from(sample_rate);
        let blocks_until = |from: f64| {
            let (mut time, mut blocks) = (from, 0u64);
            loop {
                time += step;
                blocks += 1;
                if time >= length {
                    return blocks;
                }
            }
        };
        // The first pass starts with a block at time 0, which is not a step.
        let end = match length <= 0.0 {
            true => 0,
            false => blocks_until(0.0),
        };
        Some(Loop {
            end: end * block as u64,
            period: blocks_until(start) * block as u64,
        })
    }

    /// The position that the source is in the same state at, at most one pass through the
    /// track and one through its loop from the start.
    fn reduce(&self, position: u64) -> u64 {
        match position.checked_sub(self.end) {
            Some(after) if self.period > 0 => self.end + after % self.period,
            _ => position,
        }
    }
}

/// What the merge of the tracks of a MIDI file needs of an event.
enum Event {
    Tempo(i32),
    LoopMarker,
    Other,
}

/// The time of the loop start and the length of the track, in seconds, as `rustysynth`
/// computes them: the time of the last controller 111, or of the first event without one,
/// and the time of the last event. `None` for data that the library does not read, or
/// without an event.
fn loop_times(data: &[u8]) -> Option<(f64, f64)> {
    let mut at = Bytes { data, at: 0 };
    if at.take(4)? != b"MThd" || at.u32()? != 6 {
        return None;
    }
    let format = at.u16()? as i16;
    if !(format == 0 || format == 1) {
        return None;
    }
    let tracks = at.u16()? as i16;
    let resolution = i32::from(at.u16()? as i16);
    let tracks: Vec<Vec<(i32, Event)>> = (0..tracks)
        .map(|_| track_events(&mut at))
        .collect::<Option<_>>()?;

    // The merge of the library: the earliest event first, of the first track on a tie; a
    // tempo change counts from its own event on and is no event of the merged list.
    let mut next = vec![0; tracks.len()];
    let (mut tick, mut time, mut tempo) = (0i32, 0.0f64, 120.0f64);
    let (mut first, mut marker, mut last) = (None, None, None);
    loop {
        let earliest = tracks
            .iter()
            .zip(&next)
            .enumerate()
            .filter_map(|(index, (events, next))| Some((events.get(*next)?.0, index)))
            .min_by_key(|(tick, index)| (*tick, *index));
        let Some((event_tick, index)) = earliest else {
            break;
        };
        let delta = event_tick - tick;
        tick += delta;
        time += 60.0 / (f64::from(resolution) * tempo) * f64::from(delta);
        match tracks[index][next[index]].1 {
            Event::Tempo(microseconds) => tempo = 60_000_000.0 / f64::from(microseconds),
            Event::LoopMarker => {
                marker = Some(time);
                first.get_or_insert(time);
                last = Some(time);
            }
            Event::Other => {
                first.get_or_insert(time);
                last = Some(time);
            }
        }
        next[index] += 1;
    }
    Some((marker.or(first)?, last?))
}

/// The events of the track at `at`, read as `rustysynth` reads them, with their ticks.
fn track_events(at: &mut Bytes) -> Option<Vec<(i32, Event)>> {
    if at.take(4)? != b"MTrk" {
        return None;
    }
    let size = at.u32()? as i32 as usize;
    let start = at.at;
    let mut events = Vec::new();
    let (mut tick, mut status) = (0i32, 0u8);
    loop {
        tick = tick.checked_add(at.variable()?)?;
        let first = at.byte()?;
        if first & 0x80 == 0 {
            // Running status: `first` is the first data byte.
            let event = match status & 0xf0 {
                0xc0 | 0xd0 => Event::Other,
                command => marker_or_other(command, first, at.byte()?),
            };
            events.push((tick, event));
            continue;
        }
        match first {
            0xf0 | 0xf7 => at.skip_variable()?,
            0xff => match at.byte()? {
                0x2f => {
                    at.byte()?;
                    events.push((tick, Event::Other));
                    // Bytes behind the end of the track are skipped.
                    at.at = at.at.max(start.saturating_add(size));
                    return Some(events);
                }
                0x51 => {
                    if at.variable()? != 3 {
                        return None;
                    }
                    let bytes = at.take(3)?;
                    let tempo = i32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]);
                    events.push((tick, Event::Tempo(tempo)));
                }
                _ => at.skip_variable()?,
            },
            _ => {
                let event = match first & 0xf0 {
                    0xc0 | 0xd0 => {
                        at.byte()?;
                        Event::Other
                    }
                    command => marker_or_other(command, at.byte()?, at.byte()?),
                };
                events.push((tick, event));
            }
        }
        status = first;
    }
}

/// A message of two data bytes: controller 111 is the loop marker of RPG Maker.
fn marker_or_other(command: u8, data1: u8, _data2: u8) -> Event {
    match (command, data1) {
        (0xb0, 111) => Event::LoopMarker,
        _ => Event::Other,
    }
}

/// A cursor in the bytes of a MIDI file.
struct Bytes<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Bytes<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let bytes = self.data.get(self.at..self.at.checked_add(count)?)?;
        self.at += count;
        Some(bytes)
    }

    fn byte(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        let bytes = self.take(2)?;
        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Option<u32> {
        let bytes = self.take(4)?;
        Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// A number of variable length, of at most four bytes.
    fn variable(&mut self) -> Option<i32> {
        let mut value = 0i32;
        for _ in 0..4 {
            let byte = self.byte()?;
            value = value << 7 | i32::from(byte & 0x7f);
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    fn skip_variable(&mut self) -> Option<()> {
        let size = self.variable()?;
        self.take(usize::try_from(size).ok()?)?;
        Some(())
    }
}

/// The sample rates at which the synthesiser of `rustysynth` runs.
const SYNTHESISER_RATES: std::ops::RangeInclusive<u32> = 16_000..=192_000;

/// A MIDI file played by a synthesiser that runs at the requested sample rate, or at the
/// nearest rate it can run at, its frames then resampled to the requested rate by linear
/// interpolation. One that loops never ends: at the end of the track it continues at the
/// track's controller 111, the loop marker of RPG Maker, or at the beginning of a track
/// without one.
pub(crate) struct MidiSource {
    file: Arc<MidiFile>,
    sequencer: MidiFileSequencer,
    looping: bool,
    /// Where a looping track jumps back, in frames of the synthesiser.
    jumps: Option<Loop>,
    /// Frames of the synthesiser per frame given.
    step: f64,
    /// Frames of the synthesiser, interleaved, from the one before the next frame given on.
    frames: Vec<f32>,
    /// Where the next frame given is in `frames`, in frames.
    at: f64,
    /// How many frames of the synthesiser lay before `frames`, since the start of the track.
    before: u64,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl MidiSource {
    pub(crate) fn new(
        data: &[u8],
        sample_rate: u32,
        soundfont: Arc<SoundFont>,
        looping: bool,
    ) -> Result<MidiSource, DecodeError> {
        let file = MidiFile::new_with_loop_type(&mut &data[..], MidiFileLoopType::RpgMaker)
            .map_err(|error| DecodeError::Unsupported(format!("MIDI: {error}")))?;
        let file = Arc::new(file);
        let rate = sample_rate.clamp(*SYNTHESISER_RATES.start(), *SYNTHESISER_RATES.end());
        let settings = SynthesizerSettings::new(rate as i32);
        let jumps = looping
            .then(|| Loop::of(data, rate, settings.block_size))
            .flatten();
        let synthesizer = Synthesizer::new(&soundfont, &settings)
            .map_err(|error| DecodeError::Unsupported(format!("MIDI: {error}")))?;
        let mut sequencer = MidiFileSequencer::new(synthesizer);
        sequencer.play(&file, looping);
        Ok(MidiSource {
            file,
            sequencer,
            looping,
            jumps,
            step: f64::from(rate) / f64::from(sample_rate),
            frames: Vec::new(),
            at: 0.0,
            before: 0,
            left: vec![0.0; BLOCK],
            right: vec![0.0; BLOCK],
        })
    }

    /// Has the synthesiser render its next `frames` frames, at most [`BLOCK`], into `left`
    /// and `right`.
    fn synthesise(&mut self, frames: usize) {
        self.sequencer
            .render(&mut self.left[..frames], &mut self.right[..frames]);
        SYNTHESISED.set(SYNTHESISED.get() + frames as u64);
    }

    /// Drops the frames before the one at `at` and appends the next block of the
    /// synthesiser. Returns `false` when the track has ended.
    fn refill(&mut self) -> bool {
        let consumed = (self.at as usize).min(self.frames.len() / 2);
        self.frames.drain(..consumed * 2);
        self.at -= consumed as f64;
        self.before += consumed as u64;
        if self.sequencer.end_of_sequence() {
            return false;
        }
        self.synthesise(BLOCK);
        let pairs = self.left.iter().zip(&self.right);
        self.frames
            .extend(pairs.flat_map(|(left, right)| [*left, *right]));
        true
    }
}

impl Source for MidiSource {
    fn read(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for frame in out.as_chunks_mut::<2>().0 {
            // Interpolation needs the frame at the position and the one after it.
            while self.at as usize + 1 >= self.frames.len() / 2 && self.refill() {}
            let count = self.frames.len() / 2;
            let index = self.at as usize;
            if index >= count {
                break;
            }
            // The last frame of a track that ended has no successor and is taken as it is.
            let next = (index + 1).min(count - 1);
            let weight = (self.at - index as f64) as f32;
            for (side, sample) in frame.iter_mut().enumerate() {
                let from = self.frames[index * 2 + side];
                let to = self.frames[next * 2 + side];
                *sample = from + (to - from) * weight;
            }
            self.at += self.step;
            written += 1;
        }
        written
    }

    fn rewind(&mut self) {
        self.sequencer.play(&self.file, self.looping);
        self.frames.clear();
        self.at = 0.0;
        self.before = 0;
    }

    /// The frames of the synthesiser before the next frame given.
    fn position(&self) -> u64 {
        self.before + self.at as u64
    }

    /// Renders up to `position`, which for a looping track is first reduced into the first
    /// pass through its loop: after many passes the track sounds the same as in that one,
    /// and the position counts on from there.
    fn start_from(&mut self, position: u64) {
        let position = self.jumps.map_or(position, |jumps| jumps.reduce(position));
        let mut left = position;
        while left > 0 && !self.sequencer.end_of_sequence() {
            let frames = left.min(BLOCK as u64) as usize;
            self.synthesise(frames);
            left -= frames as u64;
        }
        self.before = position - left;
    }
}
