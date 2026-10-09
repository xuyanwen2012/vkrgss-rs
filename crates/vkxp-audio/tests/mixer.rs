//! The scenarios of `specs/audio-playback`, rendered into buffers.

mod common;

use std::path::Path;

use common::{
    AMPLITUDE, MP3, OGG, Reports, damaged_mp3, frequency, level, one_note_midi, peak, side,
    sounding_seconds, tone_wav, wav_of,
};
use vkxp_audio::{Mixer, MixerOptions, SYSTEM_SOUNDFONTS, SoundFonts};
use vkxp_proto::{AudioChannel, AudioCommand};

const RATE: u32 = 44100;

fn mixer_reporting_to(reports: &Reports) -> Mixer {
    let options = MixerOptions {
        soundfonts: SoundFonts::new(Vec::new()),
        errors: Box::new(reports.clone()),
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
fn the_ogg_and_mp3_fixtures_are_heard_for_their_second() {
    for (name, data) in [("ogg", OGG), ("mp3", MP3)] {
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

/// Runs only where one of the system SoundFont locations exists.
#[test]
fn a_midi_note_is_heard_while_it_sounds() {
    if !SYSTEM_SOUNDFONTS
        .iter()
        .any(|path| Path::new(path).is_file())
    {
        eprintln!("skipped: no system SoundFont");
        return;
    }
    let reports = Reports::default();
    let options = MixerOptions {
        soundfonts: SoundFonts::configured(None),
        errors: Box::new(reports.clone()),
    };
    let mut mixer = Mixer::new(RATE, options);
    mixer.apply(play(AudioChannel::Bgm, &one_note_midi(1)));
    let out = render(&mut mixer, 1.0);
    for (index, tenth) in out.chunks(RATE as usize / 5).enumerate() {
        assert!(peak(tenth) > 0.01, "silent in tenth {index}");
    }
    assert_eq!(reports.text(), "");
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

#[test]
fn volume_scales_the_amplitude() {
    for (volume, expected) in [(0, 0.0), (50, AMPLITUDE / 2.0), (100, AMPLITUDE)] {
        let mut mixer = mixer();
        mixer.apply(play_with(AudioChannel::Bgm, &tone(440.0, 0.5), volume, 100));
        let out = side(&render(&mut mixer, 0.5), 0);
        assert!(
            (level(&out, RATE, 440.0) - expected).abs() < 0.005,
            "{volume}"
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
    assert!((around(0.25) / before - 0.75).abs() < 0.02);
    assert!((around(0.5) / before - 0.5).abs() < 0.02);
    assert!((around(0.75) / before - 0.25).abs() < 0.02);
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
