//! The output: a mixer behind the callback of an audio device.
//!
//! Commands go through a channel to a worker thread, which opens the sources (probing the
//! file, loading the SoundFont, decoding the first block) and passes the prepared commands on
//! to the callback. The callback applies them and mixes; it never waits for anything.

use std::any::Any;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use vkrgss_proto::AudioCommand;

use crate::mixer::{Mixer, MixerOptions};

/// How an output is set up; the same as a mixer.
pub type OutputOptions = MixerOptions;

/// Fills a buffer of interleaved stereo frames with the next frames of the mix. This is what
/// the callback of a device calls.
pub type Render = Box<dyn FnMut(&mut [f32]) + Send>;

/// Given the sample rate of the device, starts the mixer and returns its [`Render`].
pub type Start = Box<dyn FnOnce(u32) -> Render>;

/// An error stream shared by the worker thread and the thread that opens the device.
#[derive(Clone)]
struct Errors(Arc<Mutex<Box<dyn Write + Send>>>);

impl Write for Errors {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let mut errors = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        errors.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .flush()
    }
}

/// What the worker thread is asked for, in the order it is asked.
enum Request {
    Command(AudioCommand),
    Remember(u8),
    Recall(u8),
}

/// Sound on an audio device, or nothing where no device could be opened.
pub struct Output {
    /// The way to the worker thread; `None` without a device.
    commands: Option<Sender<Request>>,
    /// The open device. Dropping it ends the sound.
    device: Option<Box<dyn Any>>,
    /// Whether the callback gives silence without mixing.
    paused: Arc<AtomicBool>,
}

impl Output {
    /// Opens the default output device. Failing that, the failure is reported once and the
    /// output accepts and ignores every command.
    pub fn open(options: OutputOptions) -> Output {
        Output::open_with(options, open_default_device)
    }

    /// Like [`open`](Output::open) with another way to open a device. `open_device` calls
    /// the [`Start`] it is given with the sample rate of its device, has the device call the
    /// [`Render`] it gets back, and returns what keeps the device open. Its error says why
    /// there is no device.
    pub fn open_with(
        options: OutputOptions,
        open_device: impl FnOnce(Start) -> Result<Box<dyn Any>, String>,
    ) -> Output {
        let mut errors = Errors(Arc::new(Mutex::new(options.errors)));
        let soundfonts = options.soundfonts;
        let ffmpeg = options.ffmpeg;
        let (commands, received) = mpsc::channel::<Request>();
        let worker_errors = errors.clone();
        let paused = Arc::new(AtomicBool::new(false));
        let callback_paused = Arc::clone(&paused);
        let start: Start = Box::new(move |sample_rate| {
            let mut errors = worker_errors.clone();
            let options = MixerOptions {
                soundfonts,
                errors: Box::new(worker_errors),
                ffmpeg,
            };
            let (mut loader, mut channels) = Mixer::new(sample_rate, options).into_parts();
            let (prepared, ready) = mpsc::channel();
            let worker = thread::Builder::new()
                .name("vkrgss-audio".to_owned())
                .spawn(move || {
                    for request in received {
                        match request {
                            Request::Command(command) => {
                                if prepared.send(loader.prepare(command)).is_err() {
                                    break;
                                }
                            }
                            Request::Remember(slot) => loader.remember(slot),
                            Request::Recall(slot) => loader.recall(slot),
                        }
                    }
                });
            if let Err(error) = worker {
                // Without the thread no command arrives, and the device plays silence.
                let _ = writeln!(
                    errors,
                    "vkrgss: audio: cannot start the audio thread: {error}; the game runs \
                     without sound"
                );
            }
            Box::new(move |out: &mut [f32]| {
                if callback_paused.load(Ordering::Relaxed) {
                    out.fill(0.0);
                    return;
                }
                while let Ok(command) = ready.try_recv() {
                    channels.apply(command);
                }
                channels.render(out);
            })
        });
        match open_device(start) {
            Ok(device) => Output {
                commands: Some(commands),
                device: Some(device),
                paused,
            },
            Err(reason) => {
                // Nothing useful can be done when the error stream itself fails.
                let _ = writeln!(
                    errors,
                    "vkrgss: audio: cannot open an output device: {reason}; the game runs \
                     without sound"
                );
                Output {
                    commands: None,
                    device: None,
                    paused,
                }
            }
        }
    }

    /// Hands a command to the audio side without waiting for it.
    pub fn command(&self, command: AudioCommand) {
        self.request(Request::Command(command));
    }

    /// Like [`Mixer::remember`], without waiting for it. The positions are those the
    /// device callback last mixed up to.
    pub fn remember(&self, slot: u8) {
        self.request(Request::Remember(slot));
    }

    /// Like [`Mixer::recall`], without waiting for it.
    pub fn recall(&self, slot: u8) {
        self.request(Request::Recall(slot));
    }

    /// Pauses the output, with `paused`, or lets it play again. Commands are still taken
    /// meanwhile and apply when it plays.
    pub fn set_paused(&self, paused: bool) {
        if self.paused.swap(paused, Ordering::Relaxed) == paused {
            return;
        }
        let Some(device) = &self.device else {
            return;
        };
        // A device of `cpal` stops asking for frames; another one gets silence.
        if let Some(stream) = device.downcast_ref::<cpal::Stream>() {
            let changed = match paused {
                true => stream.pause(),
                false => stream.play(),
            };
            if let Err(error) = changed {
                tracing::error!("audio: the output stream cannot be paused or played: {error}");
            }
        }
        match paused {
            true => tracing::info!("audio: the output is paused"),
            false => tracing::info!("audio: the output plays again"),
        }
    }

    fn request(&self, request: Request) {
        if let Some(commands) = &self.commands {
            // The worker thread only ends with the output.
            let _ = commands.send(request);
        }
    }
}

/// Opens the default output device of the default host of `cpal`.
fn open_default_device(start: Start) -> Result<Box<dyn Any>, String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("there is no default output device")?;
    let config = device
        .default_output_config()
        .map_err(|error| error.to_string())?
        .config();
    let channels = usize::from(config.channels);
    if channels == 0 {
        return Err("the default output device has no channels".to_owned());
    }
    let mut render = start(config.sample_rate);
    let mut stereo = Vec::new();
    let mut failed = false;
    let stream = device
        .build_output_stream(
            config,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                stereo.resize(out.len() / channels * 2, 0.0);
                render(&mut stereo);
                for (frame, pair) in out
                    .chunks_exact_mut(channels)
                    .zip(stereo.as_chunks::<2>().0)
                {
                    match frame {
                        [mono] => *mono = (pair[0] + pair[1]) / 2.0,
                        [left, right, others @ ..] => {
                            *left = pair[0];
                            *right = pair[1];
                            others.fill(0.0);
                        }
                        [] => {}
                    }
                }
            },
            move |error| {
                if !std::mem::replace(&mut failed, true) {
                    tracing::error!("audio: the output stream reported an error: {error}");
                }
            },
            None,
        )
        .map_err(|error| error.to_string())?;
    stream.play().map_err(|error| error.to_string())?;
    Ok(Box::new(stream))
}
