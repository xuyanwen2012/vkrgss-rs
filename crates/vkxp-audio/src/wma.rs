//! WMA: decoded as a whole by the `ffmpeg` program, which is run for each file.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

use crate::decode::{DecodeError, Source};

/// The identifier of the header object that an ASF file, which is what holds WMA, starts
/// with.
const ASF_HEADER: [u8; 16] = [
    0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11, 0xa6, 0xd9, 0x00, 0xaa, 0x00, 0x62, 0xce, 0x6c,
];

/// Whether `data` is an ASF file.
pub(crate) fn is_asf(data: &[u8]) -> bool {
    data.starts_with(&ASF_HEADER)
}

/// A sound that was decoded as a whole: interleaved stereo frames at the rate asked for.
pub(crate) struct Decoded {
    frames: Vec<f32>,
    /// The index in `frames` of the next frame.
    next: usize,
}

impl Decoded {
    /// Has `program`, which is `ffmpeg` or stands in for it, decode `data` to frames at
    /// `sample_rate`. The program is not run by a shell and its arguments are fixed.
    pub(crate) fn by_ffmpeg(
        data: &[u8],
        sample_rate: u32,
        program: &Path,
    ) -> Result<Decoded, DecodeError> {
        let failed = |reason: String| {
            DecodeError::Unsupported(format!("WMA needs the ffmpeg program: {reason}"))
        };
        let mut child = Command::new(program)
            .args([
                "-v", "error", "-i", "pipe:0", "-f", "f32le", "-ac", "2", "-ar",
            ])
            .arg(sample_rate.to_string())
            .arg("pipe:1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| failed(format!("{} cannot be started: {error}", program.display())))?;
        let mut input = child.stdin.take();
        let output = thread::scope(|scope| {
            // The data is written while the frames are read, or both pipes would fill. A
            // program that does not read all of it shows in what it returns.
            scope.spawn(move || input.take().map(|mut input| input.write_all(data)));
            child.wait_with_output()
        })
        .map_err(|error| failed(error.to_string()))?;
        if !output.status.success() {
            return Err(failed(format!("it ended with {}", output.status)));
        }
        let frames: Vec<f32> = output
            .stdout
            .as_chunks::<8>()
            .0
            .iter()
            .flat_map(|frame| {
                let (left, right) = frame.split_at(4);
                [left, right]
                    .map(|sample| f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]))
            })
            .collect();
        if frames.is_empty() {
            return Err(failed("it returned no sound".to_owned()));
        }
        Ok(Decoded { frames, next: 0 })
    }
}

impl Source for Decoded {
    fn read(&mut self, out: &mut [f32]) -> usize {
        let rest = &self.frames[self.next * 2..];
        let frames = (out.len() / 2).min(rest.len() / 2);
        out[..frames * 2].copy_from_slice(&rest[..frames * 2]);
        self.next += frames;
        frames
    }

    fn rewind(&mut self) {
        self.next = 0;
    }

    fn position(&self) -> u64 {
        self.next as u64
    }

    fn start_from(&mut self, position: u64) {
        self.next = (self.frames.len() / 2).min(position.try_into().unwrap_or(usize::MAX));
    }
}
