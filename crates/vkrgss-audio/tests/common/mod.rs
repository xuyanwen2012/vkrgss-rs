//! Generated inputs and measurements shared by the tests. Nothing here opens a device.
#![allow(dead_code)]

use std::f32::consts::TAU;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// The committed fixtures: one second of a 440 Hz tone each. See `data/README.md`.
pub(crate) const OGG: &[u8] = include_bytes!("../data/tone.ogg");
pub(crate) const MP3: &[u8] = include_bytes!("../data/tone.mp3");
pub(crate) const MP2: &[u8] = include_bytes!("../data/tone.mp2");

/// The peak of the tones generated here.
pub(crate) const AMPLITUDE: f32 = 0.5;

/// A mono 16-bit PCM WAV file holding `samples`.
pub(crate) fn wav_of(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

/// A mono 16-bit WAV file with a sine tone of [`AMPLITUDE`].
pub(crate) fn tone_wav(sample_rate: u32, frequency: f32, seconds: f32) -> Vec<u8> {
    let count = (sample_rate as f32 * seconds).round() as usize;
    let samples: Vec<i16> = (0..count)
        .map(|index| {
            let phase = TAU * frequency * index as f32 / sample_rate as f32;
            (phase.sin() * AMPLITUDE * 32767.0).round() as i16
        })
        .collect();
    wav_of(sample_rate, &samples)
}

fn variable_length(mut value: u32, out: &mut Vec<u8>) {
    let mut bytes = vec![(value & 0x7f) as u8];
    value >>= 7;
    while value > 0 {
        bytes.push((value & 0x7f) as u8 | 0x80);
        value >>= 7;
    }
    bytes.reverse();
    out.extend_from_slice(&bytes);
}

/// A Standard MIDI file with one note, middle A on a piano, that sounds for `seconds`.
pub(crate) fn one_note_midi(seconds: u32) -> Vec<u8> {
    // 480 ticks to the quarter note at the default tempo of 120: 960 ticks to the second.
    let mut track = vec![0x00, 0xc0, 0x00, 0x00, 0x90, 69, 100];
    variable_length(960 * seconds, &mut track);
    track.extend_from_slice(&[0x80, 69, 0, 0x00, 0xff, 0x2f, 0x00]);
    midi_of(&track)
}

/// The notes of [`two_note_midi`] and their frequencies: neither is an overtone of the other.
pub(crate) const FIRST_NOTE: (u8, f32) = (69, 440.0);
pub(crate) const SECOND_NOTE: (u8, f32) = (74, 587.33);

/// A Standard MIDI file of two seconds: [`FIRST_NOTE`] on an organ for the first second and
/// [`SECOND_NOTE`] for the other. With `marker`, a controller 111 stands between the two.
pub(crate) fn two_note_midi(marker: bool) -> Vec<u8> {
    let mut track = vec![0x00, 0xc0, 19, 0x00, 0x90, FIRST_NOTE.0, 100];
    variable_length(960, &mut track);
    track.extend_from_slice(&[0x80, FIRST_NOTE.0, 0]);
    if marker {
        track.extend_from_slice(&[0x00, 0xb0, 111, 0]);
    }
    track.extend_from_slice(&[0x00, 0x90, SECOND_NOTE.0, 100]);
    variable_length(960, &mut track);
    track.extend_from_slice(&[0x80, SECOND_NOTE.0, 0, 0x00, 0xff, 0x2f, 0x00]);
    midi_of(&track)
}

/// A Standard MIDI file with the one track `track`, 480 ticks to the quarter note.
fn midi_of(track: &[u8]) -> Vec<u8> {
    let mut midi = Vec::new();
    midi.extend_from_slice(b"MThd");
    midi.extend_from_slice(&6u32.to_be_bytes());
    midi.extend_from_slice(&0u16.to_be_bytes());
    midi.extend_from_slice(&1u16.to_be_bytes());
    midi.extend_from_slice(&480u16.to_be_bytes());
    midi.extend_from_slice(b"MTrk");
    midi.extend_from_slice(&(track.len() as u32).to_be_bytes());
    midi.extend_from_slice(track);
    midi
}

/// Where each frame of [`MP3`] starts: behind its ID3v2 tag, frames of MPEG-1 layer III.
fn mp3_frames() -> Vec<usize> {
    const KILOBITS: [usize; 15] = [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
    ];
    const RATES: [usize; 3] = [44100, 48000, 32000];
    assert_eq!(&MP3[..3], b"ID3");
    let tag = MP3[6..10]
        .iter()
        .fold(0, |size, byte| size << 7 | usize::from(*byte));
    let mut frames = Vec::new();
    let mut at = 10 + tag;
    while at + 4 <= MP3.len() {
        let header = &MP3[at..at + 4];
        assert_eq!((header[0], header[1] & 0xfe), (0xff, 0xfa), "frame at {at}");
        let bit_rate = KILOBITS[usize::from(header[2] >> 4)] * 1000;
        let sample_rate = RATES[usize::from(header[2] >> 2 & 3)];
        let padding = usize::from(header[2] >> 1 & 1);
        frames.push(at);
        at += 144 * bit_rate / sample_rate + padding;
    }
    frames
}

/// [`MP3`] with every frame from the frame `first` on damaged: the frame headers stay, so
/// the file is still found to be MP3, and the side information behind each is overwritten,
/// so the frame does not decode.
pub(crate) fn damaged_mp3(first: usize) -> Vec<u8> {
    let mut data = MP3.to_vec();
    let frames = mp3_frames();
    assert!(frames.len() > 30, "{} frames", frames.len());
    for start in &frames[first..] {
        // Without a checksum, which the header says, the side information of a mono frame
        // is the 17 bytes behind the header.
        let end = (start + 4 + 17).min(data.len());
        data[start + 4..end].fill(0xff);
    }
    data
}

/// One side of an interleaved stereo buffer: 0 is left, 1 is right.
pub(crate) fn side(stereo: &[f32], side: usize) -> Vec<f32> {
    stereo.iter().skip(side).step_by(2).copied().collect()
}

pub(crate) fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

/// The frequency of a tone, from the number of times it crosses zero upwards.
pub(crate) fn frequency(samples: &[f32], sample_rate: u32) -> f32 {
    let crossings = samples
        .windows(2)
        .filter(|pair| pair[0] < 0.0 && pair[1] >= 0.0)
        .count();
    crossings as f32 * sample_rate as f32 / samples.len() as f32
}

/// The amplitude of the part of `samples` that is a tone of `frequency`.
pub(crate) fn level(samples: &[f32], sample_rate: u32, frequency: f32) -> f32 {
    let (mut sine, mut cosine) = (0.0f64, 0.0f64);
    for (index, sample) in samples.iter().enumerate() {
        let phase = f64::from(TAU * frequency) * index as f64 / f64::from(sample_rate);
        sine += f64::from(*sample) * phase.sin();
        cosine += f64::from(*sample) * phase.cos();
    }
    (2.0 * sine.hypot(cosine) / samples.len() as f64) as f32
}

/// The time in seconds after which `stereo` stays silent.
pub(crate) fn sounding_seconds(stereo: &[f32], sample_rate: u32) -> f32 {
    let last = stereo.iter().rposition(|sample| sample.abs() > 1e-3);
    last.map_or(0.0, |index| (index / 2 + 1) as f32 / sample_rate as f32)
}

/// An error stream that can be read back.
#[derive(Clone, Default)]
pub(crate) struct Reports(Arc<Mutex<Vec<u8>>>);

impl Reports {
    pub(crate) fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }

    pub(crate) fn lines(&self) -> usize {
        self.text().lines().count()
    }
}

impl Write for Reports {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
