//! The mixer: the four RGSS channels, rendered into a buffer.
//!
//! A [`Mixer`] is two halves. The loader turns an audio command into a prepared one: it opens
//! the source of a play command and reports what cannot be played. The channels apply
//! prepared commands and render. [`Mixer`] runs both in place, which is what the tests use;
//! [`Output`](crate::Output) runs the loader on a worker thread and the channels in the
//! callback of the device.

use std::io::Write;

use vkxp_proto::{AudioChannel, AudioCommand};

use crate::decode::{self, DecodeError, Source};
use crate::midi::SoundFonts;

/// The range of the pitch of a play command, in percent.
const PITCH: std::ops::RangeInclusive<u16> = 50..=150;

/// How a mixer is set up.
pub struct MixerOptions {
    /// Where the SoundFont for MIDI comes from.
    pub soundfonts: SoundFonts,
    /// Where sounds that cannot be played are reported.
    pub errors: Box<dyn Write + Send>,
}

impl Default for MixerOptions {
    fn default() -> MixerOptions {
        MixerOptions {
            soundfonts: SoundFonts::configured(None),
            errors: Box::new(std::io::stderr()),
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
    },
    Stop {
        channel: AudioChannel,
    },
    Fade {
        channel: AudioChannel,
        duration_ms: u32,
    },
}

/// Opens the sources of play commands and reports the failures, each kind once.
pub(crate) struct Loader {
    sample_rate: u32,
    soundfonts: SoundFonts,
    errors: Box<dyn Write + Send>,
    unsupported_reported: bool,
    soundfont_reported: bool,
}

impl Loader {
    pub(crate) fn prepare(&mut self, command: AudioCommand) -> Prepared {
        match command {
            AudioCommand::Play {
                channel,
                data,
                volume,
                pitch,
            } => {
                // A sound opened for a rate other than the output's, and played at the
                // output's, is faster or slower by their ratio. That is the pitch.
                let pitch = u64::from(pitch.clamp(*PITCH.start(), *PITCH.end()));
                let rate = (u64::from(self.sample_rate) * 100 + pitch / 2) / pitch;
                let source = decode::open(data.into(), rate as u32, &mut self.soundfonts)
                    .map_err(|error| self.report(&error))
                    .ok();
                Prepared::Play {
                    channel,
                    source,
                    volume,
                }
            }
            AudioCommand::Stop { channel } => Prepared::Stop { channel },
            AudioCommand::Fade {
                channel,
                duration_ms,
            } => Prepared::Fade {
                channel,
                duration_ms,
            },
        }
    }

    fn report(&mut self, error: &DecodeError) {
        let (reported, consequence) = match error {
            DecodeError::Unsupported(_) => (
                &mut self.unsupported_reported,
                "sounds that cannot be decoded stay silent",
            ),
            DecodeError::NoSoundFont { .. } | DecodeError::SoundFont { .. } => {
                (&mut self.soundfont_reported, "MIDI music stays silent")
            }
        };
        if !std::mem::replace(reported, true) {
            // Nothing useful can be done when the error stream itself fails.
            let _ = writeln!(self.errors, "vkxp: audio: {error}; {consequence}");
        }
    }
}

/// A fade to silence: `remaining` of `total` frames are left.
#[derive(Clone, Copy)]
struct Fade {
    remaining: u32,
    total: u32,
}

/// One sound on a channel.
struct Voice {
    source: Box<dyn Source>,
    volume: f32,
    looping: bool,
    fade: Option<Fade>,
}

impl Voice {
    fn new(source: Box<dyn Source>, volume: u8, looping: bool) -> Voice {
        Voice {
            source,
            volume: f32::from(volume.min(100)) / 100.0,
            looping,
            fade: None,
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
        let pairs = out.as_chunks_mut::<2>().0.iter_mut();
        for (index, (frame, sound)) in pairs.zip(scratch.as_chunks::<2>().0).enumerate() {
            if index == filled {
                break;
            }
            let mut gain = self.volume;
            if let Some(fade) = &mut self.fade {
                if fade.remaining == 0 {
                    return index;
                }
                gain *= fade.remaining as f32 / fade.total as f32;
                fade.remaining -= 1;
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

    fn fade(&mut self, frames: u32) {
        // A second fade does not make a fading voice louder again.
        let level = self
            .fade
            .map_or(1.0, |fade| fade.remaining as f32 / fade.total as f32);
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
            } => {
                let looping = matches!(channel, AudioChannel::Bgm | AudioChannel::Bgs);
                let voice = source.map(|source| Voice::new(source, volume, looping));
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
                AudioChannel::Me => self.me = None,
                AudioChannel::Se => self.se.clear(),
            },
            Prepared::Fade {
                channel,
                duration_ms,
            } => {
                let frames = u64::from(duration_ms) * u64::from(self.sample_rate) / 1000;
                let frames = frames.min(u64::from(u32::MAX)) as u32;
                let voices: &mut dyn Iterator<Item = &mut Voice> = match channel {
                    AudioChannel::Bgm => &mut self.bgm.iter_mut(),
                    AudioChannel::Bgs => &mut self.bgs.iter_mut(),
                    AudioChannel::Me => &mut self.me.iter_mut(),
                    AudioChannel::Se => &mut self.se.iter_mut(),
                };
                voices.for_each(|voice| voice.fade(frames));
            }
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
                self.me = None;
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
            *sample = sample.clamp(-1.0, 1.0);
        }
        self.scratch = scratch;
    }
}

/// The four channels of RGSS: music and background sound loop, a music effect pauses the
/// music while it plays, sound effects overlap.
///
/// Volume is a linear factor (PLACEHOLDER: the original maps volume to decibels), pitch a
/// resampling ratio and a fade a linear ramp. The mixer has no thread and no device, and
/// what it renders depends only on the commands and the number of frames rendered.
pub struct Mixer {
    loader: Loader,
    channels: Channels,
}

impl Mixer {
    /// A mixer that renders at `sample_rate` frames per second.
    pub fn new(sample_rate: u32, options: MixerOptions) -> Mixer {
        Mixer {
            loader: Loader {
                sample_rate,
                soundfonts: options.soundfonts,
                errors: options.errors,
                unsupported_reported: false,
                soundfont_reported: false,
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
}
