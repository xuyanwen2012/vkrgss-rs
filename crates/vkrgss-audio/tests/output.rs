//! The output without a device: the device-opening function is the test's own, so nothing
//! here opens an audio device or makes a sound.

mod common;

use std::io::Write;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use common::{AMPLITUDE, Reports, damaged_mp3, level, peak, side, tone_wav};
use vkrgss_audio::{Output, OutputOptions, Render, SoundFonts};
use vkrgss_proto::{AudioChannel, AudioCommand};

fn options(reports: &Reports) -> OutputOptions {
    OutputOptions {
        soundfonts: SoundFonts::new(Vec::new()),
        errors: Box::new(reports.clone()),
        ..OutputOptions::default()
    }
}

fn play(data: &[u8]) -> AudioCommand {
    AudioCommand::Play {
        channel: AudioChannel::Bgm,
        data: data.to_vec(),
        volume: 100,
        pitch: 100,
        start: 0,
        music_return_ms: 0,
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

/// Calls the device callback, a twentieth of a second of sound at a time, until `reached`
/// says that the block shows the state waited for. The worker thread takes its time to open
/// a sound, so the state is waited for, up to a deadline, and never assumed after a delay.
fn render_until(render: &mut Render, what: &str, reached: impl Fn(&[f32]) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut block = [0.0; 2 * 2205];
    loop {
        render(&mut block);
        if reached(&side(&block, 0)) {
            return;
        }
        assert!(Instant::now() < deadline, "never reached: {what}");
        std::thread::yield_now();
    }
}

/// The device here is a function the test calls itself.
#[test]
fn commands_reach_the_mixer_behind_the_device_callback() {
    const RATE: u32 = 44100;
    let tone_of = |frequency: f32| {
        move |block: &[f32]| (level(block, RATE, frequency) - AMPLITUDE).abs() < 0.01
    };

    let reports = Reports::default();
    let device: Arc<Mutex<Option<Render>>> = Arc::default();
    let slot = device.clone();
    let output = Output::open_with(options(&reports), move |start| {
        *slot.lock().unwrap() = Some(start(RATE));
        Ok(Box::new(()))
    });
    let mut render = device.lock().unwrap().take().unwrap();
    let mut block = [1.0; 2 * 512];
    render(&mut block);
    assert_eq!(peak(&block), 0.0, "silence before any command");

    output.command(play(&tone_wav(RATE, 440.0, 0.5)));
    render_until(&mut render, "the first tone", tone_of(440.0));

    // The music loops, so only the stop command can have silenced it.
    output.command(AudioCommand::Stop {
        channel: AudioChannel::Bgm,
    });
    render_until(&mut render, "silence after the stop", |block| {
        peak(block) == 0.0
    });
    assert_eq!(reports.text(), "");

    // Commands keep their order: the data that is no audio is reported before the tone that
    // was sent behind it is heard, and that tone is not the first one.
    output.command(play(b"no audio file"));
    output.command(play(&tone_wav(RATE, 1000.0, 0.5)));
    render_until(&mut render, "the second tone", tone_of(1000.0));
    assert_eq!(reports.lines(), 1, "{}", reports.text());
    assert!(reports.text().contains("supported format"));

    drop(output);
}

/// A paused output gives silence and does not mix: what plays goes on where it stopped.
#[test]
fn a_paused_output_is_silent_and_goes_on_where_it_stopped() {
    const RATE: u32 = 44100;
    let reports = Reports::default();
    let device: Arc<Mutex<Option<Render>>> = Arc::default();
    let slot = device.clone();
    let output = Output::open_with(options(&reports), move |start| {
        *slot.lock().unwrap() = Some(start(RATE));
        Ok(Box::new(()))
    });
    let mut render = device.lock().unwrap().take().unwrap();
    // A sound effect of a fifth of a second, which does not loop.
    output.command(AudioCommand::Play {
        channel: AudioChannel::Se,
        data: tone_wav(RATE, 440.0, 0.2),
        volume: 100,
        pitch: 100,
        start: 0,
        music_return_ms: 0,
    });
    render_until(&mut render, "the tone", |block| peak(block) > 0.0);

    output.set_paused(true);
    let mut block = [1.0; 2 * 2205];
    for _ in 0..20 {
        render(&mut block);
        assert_eq!(peak(&block), 0.0, "silence while paused");
    }
    // A second of calls while paused has not used up the tone.
    output.set_paused(false);
    render(&mut block);
    assert!(peak(&block) > 0.0, "the tone goes on where it stopped");
    assert_eq!(reports.text(), "");
}

/// An error stream that cannot be written while the test holds its gate.
#[derive(Clone)]
struct Gated {
    gate: Arc<Mutex<()>>,
    reports: Reports,
}

impl Write for Gated {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let _open = self.gate.lock().unwrap();
        self.reports.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Scenario "Error stream held".
#[test]
fn callback_reports_damage_without_the_error_lock() {
    const RATE: u32 = 44100;
    let reports = Reports::default();
    let gate = Arc::new(Mutex::new(()));
    let held = gate.lock().unwrap();
    let options = OutputOptions {
        errors: Box::new(Gated {
            gate: gate.clone(),
            reports: reports.clone(),
        }),
        ..options(&reports)
    };
    let device: Arc<Mutex<Option<Render>>> = Arc::default();
    let slot = device.clone();
    let output = Output::open_with(options, move |start| {
        *slot.lock().unwrap() = Some(start(RATE));
        Ok(Box::new(()))
    });
    let mut render = device.lock().unwrap().take().unwrap();

    // The first block decodes; half a second in, the data does not, while the music loops.
    output.command(play(&damaged_mp3(20)));
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        render_until(&mut render, "the music", |block| peak(block) > 0.1);
        let mut block = [0.0; 2 * 2205];
        for _ in 0..60 {
            render(&mut block);
        }
        done.send(()).unwrap();
    });
    finished
        .recv_timeout(Duration::from_secs(60))
        .expect("the callback returns for every buffer while the error stream blocks");
    assert_eq!(reports.text(), "");

    drop(held);
    let deadline = Instant::now() + Duration::from_secs(30);
    while !reports.text().contains("damaged") {
        assert!(Instant::now() < deadline, "the damage is never reported");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(reports.lines(), 1, "{}", reports.text());
}
