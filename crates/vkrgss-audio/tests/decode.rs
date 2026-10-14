//! The format scenarios of `specs/audio-playback` at the level of the sources.

mod common;

use std::path::{Path, PathBuf};

use std::sync::{Arc, Mutex};

use common::{
    AMPLITUDE, MP2, MP3, OGG, damaged_mp3, frequency, one_note_midi, peak, side, tone_wav,
};
use vkrgss_audio::decode::{self, DecodeError, Report, Source};
use vkrgss_audio::{SYSTEM_SOUNDFONTS, SoundFonts};

fn no_soundfont() -> SoundFonts {
    SoundFonts::new(Vec::new())
}

/// Opens data that is expected to play without any damage.
fn open(
    data: Arc<[u8]>,
    sample_rate: u32,
    soundfonts: &mut SoundFonts,
) -> Result<Box<dyn Source>, DecodeError> {
    let report: Report = Arc::new(|error: &DecodeError| panic!("reported: {error}"));
    decode::open(
        data,
        sample_rate,
        false,
        soundfonts,
        Path::new("ffmpeg"),
        &report,
    )
}

/// A report that keeps what it is told.
fn collecting() -> (Report, Arc<Mutex<Vec<DecodeError>>>) {
    let reported: Arc<Mutex<Vec<DecodeError>>> = Arc::default();
    let list = reported.clone();
    let report: Report = Arc::new(move |error: &DecodeError| {
        list.lock().unwrap().push(error.clone());
    });
    (report, reported)
}

/// Everything the source yields until it ends.
fn read_all(source: &mut dyn Source) -> Vec<f32> {
    let mut all = Vec::new();
    let mut block = [0.0; 2 * 1000];
    loop {
        let frames = source.read(&mut block);
        all.extend_from_slice(&block[..frames * 2]);
        if frames < block.len() / 2 {
            return all;
        }
    }
}

#[test]
fn a_wav_tone_is_the_same_tone_on_both_sides() {
    let wav = tone_wav(44100, 440.0, 1.0);
    for rate in [44100, 48000] {
        let mut source = open(wav.clone().into(), rate, &mut no_soundfont()).unwrap();
        let stereo = read_all(source.as_mut());
        assert!(stereo.len().abs_diff(rate as usize * 2) <= 4, "{rate}");
        let (left, right) = (side(&stereo, 0), side(&stereo, 1));
        assert_eq!(left, right);
        assert!((frequency(&left, rate) - 440.0).abs() < 2.0, "{rate}");
        assert!((peak(&left) - AMPLITUDE).abs() < 0.01, "{rate}");
    }
}

#[test]
fn the_ogg_mp3_and_layer_ii_fixtures_sound_for_their_second() {
    for (name, data) in [("ogg", OGG), ("mp3", MP3), ("mp2", MP2)] {
        let mut source = open(data.into(), 48000, &mut no_soundfont()).unwrap();
        let stereo = read_all(source.as_mut());
        let seconds = stereo.len() as f32 / 2.0 / 48000.0;
        assert!((seconds - 1.0).abs() < 0.1, "{name}: {seconds} s");
        assert!(peak(&stereo) > 0.1, "{name}");
        // The tone itself, away from the edges where a lossy codec rings.
        let middle = side(&stereo[9600..stereo.len() - 9600], 0);
        assert!((frequency(&middle, 48000) - 440.0).abs() < 3.0, "{name}");
    }
}

#[test]
fn a_rewound_source_yields_the_same_frames_again() {
    for data in [tone_wav(22050, 440.0, 0.25), OGG.to_vec(), MP3.to_vec()] {
        let mut source = open(data.into(), 48000, &mut no_soundfont()).unwrap();
        let first = read_all(source.as_mut());
        source.rewind();
        assert_eq!(read_all(source.as_mut()), first);
    }
}

#[test]
fn bytes_of_no_format_are_unsupported() {
    for data in [
        &b""[..],
        b"not audio at all, only a line of text",
        &[0; 4096],
    ] {
        let result = open(data.into(), 48000, &mut no_soundfont());
        assert!(matches!(result, Err(DecodeError::Unsupported(_))));
    }
}

/// Data that is recognised as MP3 and of which no frame decodes: with the first frame, which
/// only describes the file, left alone, and with every frame damaged.
#[test]
fn data_of_which_nothing_decodes_is_unsupported_with_the_reason() {
    for first in [0, 1] {
        let (report, reported) = collecting();
        let result = decode::open(
            damaged_mp3(first).into(),
            48000,
            false,
            &mut no_soundfont(),
            Path::new("ffmpeg"),
            &report,
        );
        match result {
            Err(DecodeError::Unsupported(reason)) => assert!(!reason.is_empty()),
            Err(other) => panic!("{first}: {other}"),
            Ok(_) => panic!("{first}: a source from data that does not decode"),
        }
        assert_eq!(*reported.lock().unwrap(), [], "the error is the report");
    }
}

#[test]
fn damage_behind_the_first_block_is_reported_once_and_the_rest_is_played() {
    let (report, reported) = collecting();
    let mut source = decode::open(
        damaged_mp3(20).into(),
        48000,
        false,
        &mut no_soundfont(),
        Path::new("ffmpeg"),
        &report,
    )
    .unwrap();
    assert_eq!(*reported.lock().unwrap(), [], "the first block is whole");

    let stereo = read_all(source.as_mut());
    let seconds = stereo.len() as f32 / 2.0 / 48000.0;
    assert!(
        seconds > 0.3 && seconds < 0.7,
        "about half the tone: {seconds} s"
    );
    assert!(peak(&stereo) > 0.1);
    let reports = reported.lock().unwrap().clone();
    assert!(
        matches!(reports[..], [DecodeError::Damaged(_)]),
        "{reports:?}"
    );

    // Playing it again says nothing new.
    source.rewind();
    assert_eq!(read_all(source.as_mut()).len(), stereo.len());
    assert_eq!(reported.lock().unwrap().len(), 1);
}

/// A file that was cut short ends where its data ends, and that is no damage.
#[test]
fn a_file_cut_short_plays_what_is_there_without_a_report() {
    let wav = tone_wav(44100, 440.0, 1.0);
    // The Ogg fixture has all its sound in one page, so no cut of it leaves any.
    let cut = [
        ("wav", &wav[..wav.len() / 2]),
        ("mp3", &MP3[..MP3.len() / 2]),
    ];
    for (name, data) in cut {
        let mut source = match open(data.into(), 48000, &mut no_soundfont()) {
            Ok(source) => source,
            Err(error) => panic!("{name}: {error}"),
        };
        let stereo = read_all(source.as_mut());
        let seconds = stereo.len() as f32 / 2.0 / 48000.0;
        assert!(seconds > 0.2 && seconds < 0.9, "{name}: {seconds} s");
    }
}

#[test]
fn midi_without_a_soundfont_is_the_no_soundfont_error() {
    let absent = PathBuf::from("/nonexistent/vkrgss-test.sf2");
    let mut soundfonts = SoundFonts::new(vec![absent.clone()]);
    let midi: std::sync::Arc<[u8]> = one_note_midi(1).into();
    for _ in 0..2 {
        match open(midi.clone(), 48000, &mut soundfonts) {
            Err(DecodeError::NoSoundFont { searched }) => {
                assert_eq!(searched, std::slice::from_ref(&absent))
            }
            Err(other) => panic!("{other}"),
            Ok(_) => panic!("a MIDI source without a SoundFont"),
        }
    }
    // Other formats do not need one.
    assert!(open(tone_wav(8000, 440.0, 0.1).into(), 48000, &mut soundfonts).is_ok());
}

#[test]
fn a_file_that_is_no_soundfont_is_named_in_the_error() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/README.md");
    let mut soundfonts = SoundFonts::new(vec![file.clone()]);
    match open(one_note_midi(1).into(), 48000, &mut soundfonts) {
        Err(DecodeError::SoundFont { path, .. }) => assert_eq!(path, file),
        Err(other) => panic!("{other}"),
        Ok(_) => panic!("a MIDI source from a text file"),
    }
}

/// Runs only where one of the system SoundFont locations exists.
#[test]
fn a_midi_note_sounds_while_it_is_held() {
    if !SYSTEM_SOUNDFONTS
        .iter()
        .any(|path| Path::new(path).is_file())
    {
        eprintln!("skipped: no system SoundFont");
        return;
    }
    let mut soundfonts = SoundFonts::configured(None);
    let mut source = open(one_note_midi(1).into(), 44100, &mut soundfonts).unwrap();
    let stereo = read_all(source.as_mut());
    let seconds = stereo.len() as f32 / 2.0 / 44100.0;
    assert!((seconds - 1.0).abs() < 0.1, "{seconds} s");
    // Every tenth of the second the note is held for.
    for (index, tenth) in stereo.chunks(8820).take(10).enumerate() {
        assert!(peak(tenth) > 0.01, "silent in tenth {index}");
    }
    source.rewind();
    assert_eq!(read_all(source.as_mut()).len(), stereo.len());
}
