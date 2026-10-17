//! Where the app's output goes: an app's standard output and error stream lead nowhere on
//! Android, so both are made the write end of a pipe, and a thread writes every line that
//! comes out of it to logcat, under the tag `vkrgss`, and to a file. The diagnostics, the
//! reports of the host and the script errors of the guest all write to the error stream, so
//! all of them arrive in both places.

use std::ffi::{CString, c_char, c_int};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::path::Path;
use std::thread::{self, JoinHandle};

/// The tag of the app's lines in logcat.
const TAG: &std::ffi::CStr = c"vkrgss";

/// `ANDROID_LOG_INFO` of `<android/log.h>`.
const LOG_INFO: c_int = 4;

#[link(name = "log")]
unsafe extern "C" {
    fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

/// The thread that forwards the lines.
pub(crate) struct Forwarding {
    thread: JoinHandle<()>,
}

/// Makes the standard output and the error stream lead to logcat and to the file `path`,
/// which is written afresh. Without the file, or when it cannot be created, the lines go to
/// logcat alone.
pub(crate) fn start(path: Option<&Path>) -> io::Result<Forwarding> {
    let file = path.and_then(|path| File::create(path).ok());
    let mut ends: [RawFd; 2] = [0; 2];
    // SAFETY: `ends` has room for the two descriptors `pipe` writes.
    if unsafe { libc::pipe(ends.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let [read, write] = ends;
    for stream in [libc::STDOUT_FILENO, libc::STDERR_FILENO] {
        // SAFETY: both descriptors are open; `dup2` replaces the stream's descriptor.
        if unsafe { libc::dup2(write, stream) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    // SAFETY: the write end is open, and the two streams hold copies of it now.
    unsafe { libc::close(write) };
    // SAFETY: the read end is open and owned by nothing else; the file takes it over.
    let reader = unsafe { File::from_raw_fd(read) };
    let thread = thread::Builder::new()
        .name("vkrgss-log".to_owned())
        .spawn(move || forward(reader, file))?;
    Ok(Forwarding { thread })
}

impl Forwarding {
    /// Waits until every line written so far has been forwarded. The standard output and the
    /// error stream lead nowhere from here on.
    pub(crate) fn finish(self) {
        let _ = io::stdout().flush();
        // SAFETY: the path is a NUL-terminated string; `open` returns a new descriptor or -1.
        let null = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_WRONLY) };
        if null < 0 {
            return;
        }
        for stream in [libc::STDOUT_FILENO, libc::STDERR_FILENO] {
            // SAFETY: both descriptors are open; the streams' copies of the write end close,
            // which ends the pipe once they were the last.
            unsafe { libc::dup2(null, stream) };
        }
        // SAFETY: the descriptor was opened above and the streams hold copies of it.
        unsafe { libc::close(null) };
        let _ = self.thread.join();
    }
}

/// Writes every line of `reader` to logcat and to `file`, until the pipe ends.
fn forward(reader: File, mut file: Option<File>) {
    for line in BufReader::new(reader).split(b'\n') {
        let Ok(line) = line else {
            break;
        };
        if let Some(file) = &mut file {
            let _ = file.write_all(&line).and_then(|()| file.write_all(b"\n"));
        }
        let text = CString::new(
            line.into_iter()
                .filter(|byte| *byte != 0)
                .collect::<Vec<u8>>(),
        )
        .expect("the NUL bytes were left out");
        // SAFETY: both strings are NUL-terminated and live for the call.
        unsafe { __android_log_write(LOG_INFO, TAG.as_ptr(), text.as_ptr()) };
    }
}
