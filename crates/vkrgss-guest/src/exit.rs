//! The end of the process of a script, without `exit`.

/// Ends the process of a script with `status` once its interpreter is finalised, without
/// `exit`. `exit` flushes every stream of the C library and takes the lock of each, and
/// Ruby 1.8.7 leaves the lock of the standard input held by its thread when an `Interrupt`
/// is raised in a read of it, so `exit` would wait for it for ever. The standard output and
/// error streams, the only ones a script writes through the C library once Ruby has closed
/// its files, are flushed on their own first, and so are those of Rust.
pub fn end_script(status: u8) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    // SAFETY: the two streams are the C library's own, valid for the life of the process;
    // `fflush` takes only their locks, and `_exit` ends the process without running
    // handlers or flushing anything else.
    unsafe {
        libc::fflush(c_stream::stdout());
        libc::fflush(c_stream::stderr());
        libc::_exit(libc::c_int::from(status))
    }
}

/// The standard output and error streams of the C library, whose names differ.
mod c_stream {
    #[cfg(not(target_vendor = "apple"))]
    unsafe extern "C" {
        #[link_name = "stdout"]
        static STDOUT: *mut libc::FILE;
        #[link_name = "stderr"]
        static STDERR: *mut libc::FILE;
    }
    #[cfg(target_vendor = "apple")]
    unsafe extern "C" {
        #[link_name = "__stdoutp"]
        static STDOUT: *mut libc::FILE;
        #[link_name = "__stderrp"]
        static STDERR: *mut libc::FILE;
    }

    pub(super) fn stdout() -> *mut libc::FILE {
        // SAFETY: the C library initialises the variable before `main` and never changes it.
        unsafe { STDOUT }
    }

    pub(super) fn stderr() -> *mut libc::FILE {
        // SAFETY: as for `stdout`.
        unsafe { STDERR }
    }
}
