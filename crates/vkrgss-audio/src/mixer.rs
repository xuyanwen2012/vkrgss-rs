//! The mixer: the four RGSS channels, rendered into a buffer.
//!
//! A [`Mixer`] is two halves. The loader turns an audio command into a prepared one: it opens
//! the source of a play command and reports what cannot be played. The channels apply
//! prepared commands and render. [`Mixer`] runs both in place, which is what the tests use;
//! [`Output`](crate::Output) runs the loader on a worker thread and the channels in the
//! callback of the device.
//!
//! The loader is also what remembers where the music and the background sound are, for the
//! save states: it opened their sources, and each voice tells it its position through a
//! number the two share.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use vkrgss_proto::{AudioChannel, AudioCommand};

use crate::decode::{self, DecodeError, Report, Source};
use crate::midi::SoundFonts;
use crate::wma;

/// The range of the pitch of a play command, in percent.
const PITCH: std::ops::RangeInclusive<u16> = 1..=u16::MAX;

/// How a mixer is set up.
pub struct MixerOptions {
    /// Where the SoundFont for MIDI comes from.
    pub soundfonts: SoundFonts,
    /// Where sounds that cannot be played are reported.
    pub errors: Box<dyn Write + Send>,
    /// The program that decodes WMA: `ffmpeg`, looked up on the search path, or a path.
    pub ffmpeg: PathBuf,
}

impl Default for MixerOptions {
    fn default() -> MixerOptions {
        MixerOptions {
            soundfonts: SoundFonts::configured(None),
            errors: Box::new(std::io::stderr()),
            ffmpeg: PathBuf::from("ffmpeg"),
        }
    }
}

/// An audio command whose source, if it has one, is open.
pub(crate) enum Prepared {
    Play {
        channel: AudioChannel,
        /// `None` when the data cannot be played: the channel is then silent.
        source: Option<Box<dyn Source>>,
        volume: u8,
        /// Where the voice writes its position, on the channels that are remembered.
        position: Option<Arc<AtomicU64>>,
        /// For a music effect: the time over which the music returns when it has ended.
        music_return_ms: u32,
    },
    Stop {
        channel: AudioChannel,
    },
    Fade {
        channel: AudioChannel,
        duration_ms: u32,
    },
    Volume {
        channel: AudioChannel,
        volume: u8,
    },
}

/// What has been reported, and where.
struct Reports {
    errors: Box<dyn Write + Send>,
    undecodable_reported: bool,
    ffmpeg_reported: bool,
    soundfont_reported: bool,
}

impl Reports {
    /// Writes the report of `error` unless one of its kind has been written.
    fn report(&mut self, error: &DecodeError) {
        let (reported, consequence) = match error {
            // Its own kind: other data that does not decode must not hide what WMA needs.
            DecodeError::Unsupported(reason) if reason.starts_with(wma::NEEDS_FFMPEG) => {
                (&mut self.ffmpeg_reported, "WMA sounds stay silent")
            }
            DecodeError::Unsupported(_) => (
                &mut self.undecodable_reported,
                "sounds that cannot be decoded stay silent",
            ),
            DecodeError::Damaged(_) => (
                &mut self.undecodable_reported,
                "what cannot be decoded of a sound is left out",
            ),
            DecodeError::NoSoundFont { .. } | DecodeError::SoundFont { .. } => {
                (&mut self.soundfont_reported, "MIDI music stays silent")
            }
        };
        if !std::mem::replace(reported, true) {
            // Nothing useful can be done when the error stream itself fails.
            let _ = writeln!(self.errors, "vkrgss: audio: {error}; {consequence}");
        }
    }
}

/// What the loader gave a channel to play.
struct Playing {
    /// The file of the play command, which is what tells one file from another: the command
    /// carries no name.
    data: Arc<[u8]>,
    /// The position of the voice, which the voice writes. The voice holds the other
    /// reference to it, so a count of one says that the voice is gone.
    position: Arc<AtomicU64>,
}

/// A file and a position in it, or nothing for a channel that was silent.
type Remembered = Option<(Arc<[u8]>, u64)>;

/// The place of a channel among the two that are remembered.
fn remembered(channel: AudioChannel) -> Option<usize> {
    match channel {
        AudioChannel::Bgm => Some(0),
        AudioChannel::Bgs => Some(1),
        AudioChannel::Me | AudioChannel::Se => None,
    }
}

/// Opens the sources of play commands and reports the failures, each kind once: its own and,
/// through the same [`Report`], the damage a source meets later while it plays.
pub(crate) struct Loader {
    sample_rate: u32,
    soundfonts: SoundFonts,
    ffmpeg: PathBuf,
    report: Report,
    /// What the music and the background sound play.
    playing: [Option<Playing>; 2],
    /// What each slot remembers of the two.
    slots: HashMap<u8, [Remembered; 2]>,
    /// What the next play command of each of the two starts from, if it plays that file.
    armed: [Remembered; 2],
}

impl Loader {
    /// Stores under `slot` what the music and the background sound play and where they are.
    pub(crate) fn remember(&mut self, slot: u8) {
        let playing = self.playing.each_ref().map(|playing| {
            let playing = playing.as_ref()?;
            // A voice that a fade ended, or that failed to start, left its number behind.
            (Arc::strong_count(&playing.position) > 1).then(|| {
                let position = playing.position.load(Ordering::Relaxed);
                (playing.data.clone(), position)
            })
        });
        self.slots.insert(slot, playing);
    }

    /// Arms the music and the background sound with what `slot` holds.
    pub(crate) fn recall(&mut self, slot: u8) {
        self.armed = self.slots.get(&slot).cloned().unwrap_or_default();
    }

    pub(crate) fn prepare(&mut self, command: AudioCommand) -> Prepared {
        match command {
            AudioCommand::Play {
                channel,
                data,
                volume,
                pitch,
                start,
                music_return_ms,
            } => {
                // A sound opened for a rate other than the output's, and played at the
                // output's, is faster or slower by their ratio. That is the pitch.
                let pitch = u64::from(pitch.clamp(*PITCH.start(), *PITCH.end()));
                let rate = (u64::from(self.sample_rate) * 100 + pitch / 2) / pitch;
                let data: Arc<[u8]> = data.into();
                let opened = decode::open(
                    data.clone(),
                    rate as u32,
                    loops(channel),
                    &mut self.soundfonts,
                    &self.ffmpeg,
                    &self.report,
                );
                let mut source = opened.map_err(|error| (self.report)(&error)).ok();
                // Whatever the command plays, the channel is armed no longer.
                let armed = remembered(channel).and_then(|index| self.armed[index].take());
                if let Some(source) = source.as_mut() {
                    // A recalled position knows better than the command a resumed guest
                    // repeats.
                    match armed {
                        Some((file, recalled)) if file == data => source.start_from(recalled),
                        _ if start > 0 => source.start_at_file_frame(start),
                        _ => {}
                    }
                }
                let mut position = None;
                if let Some(index) = remembered(channel) {
                    self.playing[index] = source.as_mut().map(|source| {
                        let shared = Arc::new(AtomicU64::new(source.position()));
                        position = Some(shared.clone());
                        Playing {
                            data,
                            position: shared,
                        }
                    });
                }
                Prepared::Play {
                    channel,
                    source,
                    volume,
                    position,
                    music_return_ms,
                }
            }
            AudioCommand::Stop { channel } => {
                if let Some(index) = remembered(channel) {
                    self.playing[index] = None;
                }
                Prepared::Stop { channel }
            }
            AudioCommand::Fade {
                channel,
                duration_ms,
            } => Prepared::Fade {
                channel,
                duration_ms,
            },
            AudioCommand::Volume { channel, volume } => Prepared::Volume { channel, volume },
        }
    }
}

/// Whether what plays on `channel` starts again when it ends.
fn loops(channel: AudioChannel) -> bool {
    matches!(channel, AudioChannel::Bgm | AudioChannel::Bgs)
}

/// A fade to silence: `remaining` of `total` frames are left.
#[derive(Clone, Copy)]
struct Fade {
    remaining: u32,
    total: u32,
}

/// A rise from silence to the volume of the voice: `done` of `total` frames have passed.
#[derive(Clone, Copy)]
struct Rise {
    done: u32,
    total: u32,
}

/// The frames of `duration_ms` milliseconds at `sample_rate`.
fn frames_of(duration_ms: u32, sample_rate: u32) -> u32 {
    let frames = u64::from(duration_ms) * u64::from(sample_rate) / 1000;
    frames.min(u64::from(u32::MAX)) as u32
}

/// The gain of the volume `v`, 0 to 100: 0.35 dB below full for every unit under 100, as the
/// original asks of its audio back end (read from the code of RGSS104E), and silence at 0 and
/// for a volume that is not a number or not finite.
fn gain(v: f32) -> f32 {
    if !v.is_finite() || v <= 0.0 {
        0.0
    } else {
        10f32.powf(-0.0175 * (100.0 - v))
    }
}

/// One sound on a channel.
struct Voice {
    source: Box<dyn Source>,
    /// The volume, 0 to 100, which a fade lowers in proportion.
    volume: f32,
    looping: bool,
    fade: Option<Fade>,
    /// The return of the music after a music effect.
    rise: Option<Rise>,
    /// For a music effect: the frames over which the music returns when it has ended.
    music_return: u32,
    /// Where the position of the source is written each time the voice was mixed.
    position: Option<Arc<AtomicU64>>,
}

impl Voice {
    fn new(
        source: Box<dyn Source>,
        volume: u8,
        looping: bool,
        position: Option<Arc<AtomicU64>>,
    ) -> Voice {
        Voice {
            source,
            volume: f32::from(volume.min(100)),
            looping,
            fade: None,
            rise: None,
            music_return: 0,
            position,
        }
    }

    /// Adds the next frames of the voice to `out` and returns how many frames it sounded
    /// for. Fewer than `out.len() / 2` means the voice has ended.
    fn mix(&mut self, out: &mut [f32], scratch: &mut [f32]) -> usize {
        let scratch = &mut scratch[..out.len()];
        let frames = out.len() / 2;
        let mut filled = 0;
        let mut rewound = false;
        while filled < frames {
            let read = self.source.read(&mut scratch[filled * 2..]);
            filled += read;
            if filled == frames || !self.looping || (rewound && read == 0) {
                // The last case is a track without a single frame, which cannot loop.
                break;
            }
            self.source.rewind();
            rewound = true;
        }
        if let Some(position) = &self.position {
            position.store(self.source.position(), Ordering::Relaxed);
        }
        let steady = gain(self.volume);
        let pairs = out.as_chunks_mut::<2>().0.iter_mut();
        for (index, (frame, sound)) in pairs.zip(scratch.as_chunks::<2>().0).enumerate() {
            if index == filled {
                break;
            }
            let mut gain = steady;
            let mut level = 1.0;
            if let Some(fade) = &mut self.fade {
                if fade.remaining == 0 {
                    return index;
                }
                level = fade.remaining as f32 / fade.total as f32;
                fade.remaining -= 1;
            }
            if let Some(rise) = &mut self.rise {
                level *= rise.done as f32 / rise.total as f32;
                rise.done += 1;
                if rise.done >= rise.total {
                    self.rise = None;
                }
            }
            if level < 1.0 {
                gain = self::gain(self.volume * level);
            }
            frame[0] += sound[0] * gain;
            frame[1] += sound[1] * gain;
        }
        filled
    }

    /// Lets `frames` pass without sound: a fade goes on while the voice is paused. Returns
    /// whether the voice is still to be played.
    fn pause(&mut self, frames: usize) -> bool {
        match &mut self.fade {
            Some(fade) => {
                fade.remaining = fade.remaining.saturating_sub(frames as u32);
                fade.remaining > 0
            }
            None => true,
        }
    }

    /// Lets the volume rise from silence over `frames`, evenly in the volume number. Over no
    /// frames the voice is at its volume at once, also when an earlier rise was not over.
    fn rise(&mut self, frames: u32) {
        self.rise = (frames > 0).then_some(Rise {
            done: 0,
            total: frames,
        });
    }

    /// Lowers the volume to silence over `frames`, which are more than 0: a fade of no time
    /// is a stop of the channel, see [`Channels::apply`].
    fn fade(&mut self, frames: u32) {
        // A second fade does not make a fading voice louder again. A fade of no frames has
        // nothing left of its level.
        let level = self.fade.map_or(1.0, |fade| match fade.total {
            0 => 0.0,
            total => fade.remaining as f32 / total as f32,
        });
        self.volume *= level;
        self.fade = Some(Fade {
            remaining: frames,
            total: frames,
        });
    }
}

/// The voices of the four channels.
pub(crate) struct Channels {
    sample_rate: u32,
    bgm: Option<Voice>,
    bgs: Option<Voice>,
    me: Option<Voice>,
    se: Vec<Voice>,
    /// Where a voice is read before it is added to the output.
    scratch: Vec<f32>,
}

impl Channels {
    pub(crate) fn apply(&mut self, command: Prepared) {
        match command {
            Prepared::Play {
                channel,
                source,
                volume,
                position,
                music_return_ms,
            } => {
                let voice = source.map(|source| {
                    let mut voice = Voice::new(source, volume, loops(channel), position);
                    voice.music_return = frames_of(music_return_ms, self.sample_rate);
                    voice
                });
                match channel {
                    AudioChannel::Bgm => self.bgm = voice,
                    AudioChannel::Bgs => self.bgs = voice,
                    AudioChannel::Me => self.me = voice,
                    AudioChannel::Se => self.se.extend(voice),
                }
            }
            Prepared::Stop { channel } => match channel {
                AudioChannel::Bgm => self.bgm = None,
                AudioChannel::Bgs => self.bgs = None,
                AudioChannel::Me => self.end_effect(),
                AudioChannel::Se => self.se.clear(),
            },
            Prepared::Fade {
                channel,
                duration_ms,
            } => {
                // A fade of no time ends its channel at once, as a stop does, so no later
                // fade meets a fade of no frames.
                match frames_of(duration_ms, self.sample_rate) {
                    0 => self.apply(Prepared::Stop { channel }),
                    frames => self.voices(channel).for_each(|voice| voice.fade(frames)),
                }
            }
            Prepared::Volume { channel, volume } => {
                let volume = f32::from(volume.min(100));
                self.voices(channel).for_each(|voice| voice.volume = volume);
            }
        }
    }

    /// The voices of a channel.
    fn voices(&mut self, channel: AudioChannel) -> Box<dyn Iterator<Item = &mut Voice> + '_> {
        match channel {
            AudioChannel::Bgm => Box::new(self.bgm.iter_mut()),
            AudioChannel::Bgs => Box::new(self.bgs.iter_mut()),
            AudioChannel::Me => Box::new(self.me.iter_mut()),
            AudioChannel::Se => Box::new(self.se.iter_mut()),
        }
    }

    /// Ends the music effect, if one plays, and lets the music return over its time.
    fn end_effect(&mut self) {
        if let Some(effect) = self.me.take()
            && let Some(music) = &mut self.bgm
        {
            music.rise(effect.music_return);
        }
    }

    /// Fills `out`, interleaved stereo, with the next frames of the mix.
    pub(crate) fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let frames = out.len() / 2;
        let out = &mut out[..frames * 2];
        let mut scratch = std::mem::take(&mut self.scratch);
        if scratch.len() < out.len() {
            scratch.resize(out.len(), 0.0);
        }

        // The music effect comes first: the music is silent for as long as it sounds.
        let mut effect = 0;
        if let Some(voice) = &mut self.me {
            effect = voice.mix(out, &mut scratch);
            if effect < frames {
                self.end_effect();
            }
        }
        if let Some(voice) = &mut self.bgm {
            let played = voice.pause(effect)
                && voice.mix(&mut out[effect * 2..], &mut scratch) == frames - effect;
            if !played {
                self.bgm = None;
            }
        }
        if let Some(voice) = &mut self.bgs
            && voice.mix(out, &mut scratch) < frames
        {
            self.bgs = None;
        }
        self.se
            .retain_mut(|voice| voice.mix(out, &mut scratch) == frames);

        for sample in out.iter_mut() {
            // `clamp` keeps a NaN, and the device must not get one.
            *sample = match sample.is_nan() {
                true => 0.0,
                false => sample.clamp(-1.0, 1.0),
            };
        }
        self.scratch = scratch;
    }
}

/// The four channels of RGSS: music and background sound loop, a music effect pauses the
/// music while it plays, sound effects overlap.
///
/// Volume is a gain in decibels, 0.35 dB below full for every unit under 100, as in the
/// original; pitch is a resampling ratio; a fade lowers the volume evenly, so its ramp is
/// even in decibels until it ends in silence. The mixer has no thread and no device, and
/// what it renders depends only on the commands and the number of frames rendered.
///
/// For the save states it remembers where the music and the background sound are, see
/// [`remember`](Mixer::remember) and [`recall`](Mixer::recall). What it remembers lives as
/// long as it does.
pub struct Mixer {
    loader: Loader,
    channels: Channels,
}

impl Mixer {
    /// A mixer that renders at `sample_rate` frames per second.
    pub fn new(sample_rate: u32, options: MixerOptions) -> Mixer {
        let reports = Mutex::new(Reports {
            errors: options.errors,
            undecodable_reported: false,
            ffmpeg_reported: false,
            soundfont_reported: false,
        });
        let report: Report = Arc::new(move |error: &DecodeError| {
            let mut reports = reports.lock().unwrap_or_else(PoisonError::into_inner);
            reports.report(error);
        });
        Mixer {
            loader: Loader {
                sample_rate,
                soundfonts: options.soundfonts,
                ffmpeg: options.ffmpeg,
                report,
                playing: [None, None],
                slots: HashMap::new(),
                armed: [None, None],
            },
            channels: Channels {
                sample_rate,
                bgm: None,
                bgs: None,
                me: None,
                se: Vec::new(),
                scratch: Vec::new(),
            },
        }
    }

    /// Applies a command. A play command whose data cannot be played leaves its channel
    /// silent, as a stop command does for the channels with one track.
    pub fn apply(&mut self, command: AudioCommand) {
        let prepared = self.loader.prepare(command);
        self.channels.apply(prepared);
    }

    /// Fills `out`, interleaved stereo, with the next frames of the mix.
    pub fn render(&mut self, out: &mut [f32]) {
        self.channels.render(out);
    }

    /// Remembers under `slot`, in place of what it held, the file and the position of the
    /// music and of the background sound, or that the channel is silent. The music effect
    /// and the sound effects are not remembered.
    pub fn remember(&mut self, slot: u8) {
        self.loader.remember(slot);
    }

    /// Arms the music and the background sound with what `slot` holds. The next play
    /// command on each of the two disarms it, and starts at the remembered position if its
    /// file is the remembered one; its volume and pitch are its own, and a fade is not
    /// brought back.
    pub fn recall(&mut self, slot: u8) {
        self.loader.recall(slot);
    }

    pub(crate) fn into_parts(self) -> (Loader, Channels) {
        (self.loader, self.channels)
    }
}
