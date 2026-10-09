//! The demo guest without a window: scripted input, frame dumps, no real time.
//!
//! ```sh
//! cargo run -p vkxp-host --example demo-headless -- --frames 30 --dump 10=target/frame.png
//! ```
//!
//! Options:
//!
//! - `--frames N`: stop after N frames. Required, because the demo runs until it is told to
//!   quit.
//! - `--dump N=PATH`: write frame N (counted from 0) as a PNG file. May be given several times.
//! - `--input SCRIPT`: the buttons held from given frames on, for example
//!   `"5:RIGHT, 12:RIGHT+SHIFT, 20:"`.
//! - `--device DEVICE`: index of a Vulkan device or a part of its name. Defaults to llvmpipe.

use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;

use vkxp_host::demo::{self, DemoOptions};
use vkxp_host::{HeadlessOptions, InputScript, device_from_text, headless_link, run_headless};

const USAGE: &str = "usage: demo-headless --frames N [--dump N=PATH]... [--input SCRIPT] \
                     [--device DEVICE]";

fn parse(arguments: &[String]) -> Result<HeadlessOptions, String> {
    let mut options = HeadlessOptions::default();
    let mut arguments = arguments.iter();
    while let Some(option) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{option} needs a value"))
        };
        match option.as_str() {
            "--frames" => {
                let text = value()?;
                let frames = text
                    .parse()
                    .map_err(|_| format!("`{text}` is not a number of frames"))?;
                options.frames = Some(frames);
            }
            "--dump" => {
                let text = value()?;
                let dump = text
                    .split_once('=')
                    .filter(|(_, path)| !path.is_empty())
                    .and_then(|(frame, path)| Some((frame.parse().ok()?, PathBuf::from(path))));
                options
                    .dumps
                    .push(dump.ok_or_else(|| format!("`{text}` is not of the form N=PATH"))?);
            }
            "--input" => {
                options.script = InputScript::parse(value()?).map_err(|error| error.to_string())?;
            }
            "--device" => options.device = device_from_text(value()?),
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    if options.frames.is_none() {
        return Err("--frames is required".to_owned());
    }
    Ok(options)
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse(&arguments) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("demo-headless: {message}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let dumps: Vec<PathBuf> = options.dumps.iter().map(|(_, path)| path.clone()).collect();

    let (guest_end, host) = headless_link();
    let guest = thread::spawn(move || demo::run(&guest_end, DemoOptions::default()));
    let result = run_headless(options, host);
    // The run has dropped its end of the link, so the demo ends whatever state it was in.
    let _ = guest.join();

    match result {
        Ok(report) => {
            println!(
                "{} frame(s) in {} ms of virtual time",
                report.frames,
                report.elapsed.as_millis()
            );
            for path in dumps {
                println!("wrote {}", path.display());
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("demo-headless: {error}");
            ExitCode::FAILURE
        }
    }
}
