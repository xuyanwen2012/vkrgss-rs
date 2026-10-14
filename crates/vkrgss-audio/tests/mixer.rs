//! The scenarios of `specs/audio-playback`, rendered into buffers.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use common::{
    AMPLITUDE, FIRST_NOTE, MP2, MP3, OGG, Reports, SECOND_NOTE, damaged_mp3, frequency, level,
    one_note_midi, peak, side, sounding_seconds, tone_wav, two_note_midi, wav_of,
};
use vkrgss_audio::{Mixer, MixerOptions, SYSTEM_SOUNDFONTS, SoundFonts};
use vkrgss_proto::{AudioChannel, AudioCommand};

const RATE: u32 = 44100;

fn mixer_reporting_to(reports: &Reports) -> Mixer {
    let options = MixerOptions {
        soundfonts: SoundFonts::new(Vec::new()),
        errors: Box::new(reports.clone()),
        ..MixerOptions::default()
    };
    Mixer::new(RATE, options)
}

fn mixer() -> Mixer {
    mixer_reporting_to(&Reports::default())
}

fn play(channel: AudioChannel, data: &[u8]) -> AudioCommand {
    play_with(channel, data, 100, 100)
}

fn play_with(channel: AudioChannel, data: &[u8], volume: u8, pitch: u16) -> AudioCommand {
    AudioCommand::Play {
        channel,
        data: data.to_vec(),
        volume,
        pitch,
        start: 0,
        music_return_ms: 0,
    }
}

/// A play command with the two fields that RGSS3 uses.
fn play_from(
    channel: AudioChannel,
    data: &[u8],
    (volume, pitch): (u8, u16),
    start: u64,
    music_return_ms: u32,
) -> AudioCommand {
    AudioCommand::Play {
        channel,
        data: data.to_vec(),
        volume,
        pitch,
        start,
        music_return_ms,
    }
}

/// The next `seconds` of the mix, interleaved.
fn render(mixer: &mut Mixer, seconds: f32) -> Vec<f32> {
    let mut out = vec![0.0; (RATE as f32 * seconds).round() as usize * 2];
    // In blocks of a size a device might ask for, which no duration here is a multiple of.
    for block in out.chunks_mut(2 * 1000) {
        mixer.render(block);
    }
    out
}

fn tone(frequency: f32, seconds: f32) -> Vec<u8> {
    tone_wav(RATE, frequency, seconds)
}

#[test]
fn a_wav_tone_is_heard_on_both_output_channels() {
    let mut mixer = mixer();
    mixer.apply(play(AudioChannel::Se, &tone(440.0, 1.0)));
    let out = render(&mut mixer, 1.0);
    for channel in 0..2 {
        let heard = side(&out, channel);
        assert!((frequency(&heard, RATE) - 440.0).abs() < 2.0);
        assert!((level(&heard, RATE, 440.0) - AMPLITUDE).abs() < 0.01);
    }
    assert_eq!(
        peak(&render(&mut mixer, 0.5)),
        0.0,
        "a sound effect plays once"
    );
}

#[test]
fn the_ogg_mp3_and_layer_ii_fixtures_are_heard_for_their_second() {
    for (name, data) in [("ogg", OGG), ("mp3", MP3), ("mp2", MP2)] {
        let reports = Reports::default();
        let mut mixer = mixer_reporting_to(&reports);
        mixer.apply(play(AudioChannel::Se, data));
        let out = render(&mut mixer, 2.0);
        assert!(peak(&out) > 0.1, "{name}");
        let seconds = sounding_seconds(&out, RATE);
        assert!((seconds - 1.0).abs() < 0.1, "{name}: {seconds} s");
        assert_eq!(reports.text(), "", "{name}");
    }
}

/// A mixer with the system SoundFont, or `None`, with a note that the test is skipped,
/// where none of the system SoundFont locations exists.
fn mixer_with_soundfont(reports: &Reports) -> Option<Mixer> {
    if !SYSTEM_SOUNDFONTS
        .iter()
        .any(|path| Path::new(path).is_file())
    {
        eprintln!("skipped: no system SoundFont");
        return None;
    }
    let options = MixerOptions {
        soundfonts: SoundFonts::configured(None),
        errors: Box::new(reports.clone()),
        ..MixerOptions::default()
    };
    Some(Mixer::new(RATE, options))
}

/// Runs only where one of the system SoundFont locations exists.
#[test]
fn a_midi_note_is_heard_while_it_sounds() {
    let reports = Reports::default();
    let Some(mut mixer) = mixer_with_soundfont(&reports) else {
        return;
    };
    mixer.apply(play(AudioChannel::Bgm, &one_note_midi(1)));
    let out = render(&mut mixer, 1.0);
    for (index, tenth) in out.chunks(RATE as usize / 5).enumerate() {
        assert!(peak(tenth) > 0.01, "silent in tenth {index}");
    }
    assert_eq!(reports.text(), "");
}

/// Which of the two notes of [`two_note_midi`] sounds in each second of four, the file
/// played as music: `false` is the first note.
fn notes_heard(marker: bool) -> Option<[bool; 4]> {
    let reports = Reports::default();
    let mut mixer = mixer_with_soundfont(&reports)?;
    mixer.apply(play(AudioChannel::Bgm, &two_note_midi(marker)));
    let out = render(&mut mixer, 4.0);
    assert_eq!(reports.text(), "");
    let mut heard = [false; 4];
    for (second, heard) in heard.iter_mut().enumerate() {
        // A tenth of a second late in the second, when the note before has died away.
        let from = (second * 4 + 3) * RATE as usize / 4;
        let middle = side(&out[from * 2..(from + RATE as usize / 10) * 2], 0);
        let first = level(&middle, RATE, FIRST_NOTE.1);
        let second_note = level(&middle, RATE, SECOND_NOTE.1);
        assert!(
            first.max(second_note) > 0.002 && first.max(second_note) > 5.0 * first.min(second_note),
            "second {second}: {first} and {second_note}"
        );
        *heard = second_note > first;
    }
    Some(heard)
}

/// Scenario "Marker". Runs only where one of the system SoundFont locations exists.
#[test]
fn looping_midi_continues_at_its_loop_marker() {
    if let Some(heard) = notes_heard(true) {
        assert_eq!(heard, [false, true, true, true]);
    }
}

/// Scenario "No marker". Runs only where one of the system SoundFont locations exists.
#[test]
fn looping_midi_without_a_marker_starts_again() {
    if let Some(heard) = notes_heard(false) {
        assert_eq!(heard, [false, true, false, true]);
    }
}

/// A MIDI file that does not loop ends at its end, marker or not.
#[test]
fn midi_as_an_effect_ends_at_its_end() {
    let reports = Reports::default();
    let Some(mut mixer) = mixer_with_soundfont(&reports) else {
        return;
    };
    mixer.apply(play(AudioChannel::Me, &two_note_midi(true)));
    let out = render(&mut mixer, 4.0);
    assert!(peak(&out[..RATE as usize * 4]) > 0.01);
    assert_eq!(peak(&out[RATE as usize * 5..]), 0.0);
}

#[test]
fn music_and_background_sound_loop() {
    for channel in [AudioChannel::Bgm, AudioChannel::Bgs] {
        let mut mixer = mixer();
        mixer.apply(play(channel, &tone(440.0, 0.5)));
        let out = render(&mut mixer, 2.0);
        let last = side(&out[RATE as usize * 3..], 0);
        assert!((level(&last, RATE, 440.0) - AMPLITUDE).abs() < 0.01);
    }
}

#[test]
fn a_music_effect_plays_once() {
    let mut mixer = mixer();
    mixer.apply(play(AudioChannel::Me, &tone(440.0, 0.5)));
    let out = render(&mut mixer, 2.0);
    assert!((sounding_seconds(&out, RATE) - 0.5).abs() < 0.01);
}

#[test]
fn sound_effects_overlap() {
    let mut mixer = mixer();
    mixer.apply(play(AudioChannel::Se, &tone(440.0, 0.5)));
    let alone = side(&render(&mut mixer, 0.1), 0);
    mixer.apply(play(AudioChannel::Se, &tone(1000.0, 0.5)));
    let both = side(&render(&mut mixer, 0.4), 0);
    let after = side(&render(&mut mixer, 0.1), 0);

    assert!((level(&alone, RATE, 440.0) - AMPLITUDE).abs() < 0.01);
    assert!(level(&alone, RATE, 1000.0) < 0.01);
    assert!((level(&both, RATE, 440.0) - AMPLITUDE).abs() < 0.01);
    assert!((level(&both, RATE, 1000.0) - AMPLITUDE).abs() < 0.01);
    assert!(level(&after, RATE, 440.0) < 0.01);
    assert!((level(&after, RATE, 1000.0) - AMPLITUDE).abs() < 0.01);
}

/// Scenario "No return time".
#[test]
fn a_music_effect_pauses_the_music_which_then_continues() {
    // A ramp, so that every frame of the music tells where in the track it is.
    let ramp: Vec<i16> = (0..RATE).map(|frame| (frame / 2) as i16).collect();
    let music = wav_of(RATE, &ramp);
    let effect = tone(1000.0, 0.3);

    let mut mixer = mixer();
    mixer.apply(play(AudioChannel::Bgm, &music));
    let before = render(&mut mixer, 0.2);
    mixer.apply(play(AudioChannel::Me, &effect));
    let during = render(&mut mixer, 0.3);
    let after = render(&mut mixer, 0.2);

    let mut music_alone = self::mixer();
    music_alone.apply(play(AudioChannel::Bgm, &music));
    assert_eq!(before, render(&mut music_alone, 0.2));
    assert!(
        before[4] > 0.0 && before[before.len() - 1] > 0.1,
        "the ramp rises"
    );
    let mut effect_alone = self::mixer();
    effect_alone.apply(play(AudioChannel::Me, &effect));
    assert_eq!(
        during,
        render(&mut effect_alone, 0.3),
        "the music is silent"
    );
    assert_eq!(after, render(&mut music_alone, 0.2), "from where it paused");
}

#[test]
fn a_play_command_replaces_what_its_channel_played() {
    for channel in [AudioChannel::Bgm, AudioChannel::Bgs, AudioChannel::Me] {
        let mut mixer = mixer();
        mixer.apply(play(channel, &tone(440.0, 1.0)));
        render(&mut mixer, 0.1);
        mixer.apply(play(channel, &tone(1000.0, 1.0)));
        let out = side(&render(&mut mixer, 0.2), 0);
        assert!(level(&out, RATE, 440.0) < 0.01);
        assert!((level(&out, RATE, 1000.0) - AMPLITUDE).abs() < 0.01);
    }
}

/// Scenario "Volume curve", and the gain table of the design.
#[test]
fn volume_sets_the_gain_in_decibels() {
    let gains = [
        (100, 1.0),
        (90, 0.668),
        (80, 0.447),
        (50, 0.133),
        (20, 0.0398),
        (1, 0.0185),
        (0, 0.0),
    ];
    for (volume, gain) in gains {
        let mut mixer = mixer();
        mixer.apply(play_with(AudioChannel::Bgm, &tone(440.0, 0.5), volume, 100));
        let out = side(&render(&mut mixer, 0.5), 0);
        let heard = level(&out, RATE, 440.0) / AMPLITUDE;
        assert!(
            (heard - gain).abs() < gain * 0.01 + 0.0005,
            "{volume}: {heard}"
        );
        if volume == 0 {
            assert_eq!(peak(&out), 0.0);
        }
    }
}

#[test]
fn pitch_50_halves_the_frequency_and_doubles_the_duration() {
    let mut mixer = mixer();
    mixer.apply(play_with(AudioChannel::Se, &tone(440.0, 1.0), 100, 50));
    let out = render(&mut mixer, 3.0);
    assert!((sounding_seconds(&out, RATE) - 2.0).abs() < 0.01);
    let heard = side(&out[..RATE as usize * 4], 0);
    assert!((frequency(&heard, RATE) - 220.0).abs() < 1.0);
    assert!((level(&heard, RATE, 220.0) - AMPLITUDE).abs() < 0.01);
}

#[test]
fn pitch_150_plays_half_as_fast_again() {
    let mut mixer = mixer();
    mixer.apply(play_with(AudioChannel::Se, &tone(440.0, 1.5), 100, 150));
    let out = render(&mut mixer, 2.0);
    assert!((sounding_seconds(&out, RATE) - 1.0).abs() < 0.01);
    let heard = side(&out[..RATE as usize * 2], 0);
    assert!((frequency(&heard, RATE) - 660.0).abs() < 2.0);
}

/// Scenario "Pitch outside 50 to 150".
#[test]
fn pitch_outside_50_to_150() {
    for (pitch, heard_frequency, seconds) in [(200, 880.0, 0.5), (25, 110.0, 4.0)] {
        let mut mixer = mixer();
        mixer.apply(play_with(AudioChannel::Se, &tone(440.0, 1.0), 100, pitch));
        let out = render(&mut mixer, 5.0);
        let sounding = sounding_seconds(&out, RATE);
        assert!((sounding - seconds).abs() < 0.01, "{pitch}: {sounding} s");
        let heard = side(&out[..(RATE as f32 * seconds) as usize * 2], 0);
        let found = frequency(&heard, RATE);
        assert!(
            (found - heard_frequency).abs() < heard_frequency * 0.005,
            "{pitch}: {found} Hz"
        );
    }
}

#[test]
fn stop_silences_its_channel_at_once() {
    let channels = [
        AudioChannel::Bgm,
        AudioChannel::Bgs,
        AudioChannel::Me,
        AudioChannel::Se,
    ];
    for channel in channels {
        let mut mixer = mixer();
        mixer.apply(play(channel, &tone(440.0, 1.0)));
        mixer.apply(play(AudioChannel::Se, &tone(440.0, 1.0)));
        assert!(peak(&render(&mut mixer, 0.1)) > 0.1);
        mixer.apply(AudioCommand::Stop { channel });
        mixer.apply(AudioCommand::Stop {
            channel: AudioChannel::Se,
        });
        assert_eq!(peak(&render(&mut mixer, 0.5)), 0.0, "{channel:?}");
    }
}

/// Scenario "Fade".
#[test]
fn a_fade_lowers_the_volume_evenly_and_then_stops() {
    let mut mixer = mixer();
    mixer.apply(play(AudioChannel::Bgm, &tone(440.0, 0.5)));
    let before = peak(&render(&mut mixer, 0.5));
    mixer.apply(AudioCommand::Fade {
        channel: AudioChannel::Bgm,
        duration_ms: 1000,
    });
    let fading = render(&mut mixer, 1.0);
    // A hundredth of a second, more than four periods of the tone, around each time.
    let around = |seconds: f32| {
        let frame = (RATE as f32 * seconds) as usize;
        peak(&fading[(frame - 220) * 2..(frame + 220) * 2])
    };
    // Even in decibels: the gains of the volumes 75, 50 and 25.
    assert!((around(0.25) / before - 0.365).abs() < 0.02);
    assert!((around(0.5) / before - 0.133).abs() < 0.01);
    assert!((around(0.75) / before - 0.0487).abs() < 0.005);
    assert_eq!(peak(&render(&mut mixer, 1.0)), 0.0, "stopped: no loop");

    // The channel plays again afterwards.
    mixer.apply(play(AudioChannel::Bgm, &tone(440.0, 0.5)));
    assert!((peak(&render(&mut mixer, 0.5)) - before).abs() < 0.01);
}

#[test]
fn unsupported_data_is_reported_once_and_leaves_the_channel_silent() {
    let reports = Reports::default();
    let mut mixer = mixer_reporting_to(&reports);
    mixer.apply(play(AudioChannel::Bgm, &tone(440.0, 0.5)));
    render(&mut mixer, 0.1);

    mixer.apply(play(AudioChannel::Bgm, b"these bytes are no audio file"));
    assert_eq!(peak(&render(&mut mixer, 0.5)), 0.0);
    assert_eq!(reports.lines(), 1, "{}", reports.text());
    assert!(reports.text().contains("supported format"));

    mixer.apply(play(AudioChannel::Se, &[0xff; 64]));
    assert_eq!(peak(&render(&mut mixer, 0.1)), 0.0);
    assert_eq!(reports.lines(), 1, "{}", reports.text());

    // Later commands still work.
    mixer.apply(play(AudioChannel::Bgm, &tone(440.0, 0.5)));
    assert!(peak(&render(&mut mixer, 0.2)) > 0.1);
    mixer.apply(AudioCommand::Stop {
        channel: AudioChannel::Bgm,
    });
    assert_eq!(peak(&render(&mut mixer, 0.1)), 0.0);
}

/// Data that is found to be MP3 and of which no frame decodes.
#[test]
fn damaged_data_is_reported_once_and_leaves_the_channel_silent() {
    let reports = Reports::default();
    let mut mixer = mixer_reporting_to(&reports);
    mixer.apply(play(AudioChannel::Bgm, &tone(440.0, 0.5)));
    render(&mut mixer, 0.1);

    let damaged = damaged_mp3(1);
    mixer.apply(play(AudioChannel::Bgm, &damaged));
    assert_eq!(peak(&render(&mut mixer, 1.5)), 0.0);
    assert_eq!(reports.lines(), 1, "{}", reports.text());
    assert!(reports.text().contains("supported format"));

    // Playing it again, and other data that is no audio, adds nothing.
    mixer.apply(play(AudioChannel::Bgm, &damaged));
    mixer.apply(play(AudioChannel::Se, &damaged));
    mixer.apply(play(AudioChannel::Me, b"these bytes are no audio file"));
    assert_eq!(peak(&render(&mut mixer, 0.5)), 0.0);
    assert_eq!(reports.lines(), 1, "{}", reports.text());

    // Later commands still work.
    mixer.apply(play(AudioChannel::Bgm, &tone(440.0, 0.5)));
    let out = side(&render(&mut mixer, 0.5), 0);
    assert!((level(&out, RATE, 440.0) - AMPLITUDE).abs() < 0.01);
}

/// The data decodes at first and then does not: the failure comes while the mixer renders.
#[test]
fn damage_behind_the_first_block_is_reported_once_while_the_rest_plays() {
    let reports = Reports::default();
    let mut mixer = mixer_reporting_to(&reports);
    let damaged = damaged_mp3(20);
    mixer.apply(play(AudioChannel::Bgm, &damaged));
    assert_eq!(reports.text(), "", "nothing is wrong with the first block");

    // The half that decodes is heard, and loops as music does.
    let out = render(&mut mixer, 3.0);
    assert!(peak(&out[..RATE as usize / 2]) > 0.1);
    assert!(peak(&out[RATE as usize * 4..]) > 0.1);
    assert_eq!(reports.lines(), 1, "{}", reports.text());
    assert!(reports.text().contains("damaged"), "{}", reports.text());

    mixer.apply(play(AudioChannel::Se, &damaged));
    mixer.apply(play(AudioChannel::Me, &damaged_mp3(1)));
    render(&mut mixer, 1.0);
    assert_eq!(reports.lines(), 1, "{}", reports.text());

    // Later commands still work.
    mixer.apply(AudioCommand::Stop {
        channel: AudioChannel::Bgm,
    });
    assert_eq!(peak(&render(&mut mixer, 1.0)), 0.0);
    mixer.apply(play(AudioChannel::Bgm, &tone(440.0, 0.5)));
    let out = side(&render(&mut mixer, 0.5), 0);
    assert!((level(&out, RATE, 440.0) - AMPLITUDE).abs() < 0.01);
}

#[test]
fn midi_without_a_soundfont_is_reported_once_and_silent() {
    let reports = Reports::default();
    let mut mixer = mixer_reporting_to(&reports);
    for _ in 0..2 {
        mixer.apply(play(AudioChannel::Bgm, &one_note_midi(1)));
        assert_eq!(peak(&render(&mut mixer, 0.2)), 0.0);
    }
    assert_eq!(reports.lines(), 1, "{}", reports.text());
    assert!(reports.text().contains("SoundFont"));

    mixer.apply(play(AudioChannel::Se, &tone(440.0, 0.5)));
    assert!(peak(&render(&mut mixer, 0.2)) > 0.1);
}

/// Data that starts as an ASF file does, which is all that is looked at before the program
/// is run.
fn wma() -> Vec<u8> {
    let mut data = vec![
        0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11, 0xa6, 0xd9, 0x00, 0xaa, 0x00, 0x62, 0xce,
        0x6c,
    ];
    data.extend_from_slice(&[0x5a; 100_000]);
    data
}

fn mixer_with_ffmpeg(program: PathBuf, reports: &Reports) -> Mixer {
    let options = MixerOptions {
        soundfonts: SoundFonts::new(Vec::new()),
        errors: Box::new(reports.clone()),
        ffmpeg: program,
    };
    Mixer::new(RATE, options)
}

/// Held by the tests that run a program: a program written while another test starts one
/// may be held open by that one for a moment, and then cannot be run.
static PROGRAMS: Mutex<()> = Mutex::new(());

/// Scenario "Program present". The stand-in reads the data, notes its arguments and writes
/// the file that lies next to it.
#[test]
fn wma_is_decoded_by_the_program_of_the_options() {
    let _programs = PROGRAMS.lock().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("stand-in");
    let script = "#!/bin/sh\ncat > /dev/null\necho \"$@\" > \"$0.arguments\"\ncat \"$0.frames\"\n";
    std::fs::write(&program, script).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    // A ramp that rises on the left and falls on the right.
    let signal: Vec<f32> = (0..2000)
        .flat_map(|frame| [frame as f32 / 4000.0, -(frame as f32) / 4000.0])
        .collect();
    let bytes: Vec<u8> = signal
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    std::fs::write(directory.path().join("stand-in.frames"), bytes).unwrap();

    let reports = Reports::default();
    let mut mixer = mixer_with_ffmpeg(program, &reports);
    mixer.apply(play(AudioChannel::Se, &wma()));
    let mut out = vec![0.0; 2 * 3000];
    mixer.render(&mut out);
    assert_eq!(out[..signal.len()], signal);
    assert_eq!(peak(&out[signal.len()..]), 0.0);
    assert_eq!(reports.text(), "");
    let arguments = std::fs::read_to_string(directory.path().join("stand-in.arguments")).unwrap();
    assert_eq!(
        arguments.trim_end(),
        "-v error -i pipe:0 -f f32le -ac 2 -ar 44100 pipe:1"
    );
}

/// Scenario "Program absent".
#[test]
fn wma_without_the_program_is_reported_and_silent() {
    let _programs = PROGRAMS.lock().unwrap();
    let reports = Reports::default();
    let program = PathBuf::from("/nonexistent/vkrgss-test-ffmpeg");
    let mut mixer = mixer_with_ffmpeg(program, &reports);
    mixer.apply(play(AudioChannel::Bgm, &wma()));
    assert_eq!(peak(&render(&mut mixer, 0.2)), 0.0);
    assert_eq!(reports.lines(), 1, "{}", reports.text());
    assert!(reports.text().contains("ffmpeg"), "{}", reports.text());

    mixer.apply(play(AudioChannel::Bgm, &tone(440.0, 0.5)));
    let out = side(&render(&mut mixer, 0.5), 0);
    assert!((level(&out, RATE, 440.0) - AMPLITUDE).abs() < 0.01);
}

/// What WMA needs is said once, whatever else was reported before as not decodable.
#[test]
fn wma_without_the_program_is_reported_behind_other_undecodable_data() {
    let _programs = PROGRAMS.lock().unwrap();
    let reports = Reports::default();
    let program = PathBuf::from("/nonexistent/vkrgss-test-ffmpeg");
    let mut mixer = mixer_with_ffmpeg(program, &reports);
    mixer.apply(play(AudioChannel::Se, b"these bytes are no audio file"));
    assert_eq!(reports.lines(), 1, "{}", reports.text());
    assert!(!reports.text().contains("ffmpeg"), "{}", reports.text());

    mixer.apply(play(AudioChannel::Se, &wma()));
    assert_eq!(reports.lines(), 2, "{}", reports.text());
    assert!(reports.text().contains("ffmpeg"), "{}", reports.text());

    // Each of the two kinds again adds nothing.
    mixer.apply(play(AudioChannel::Bgm, &wma()));
    mixer.apply(play(AudioChannel::Se, b"these bytes are no audio file"));
    assert_eq!(peak(&render(&mut mixer, 0.2)), 0.0);
    assert_eq!(reports.lines(), 2, "{}", reports.text());

    mixer.apply(play(AudioChannel::Bgm, &tone(440.0, 0.5)));
    let out = side(&render(&mut mixer, 0.5), 0);
    assert!((level(&out, RATE, 440.0) - AMPLITUDE).abs() < 0.01);
}

/// A program that fails, or returns nothing, is as good as none.
#[test]
fn wma_that_the_program_does_not_decode_is_reported_and_silent() {
    for program in ["false", "true"] {
        let _programs = PROGRAMS.lock().unwrap();
        let reports = Reports::default();
        let mut mixer = mixer_with_ffmpeg(PathBuf::from(program), &reports);
        mixer.apply(play(AudioChannel::Se, &wma()));
        assert_eq!(peak(&render(&mut mixer, 0.2)), 0.0, "{program}");
        assert!(reports.text().contains("ffmpeg"), "{}", reports.text());
    }
}

const BGM: AudioChannel = AudioChannel::Bgm;
const BGS: AudioChannel = AudioChannel::Bgs;

fn stop(channel: AudioChannel) -> AudioCommand {
    AudioCommand::Stop { channel }
}

/// The next `frames` frames of the mix, interleaved.
fn frames(mixer: &mut Mixer, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames * 2];
    mixer.render(&mut out);
    out
}

/// A second in which every frame tells where in the file it is.
fn ramp() -> Vec<u8> {
    let ramp: Vec<i16> = (0..RATE).map(|frame| (frame / 2) as i16).collect();
    wav_of(RATE, &ramp)
}

/// Scenarios "Same file" and "Another file".
#[test]
fn a_recalled_position_is_where_the_same_file_starts() {
    let (ramp, other) = (ramp(), tone(440.0, 0.5));
    let mut mixer = mixer();
    mixer.apply(play(BGM, &ramp));
    frames(&mut mixer, 1000);
    mixer.remember(0);
    mixer.apply(stop(BGM));

    // What the file sounds like from its beginning.
    let mut whole = self::mixer();
    whole.apply(play(BGM, &ramp));
    let start = frames(&mut whole, 1000);
    let from_1000 = frames(&mut whole, 1000);
    assert_eq!(from_1000[0], 500.0 / 32768.0, "frame 1000 of the ramp");

    mixer.recall(0);
    mixer.apply(play(BGM, &ramp));
    assert_eq!(frames(&mut mixer, 1000), from_1000);
    mixer.apply(play(BGM, &ramp));
    assert_eq!(frames(&mut mixer, 1000), start, "one command only");

    // Another file, and the remembered one behind it, start at their beginnings.
    mixer.recall(0);
    mixer.apply(play(BGM, &other));
    let mut other_alone = self::mixer();
    other_alone.apply(play(BGM, &other));
    assert_eq!(frames(&mut mixer, 1000), frames(&mut other_alone, 1000));
    mixer.apply(play(BGM, &ramp));
    assert_eq!(frames(&mut mixer, 1000), start);

    // Volume and pitch are those of the command.
    mixer.recall(0);
    mixer.apply(play_with(BGM, &ramp, 0, 100));
    assert_eq!(peak(&frames(&mut mixer, 1000)), 0.0);
}

/// Scenario "Silent channel", and a slot is remembered anew each time.
#[test]
fn a_channel_remembered_silent_starts_at_the_beginning() {
    let ramp = ramp();
    let mut whole = mixer();
    whole.apply(play(BGS, &ramp));
    let start = frames(&mut whole, 1000);
    let from_1000 = frames(&mut whole, 1000);

    let mut mixer = mixer();
    mixer.apply(play(BGM, &ramp));
    frames(&mut mixer, 1000);
    mixer.remember(0);
    mixer.apply(stop(BGM));
    mixer.recall(0);
    mixer.apply(play(BGS, &ramp));
    assert_eq!(frames(&mut mixer, 1000), start, "only the music played");

    // The background sound is at frame 1000 now: remembered, then remembered as stopped.
    mixer.remember(0);
    mixer.apply(stop(BGS));
    mixer.recall(0);
    mixer.apply(play(BGS, &ramp));
    assert_eq!(frames(&mut mixer, 1000), from_1000);
    mixer.apply(stop(BGS));
    mixer.remember(0);
    mixer.recall(0);
    mixer.apply(play(BGS, &ramp));
    assert_eq!(frames(&mut mixer, 1000), start);

    // A voice that a fade ended is silence too, and so is a slot that holds nothing.
    mixer.apply(AudioCommand::Fade {
        channel: BGS,
        duration_ms: 10,
    });
    frames(&mut mixer, 1000);
    mixer.remember(0);
    mixer.recall(0);
    mixer.apply(play(BGS, &ramp));
    assert_eq!(frames(&mut mixer, 1000), start);
    mixer.recall(9);
    mixer.apply(play(BGS, &ramp));
    assert_eq!(frames(&mut mixer, 1000), start);
}

/// Scenario "Same file" with a MIDI file, whose position is counted through its loops.
/// Runs only where one of the system SoundFont locations exists.
#[test]
fn a_recalled_position_is_where_the_same_midi_file_starts() {
    let reports = Reports::default();
    let (Some(mut mixer), Some(mut whole)) = (
        mixer_with_soundfont(&reports),
        mixer_with_soundfont(&reports),
    ) else {
        return;
    };
    let midi = two_note_midi(true);
    // Two and a half seconds: in the second round, which began at the marker.
    let played = RATE as usize * 5 / 2;
    whole.apply(play(BGM, &midi));
    frames(&mut whole, played);
    let from_there = frames(&mut whole, 4000);
    assert!(peak(&from_there) > 0.01);

    mixer.apply(play(BGM, &midi));
    frames(&mut mixer, played);
    mixer.remember(3);
    mixer.apply(stop(BGM));
    mixer.recall(3);
    mixer.apply(play(BGM, &midi));
    assert_eq!(frames(&mut mixer, 4000), from_there);
    assert_eq!(reports.text(), "");
}

/// The value of frame `frame` of [`ramp`].
fn ramp_at(frame: u32) -> f32 {
    (frame / 2) as f32 / 32768.0
}

/// Scenarios "Start", "Start at half the pitch" and "Start of zero".
#[test]
fn a_play_command_starts_at_its_frame() {
    let ramp = ramp();
    for (channel, pitch, start) in [(BGM, 100, 1000), (BGS, 50, 1000), (BGM, 100, 0)] {
        let mut mixer = mixer();
        mixer.apply(play_from(channel, &ramp, (100, pitch), start, 0));
        let first = frames(&mut mixer, 1);
        assert_eq!(first[0], ramp_at(start as u32), "{channel:?} {pitch}");
    }
    // A sound effect starts there too.
    let mut mixer = mixer();
    mixer.apply(play_from(AudioChannel::Se, &ramp, (100, 100), 3000, 0));
    assert_eq!(frames(&mut mixer, 1)[0], ramp_at(3000));
}

/// A file of 2000 frames at `rate` in which frame `n` holds `n + 100`.
fn counted(rate: u32) -> Vec<u8> {
    let samples: Vec<i16> = (0..2000).map(|frame| frame + 100).collect();
    wav_of(rate, &samples)
}

/// The first frame mixed of [`counted`] at `rate`, played as music at `pitch` from `start`,
/// as the number of its frame.
fn first_counted(rate: u32, pitch: u16, start: u64) -> f32 {
    let mut mixer = mixer();
    mixer.apply(play_from(BGM, &counted(rate), (100, pitch), start, 0));
    frames(&mut mixer, 1)[0] * 32768.0 - 100.0
}

/// Scenario "Start" at any pitch: the frame of the file itself, also where an output frame
/// steps over it, at the last frame, and at a ratio of rates that is no whole number.
#[test]
fn a_play_command_starts_at_its_frame_at_any_pitch() {
    assert_eq!(first_counted(RATE, 200, 1001), 1001.0);
    assert_eq!(first_counted(RATE, 200, 1999), 1999.0);
    assert_eq!(first_counted(RATE, 100, 1999), 1999.0);
    assert_eq!(first_counted(RATE, 453, 1000), 1000.0);
    assert_eq!(first_counted(48000, 100, 1001), 1001.0);
    assert_eq!(first_counted(22050, 73, 777), 777.0);
}

/// Scenario "Behind the end".
#[test]
fn a_start_behind_the_end_of_a_looping_sound_begins_it() {
    let short: Vec<i16> = (0..2000).map(|frame| (frame / 2 + 100) as i16).collect();
    let short = wav_of(RATE, &short);
    let mut mixer = mixer();
    mixer.apply(play_from(BGM, &short, (100, 100), 5000, 0));
    assert_eq!(frames(&mut mixer, 1)[0], 100.0 / 32768.0);
}

/// Scenario "A recalled position wins".
#[test]
fn a_recalled_position_wins_over_the_start() {
    let ramp = ramp();
    let mut mixer = mixer();
    mixer.apply(play(BGM, &ramp));
    frames(&mut mixer, 1000);
    mixer.remember(0);
    mixer.apply(stop(BGM));
    mixer.recall(0);
    mixer.apply(play_from(BGM, &ramp, (100, 100), 5, 0));
    assert_eq!(frames(&mut mixer, 1)[0], ramp_at(1000));
}

/// Scenario "MIDI" of the start position. Runs only where one of the system SoundFont
/// locations exists.
#[test]
fn midi_starts_at_its_beginning() {
    let reports = Reports::default();
    let (Some(mut mixer), Some(mut whole)) = (
        mixer_with_soundfont(&reports),
        mixer_with_soundfont(&reports),
    ) else {
        return;
    };
    let midi = one_note_midi(1);
    mixer.apply(play_from(BGM, &midi, (100, 100), 100_000, 0));
    whole.apply(play(BGM, &midi));
    let tenth = RATE as usize / 10;
    let first = frames(&mut mixer, tenth);
    assert!(peak(&first) > 0.01);
    assert_eq!(first, frames(&mut whole, tenth));
    assert_eq!(reports.text(), "");
}

/// Scenario "Lower and go on".
#[test]
fn a_volume_command_lowers_without_a_restart() {
    let ramp = ramp();
    let mut mixer = mixer();
    mixer.apply(play(BGM, &ramp));
    frames(&mut mixer, 1000);
    mixer.apply(AudioCommand::Volume {
        channel: BGM,
        volume: 50,
    });
    let heard = frames(&mut mixer, 1)[0] / ramp_at(1000);
    assert!((heard - 0.133).abs() < 0.001, "{heard}");
}

/// Scenario "Nothing plays".
#[test]
fn a_volume_command_without_a_voice_does_nothing() {
    let sound = tone(440.0, 0.5);
    let mut mixer = mixer();
    mixer.apply(AudioCommand::Volume {
        channel: BGS,
        volume: 50,
    });
    mixer.apply(play(BGS, &sound));
    let mut alone = self::mixer();
    alone.apply(play(BGS, &sound));
    assert_eq!(render(&mut mixer, 0.2), render(&mut alone, 0.2));
}

/// The peak of the music around `seconds` of `out`, a hundredth of a second, more than four
/// periods of a tone of 440 Hz.
fn around(out: &[f32], seconds: f32) -> f32 {
    let frame = (RATE as f32 * seconds) as usize;
    peak(&out[(frame - 220) * 2..(frame + 220) * 2])
}

/// A tone of 440 Hz as music at `volume` and its peak before an effect of 1000 Hz and half
/// a second is played with the return time 1000 ms; then, if `stop_after` is given, the
/// effect is stopped after that many seconds. Returns the peak before the effect, the
/// mix while the effect sounds, and the two seconds after the end of the effect.
fn music_after_an_effect(volume: u8, stop_after: Option<f32>) -> (f32, Vec<f32>, Vec<f32>) {
    let mut mixer = mixer();
    mixer.apply(play_with(BGM, &tone(440.0, 0.5), volume, 100));
    let before = peak(&render(&mut mixer, 0.5));
    mixer.apply(play_from(
        AudioChannel::Me,
        &tone(1000.0, 0.5),
        (100, 100),
        0,
        1000,
    ));
    let during = match stop_after {
        Some(seconds) => {
            let during = render(&mut mixer, seconds);
            mixer.apply(stop(AudioChannel::Me));
            during
        }
        None => render(&mut mixer, 0.5),
    };
    (before, during, render(&mut mixer, 2.0))
}

/// Scenarios "One second, measured on the original" and "A stopped effect".
#[test]
fn the_music_returns_over_one_second() {
    for stop_after in [None, Some(0.1)] {
        let (before, during, after) = music_after_an_effect(100, stop_after);
        assert!(level(&side(&during, 0), RATE, 440.0) < 0.001);
        let half = around(&after, 0.5) / before;
        assert!((half - 0.133).abs() < 0.01, "{stop_after:?}: {half}");
        let full = peak(&after[RATE as usize * 2..]) / before;
        assert!((full - 1.0).abs() < 0.001, "{stop_after:?}: {full}");
        assert!(peak(&after[..RATE as usize / 5]) / before < 0.05);
    }
}

/// Scenario "A music at a lower volume".
#[test]
fn the_music_returns_to_its_own_volume() {
    let (before, _, after) = music_after_an_effect(50, None);
    // The gains of the volumes 25 and 50 against that of 50.
    let half = around(&after, 0.5) / before;
    assert!((half - 0.0487 / 0.133).abs() < 0.02, "{half}");
    let full = peak(&after[RATE as usize * 2..]) / before;
    assert!((full - 1.0).abs() < 0.001, "{full}");
}

/// Scenario "No return time" after an effect whose music was still returning: the music is
/// at its own amplitude at once, whether the second effect ends by itself or by a stop.
#[test]
fn no_return_time_ends_a_return_that_was_not_over() {
    let music = wav_of(RATE, &vec![8192; RATE as usize]);
    let effect = wav_of(RATE, &vec![0; RATE as usize / 10]);
    for stopped in [false, true] {
        let mut mixer = mixer();
        mixer.apply(play(BGM, &music));
        mixer.apply(play_from(AudioChannel::Me, &effect, (100, 100), 0, 1000));
        // The effect, then a quarter of the return.
        frames(&mut mixer, RATE as usize / 10 + RATE as usize / 4);
        mixer.apply(play_from(AudioChannel::Me, &effect, (100, 100), 0, 0));
        let after = if stopped {
            frames(&mut mixer, 100);
            mixer.apply(stop(AudioChannel::Me));
            frames(&mut mixer, 1)
        } else {
            let out = frames(&mut mixer, RATE as usize / 10 + 1);
            out[out.len() - 2..].to_vec()
        };
        assert_eq!(after, [0.25, 0.25], "stopped: {stopped}");
    }
}

/// Scenario "A stop without an effect".
#[test]
fn a_stop_of_the_effect_channel_without_an_effect_changes_nothing() {
    let music = tone(440.0, 0.5);
    let mut mixer = mixer();
    mixer.apply(play(BGM, &music));
    let mut alone = self::mixer();
    alone.apply(play(BGM, &music));
    assert_eq!(render(&mut mixer, 0.2), render(&mut alone, 0.2));
    mixer.apply(stop(AudioChannel::Me));
    assert_eq!(render(&mut mixer, 0.5), render(&mut alone, 0.5));
}
