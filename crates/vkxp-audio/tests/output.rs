//! The output without a device: the device-opening function is the test's own, so nothing
//! here opens an audio device or makes a sound.

mod common;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{Reports, peak, tone_wav};
use vkxp_audio::{Output, OutputOptions, Render, SoundFonts};
use vkxp_proto::{AudioChannel, AudioCommand};

fn options(reports: &Reports) -> OutputOptions {
    OutputOptions {
        soundfonts: SoundFonts::new(Vec::new()),
        errors: Box::new(reports.clone()),
    }
}

fn play(data: &[u8]) -> AudioCommand {
    AudioCommand::Play {
        channel: AudioChannel::Bgm,
        data: data.to_vec(),
        volume: 100,
        pitch: 100,
    }
}

#[test]
fn without_a_device_commands_are_accepted_and_one_report_is_made() {
    let reports = Reports::default();
    let output = Output::open_with(options(&reports), |_start| Err("no such device".to_owned()));
    assert_eq!(reports.lines(), 1, "{}", reports.text());
    assert!(reports.text().contains("no such device"));

    output.command(play(&tone_wav(44100, 440.0, 0.5)));
    output.command(play(b"no audio file"));
    output.command(AudioCommand::Fade {
        channel: AudioChannel::Bgm,
        duration_ms: 100,
    });
    output.command(AudioCommand::Stop {
        channel: AudioChannel::Bgm,
    });
    assert_eq!(reports.lines(), 1, "{}", reports.text());
}

/// The device here is a function the test calls itself.
#[test]
fn commands_reach_the_mixer_behind_the_device_callback() {
    let reports = Reports::default();
    let device: Arc<Mutex<Option<Render>>> = Arc::default();
    let slot = device.clone();
    let output = Output::open_with(options(&reports), move |start| {
        *slot.lock().unwrap() = Some(start(44100));
        Ok(Box::new(()))
    });
    let mut render = device.lock().unwrap().take().unwrap();
    let mut block = [1.0; 2 * 512];
    render(&mut block);
    assert_eq!(peak(&block), 0.0, "silence before any command");

    // The worker thread opens the sound; the callback hears it a little later.
    let heard = |render: &mut Render| {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut block = [0.0; 2 * 512];
        while Instant::now() < deadline {
            render(&mut block);
            if peak(&block) > 0.1 {
                return true;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        false
    };
    output.command(play(&tone_wav(44100, 440.0, 0.5)));
    assert!(heard(&mut render));

    // Commands keep their order: the bad data silences the channel, the tone follows it.
    output.command(play(b"no audio file"));
    output.command(play(&tone_wav(44100, 440.0, 0.5)));
    assert!(heard(&mut render));
    assert_eq!(reports.lines(), 1, "{}", reports.text());
    assert!(reports.text().contains("supported format"));

    drop(output);
}
