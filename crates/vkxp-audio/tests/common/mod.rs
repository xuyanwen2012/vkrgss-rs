//! Generated inputs and measurements shared by the tests. Nothing here opens a device.
#![allow(dead_code)]

use std::f32::consts::TAU;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// The committed fixtures: one second of a 440 Hz tone each. See `data/README.md`.
pub const OGG: &[u8] = include_bytes!("../data/tone.ogg");
pub const MP3: &[u8] = include_bytes!("../data/tone.mp3");

/// The peak of the tones generated here.
pub const AMPLITUDE: f32 = 0.5;

/// A mono 16-bit PCM WAV file holding `samples`.
pub fn wav_of(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
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
pub fn tone_wav(sample_rate: u32, frequency: f32, seconds: f32) -> Vec<u8> {
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
pub fn one_note_midi(seconds: u32) -> Vec<u8> {
    // 480 ticks to the quarter note at the default tempo of 120: 960 ticks to the second.
    let mut track = vec![0x00, 0xc0, 0x00, 0x00, 0x90, 69, 100];
    variable_length(960 * seconds, &mut track);
    track.extend_from_slice(&[0x80, 69, 0, 0x00, 0xff, 0x2f, 0x00]);
    let mut midi = Vec::new();
    midi.extend_from_slice(b"MThd");
    midi.extend_from_slice(&6u32.to_be_bytes());
    midi.extend_from_slice(&0u16.to_be_bytes());
    midi.extend_from_slice(&1u16.to_be_bytes());
    midi.extend_from_slice(&480u16.to_be_bytes());
    midi.extend_from_slice(b"MTrk");
    midi.extend_from_slice(&(track.len() as u32).to_be_bytes());
    midi.extend_from_slice(&track);
    midi
}

/// One side of an interleaved stereo buffer: 0 is left, 1 is right.
pub fn side(stereo: &[f32], side: usize) -> Vec<f32> {
    stereo.iter().skip(side).step_by(2).copied().collect()
}

pub fn peak(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

/// The frequency of a tone, from the number of times it crosses zero upwards.
pub fn frequency(samples: &[f32], sample_rate: u32) -> f32 {
    let crossings = samples
        .windows(2)
        .filter(|pair| pair[0] < 0.0 && pair[1] >= 0.0)
        .count();
    crossings as f32 * sample_rate as f32 / samples.len() as f32
}

/// The amplitude of the part of `samples` that is a tone of `frequency`.
pub fn level(samples: &[f32], sample_rate: u32, frequency: f32) -> f32 {
    let (mut sine, mut cosine) = (0.0f64, 0.0f64);
    for (index, sample) in samples.iter().enumerate() {
        let phase = f64::from(TAU * frequency) * index as f64 / f64::from(sample_rate);
        sine += f64::from(*sample) * phase.sin();
        cosine += f64::from(*sample) * phase.cos();
    }
    (2.0 * sine.hypot(cosine) / samples.len() as f64) as f32
}

/// The time in seconds after which `stereo` stays silent.
pub fn sounding_seconds(stereo: &[f32], sample_rate: u32) -> f32 {
    let last = stereo.iter().rposition(|sample| sample.abs() > 1e-3);
    last.map_or(0.0, |index| (index / 2 + 1) as f32 / sample_rate as f32)
}

/// An error stream that can be read back.
#[derive(Clone, Default)]
pub struct Reports(Arc<Mutex<Vec<u8>>>);

impl Reports {
    pub fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }

    pub fn lines(&self) -> usize {
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
