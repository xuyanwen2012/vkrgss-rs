//! Headless proof of the guest/host contract: encoded messages in, PNG out.
//!
//! ```sh
//! cargo run -p vkrgss-compose-ref --example render_frame -- <messages> <output.png>
//! ```
//!
//! `<messages>` is an encoded stream of `GuestToHost` messages (see `vkrgss_proto::codec`). The
//! messages go through the CPU reference compositor in order, and the last frame they produce
//! is written as PNG. No window, Ruby or GPU is involved.

use std::error::Error;
use std::process::ExitCode;

use vkrgss_compose_ref::{encode_png, render_stream};
use vkrgss_proto::GuestToHost;
use vkrgss_proto::codec::decode_stream;

fn run(input: &str, output: &str) -> Result<(), Box<dyn Error>> {
    let bytes = std::fs::read(input).map_err(|error| format!("cannot read `{input}`: {error}"))?;
    let messages: Vec<GuestToHost> = decode_stream(&bytes)?;
    let frame = render_stream(&messages)?
        .ok_or_else(|| format!("`{input}` holds {} messages but no frame", messages.len()))?;
    std::fs::write(output, encode_png(&frame)?)
        .map_err(|error| format!("cannot write `{output}`: {error}"))?;
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output] = args.as_slice() else {
        eprintln!("usage: render_frame <messages> <output.png>");
        return ExitCode::from(2);
    };
    match run(input, output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("render_frame: {error}");
            ExitCode::FAILURE
        }
    }
}
