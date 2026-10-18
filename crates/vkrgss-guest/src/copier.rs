//! States as copies of the guest process: the guest's side of it.
//!
//! This is platform code for Linux, Android and macOS. A guest process that is asked to save a state calls
//! `fork` on its interpreter thread, in a wait for a tick ([`ForkCopier::park`], which the
//! executable calls from the copier it gives its binding). The process that asked goes on. The copy connects to
//! the socket of the program by itself, states its version as every guest does, says with
//! its first message which slot it is the state of, and waits: it is parked. A parked copy that is told to resume first leaves a
//! copy of itself behind in the same way, so that the slot can be loaded again, and then
//! comes back with its connection as the new link ([`Parked::Resumed`]). One whose connection ends
//! exits at once.
//!
//! In a copy only the thread that called `fork` exists. So a copy never returns to the main
//! thread of the guest process: when its game has ended it exits from the interpreter thread
//! ([`exit_if_copy`]).

use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use vkrgss_proto::framing::{read_message, write_message};
use vkrgss_proto::link::GuestEnd;
use vkrgss_proto::{GuestToHost, HostToGuest};

/// Whether this process is a copy of the guest process that was parked and resumed.
static COPY: AtomicBool = AtomicBool::new(false);

/// Where [`ForkCopier::park`] came back.
#[derive(Debug)]
pub enum Parked {
    /// In the guest that asked, which goes on. A copy of it is parked.
    WentOn,
    /// In a copy that was parked and has been told to resume, with its link to the host,
    /// which holds nothing of the guest it was copied from.
    Resumed(GuestEnd),
}

/// No copy could be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the guest process could not be copied")]
pub struct ParkFailed;

/// Copies the guest process with `fork`.
#[derive(Debug)]
pub struct ForkCopier {
    /// The socket of the program, which a copy connects to.
    socket: PathBuf,
    /// The version of the executable, which a copy states on its connection.
    version: String,
    /// The descriptor that the link of the binding owns in this process.
    connection: AtomicI32,
    /// What a copy calls to get the timer of the interpreter's threads back.
    restart_timer: fn(),
}

/// Where a call of `fork` came back.
enum Forked {
    /// In the process that called, with whether the copy has attached.
    Caller(bool),
    /// In the copy, which is parked, with its connection.
    Copy(UnixStream),
}

impl ForkCopier {
    /// A copier for the guest whose link is `connection`, which was made by connecting to
    /// `socket`. A copy calls `restart_timer` on its one thread, the interpreter's, before
    /// it connects.
    pub(crate) fn new(
        socket: PathBuf,
        version: String,
        connection: &UnixStream,
        restart_timer: fn(),
    ) -> ForkCopier {
        ForkCopier {
            socket,
            version,
            connection: AtomicI32::new(connection.as_raw_fd()),
            restart_timer,
        }
    }

    /// Forks. The copy connects, takes the inherited connection out of this process and
    /// attaches as the state of `slot`; the caller waits until it has, or has failed.
    fn fork(&self, slot: u8) -> Forked {
        let Ok((mut attached, mut attach)) = std::io::pipe() else {
            return Forked::Caller(false);
        };
        // SAFETY: flushing every output stream of the C library takes no pointer of ours.
        // Without it, what the interpreter has buffered would be written by both processes.
        unsafe { libc::fflush(std::ptr::null_mut()) };
        // SAFETY: `fork` is called on the interpreter thread. The one other thread of the
        // guest process waits for this one and holds no lock, so the copy, in which only
        // this thread exists, finds every lock free and all of the interpreter's memory as
        // this thread left it.
        match unsafe { libc::fork() } {
            -1 => {
                tracing::warn!("cannot fork: {}", std::io::Error::last_os_error());
                Forked::Caller(false)
            }
            0 => {
                drop(attached);
                match self.attach(slot) {
                    Some(stream) => {
                        // Tells the caller, which goes on only now: what it sends next
                        // reaches the program after the attachment.
                        let _ = attach.write_all(&[1]);
                        Forked::Copy(stream)
                    }
                    // SAFETY: ends this process at once, without running anything that
                    // belongs to the process it was copied from.
                    None => unsafe { libc::_exit(1) },
                }
            }
            _ => {
                drop(attach);
                // The end of the pipe without the byte is a copy that could not attach.
                Forked::Caller(attached.read_exact(&mut [0]).is_ok())
            }
        }
    }

    /// What a copy does first: it stops being ended with its parent, gets the timer of the
    /// interpreter's threads back, connects to the program and attaches as the state of
    /// `slot`. The connection it inherited is closed by putting the new one in its place,
    /// so that the descriptor the binding's link still holds is not one of another guest.
    fn attach(&self, slot: u8) -> Option<UnixStream> {
        COPY.store(true, Ordering::SeqCst);
        crate::parent_watch::release_copy();
        // On the interpreter thread, as the interpreter itself restarts its timer after a
        // `fork`: timers are not inherited.
        (self.restart_timer)();
        let mut stream = crate::open(&self.socket, &self.version).ok()?;
        let inherited = self.connection.load(Ordering::SeqCst);
        // SAFETY: both descriptors are open in this process: `stream` is alive, and
        // `inherited` is owned by the link of the binding, which closes it when it takes
        // the new link. `dup2` closes what `inherited` named and makes it name the new
        // connection.
        if unsafe { libc::dup2(stream.as_raw_fd(), inherited) } < 0 {
            return None;
        }
        let attach = GuestToHost::Attach {
            pid: std::process::id(),
            parked: Some(slot),
        };
        write_message(&mut stream, &attach).ok()?;
        Some(stream)
    }
}

impl ForkCopier {
    /// Parks a copy of the guest process as the state of `slot`. To be called on the
    /// interpreter thread, in a wait for a tick. Returns [`Parked::WentOn`] in the guest
    /// that asked and, in a copy that was resumed, the new link.
    pub fn park(&self, slot: u8) -> Result<Parked, ParkFailed> {
        // The connection of this process if it is a copy that was told to resume.
        let mut resumed: Option<UnixStream> = None;
        loop {
            match self.fork(slot) {
                Forked::Caller(attached) => {
                    return match resumed {
                        // Without a new copy the slot is empty from here on.
                        Some(stream) => {
                            self.connection.store(stream.as_raw_fd(), Ordering::SeqCst);
                            Ok(Parked::Resumed(GuestEnd::over_stream(stream)))
                        }
                        None if attached => Ok(Parked::WentOn),
                        None => Err(ParkFailed),
                    };
                }
                Forked::Copy(stream) => {
                    // The connection of the process this one was copied from, if that was
                    // a copy itself: it is not this process's to hold open.
                    drop(resumed.take());
                    // Parked. Read without a buffer: what follows is the link's.
                    match read_message::<HostToGuest>(&mut &stream) {
                        Ok(Some(HostToGuest::Resume)) => resumed = Some(stream),
                        // The connection has ended: the state was replaced, or the
                        // program is gone.
                        // SAFETY: ends this process at once, without unwinding, the
                        // interpreter's shutdown or a flush of what the process it was
                        // copied from had buffered.
                        _ => unsafe { libc::_exit(0) },
                    }
                }
            }
        }
    }
}

/// Ends the process if it is a copy that was resumed, with the status of a game that went
/// `ok`. Called on the interpreter thread when the game has ended; a copy has no other
/// thread to return to.
pub fn exit_if_copy(ok: bool) {
    if COPY.load(Ordering::SeqCst) {
        // SAFETY: flushes the output streams of the C library, which takes no pointer of
        // ours, and ends the process without running the exit handlers, which belong to
        // threads this process does not have.
        unsafe {
            libc::fflush(std::ptr::null_mut());
            libc::_exit(if ok { 0 } else { 1 });
        }
    }
}
