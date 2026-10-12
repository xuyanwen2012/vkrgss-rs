//! The scenarios of `specs/audio-playback`, rendered into buffers.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use common::{
    AMPLITUDE, FIRST_NOTE, MP2, MP3, OGG, Reports, SECOND_NOTE, damaged_mp3, frequency, level,
    one_note_midi, peak, side, sounding_seconds, tone_wav, two_note_midi, wav_of,
};
use vkxp_audio::{Mixer, MixerOptions, SYSTEM_SOUNDFONTS, SoundFonts};
use vkxp_proto::{AudioChannel, AudioCommand};

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
    let program = PathBuf::from("/nonexistent/vkxp-test-ffmpeg");
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
    let program = PathBuf::from("/nonexistent/vkxp-test-ffmpeg");
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
