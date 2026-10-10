//! The guest in a process of its own: the parent's side of it.
//!
//! The program starts itself again with the hidden option [`GUEST_ROLE`], which names a Unix
//! stream socket the parent listens on, in a directory only the user can enter, and carries
//! the seed of the run. The child settles its configuration from the same arguments, sets
//! the game up as the guest thread of the other form does, connects, and plays the game with
//! the socket as its link.
//!
//! The runners of the host know nothing of this. The guest they are given is [`relay`],
//! which holds the guest end of their in-process link and hands every message on, one thread
//! for each direction.

use std::io::BufReader;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use vkxp_proto::framing::{read_message, write_message};
use vkxp_proto::link::GuestEnd;
use vkxp_proto::{GuestToHost, HostToGuest};

use crate::GUEST_WAIT;
use crate::args::GUEST_ROLE;

/// How long the parent waits for the guest process to connect. The guest connects once the
/// game's files are open and the interpreter runs.
const GUEST_CONNECT: Duration = Duration::from_secs(30);

/// How often a wait looks at the guest process.
const POLL: Duration = Duration::from_millis(5);

/// A running guest process and the directory of its socket. Dropping it ends a process that
/// still runs and removes the directory.
pub(crate) struct GuestProcess {
    child: Child,
    dir: PathBuf,
}

impl GuestProcess {
    /// Starts the guest process of a run with this seed and waits for it to connect. To be
    /// called on the main thread, before the working directory changes: the child reads the
    /// arguments of this process as this process did, and it is ended by the kernel when
    /// the thread that started it ends.
    pub(crate) fn start(seed: u64) -> Result<(GuestProcess, UnixStream)> {
        let variable = std::env::var_os("XDG_RUNTIME_DIR").filter(|value| !value.is_empty());
        let base = variable.map_or_else(std::env::temp_dir, PathBuf::from);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        let dir = base.join(format!("vkxp-{}-{stamp:08x}", std::process::id()));
        // Fails when the directory exists: one made by somebody else is not used.
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .with_context(|| format!("cannot create the directory {}", dir.display()))?;
        let remove = |error| {
            let _ = std::fs::remove_dir_all(&dir);
            error
        };

        let socket = dir.join("guest.sock");
        let listener = UnixListener::bind(&socket)
            .and_then(|listener| listener.set_nonblocking(true).map(|()| listener))
            .with_context(|| format!("cannot listen on the socket {}", socket.display()))
            .map_err(remove)?;
        let role = socket
            .to_str()
            .map(|socket| format!("{seed}={socket}"))
            .ok_or_else(|| anyhow!("the socket path {} is not UTF-8", socket.display()))
            .map_err(remove)?;
        let child = std::env::current_exe()
            .and_then(|program| {
                Command::new(program)
                    .args(std::env::args_os().skip(1))
                    .args([GUEST_ROLE, &role])
                    .spawn()
            })
            .context("cannot start the guest process")
            .map_err(remove)?;
        let mut process = GuestProcess { child, dir };

        let deadline = Instant::now() + GUEST_CONNECT;
        loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    stream.set_nonblocking(false)?;
                    return Ok((process, stream));
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error).context("cannot accept the guest process"),
            }
            if let Some(status) = process.child.try_wait()? {
                // It has said why on the error stream, unless a signal ended it.
                bail!("the guest process {}", ended(status));
            }
            if Instant::now() >= deadline {
                bail!(
                    "the guest process did not connect within {} seconds",
                    GUEST_CONNECT.as_secs()
                );
            }
            thread::sleep(POLL);
        }
    }

    /// Waits for the guest process to exit, once the host has ended, and says whether the
    /// game ended without a failure. A process that does not exit within [`GUEST_WAIT`] is
    /// ended.
    pub(crate) fn wait(mut self) -> Result<bool> {
        let deadline = Instant::now() + GUEST_WAIT;
        let status = loop {
            if let Some(status) = self.child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                eprintln!(
                    "vkxp: the game's scripts did not end within {} seconds of being asked to \
                     quit; giving up",
                    GUEST_WAIT.as_secs()
                );
                // Dropping `self` ends the process.
                return Ok(false);
            }
            thread::sleep(POLL);
        };
        // A guest that exits with a status has reported its failure itself.
        if status.signal().is_some() {
            eprintln!("vkxp: the guest process {}", ended(status));
        }
        Ok(status.success())
    }
}

impl Drop for GuestProcess {
    fn drop(&mut self) {
        // Both fail for a process that has been waited for already.
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// How a process ended, to follow "the guest process".
fn ended(status: ExitStatus) -> String {
    match (status.code(), status.signal()) {
        (Some(code), _) => format!("exited with status {code} before it connected"),
        (None, Some(signal)) => format!("was ended by signal {}", signal_name(signal)),
        (None, None) => format!("ended: {status}"),
    }
}

/// The name of a signal, as far as a process is ended by it in practice.
fn signal_name(signal: i32) -> String {
    let name = match signal {
        libc::SIGHUP => "SIGHUP",
        libc::SIGINT => "SIGINT",
        libc::SIGQUIT => "SIGQUIT",
        libc::SIGILL => "SIGILL",
        libc::SIGTRAP => "SIGTRAP",
        libc::SIGABRT => "SIGABRT",
        libc::SIGBUS => "SIGBUS",
        libc::SIGFPE => "SIGFPE",
        libc::SIGKILL => "SIGKILL",
        libc::SIGUSR1 => "SIGUSR1",
        libc::SIGSEGV => "SIGSEGV",
        libc::SIGUSR2 => "SIGUSR2",
        libc::SIGPIPE => "SIGPIPE",
        libc::SIGALRM => "SIGALRM",
        libc::SIGTERM => "SIGTERM",
        libc::SIGVTALRM => "SIGVTALRM",
        libc::SIGXCPU => "SIGXCPU",
        libc::SIGXFSZ => "SIGXFSZ",
        libc::SIGSYS => "SIGSYS",
        _ => return signal.to_string(),
    };
    name.to_owned()
}

/// Stands in for the guest at the guest end of an in-process link: hands what the guest
/// process sends over `stream` to the host and what the host sends to the guest process.
/// Returns once both directions have ended. When the connection ends, the host sees the
/// guest as gone; when the host has gone, the connection is closed.
pub(crate) fn relay(link: GuestEnd, stream: UnixStream) {
    let (to_host, from_host) = link.split();
    // Set once the host has gone: what the connection does from then on is no failure.
    let closing = AtomicBool::new(false);
    thread::scope(|scope| {
        let (stream, closing) = (&stream, &closing);
        scope.spawn(move || {
            while let Ok(message) = from_host.recv() {
                let written: Result<_, _> = write_message::<HostToGuest>(&mut &*stream, &message);
                if let Err(error) = written {
                    tracing::debug!("cannot write to the guest process: {error}");
                    break;
                }
            }
            closing.store(true, Ordering::SeqCst);
            // Ends the read below, and tells the guest process that the host has gone.
            let _ = stream.shutdown(std::net::Shutdown::Both);
        });

        let mut reader = BufReader::new(stream);
        loop {
            match read_message::<GuestToHost>(&mut reader) {
                Ok(Some(message)) => {
                    if to_host.send(message).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    if !closing.load(Ordering::SeqCst) {
                        tracing::warn!("the connection to the guest process failed: {error}");
                    }
                    break;
                }
            }
        }
        // The host learns that the guest has gone and lets go of its end, which ends the
        // other direction.
        drop(to_host);
    });
}

#[cfg(test)]
mod tests {
    use vkxp_host::headless_link;
    use vkxp_proto::InputSnapshot;

    use super::*;

    #[test]
    fn the_relay_hands_messages_on_in_both_directions() {
        let (guest_end, host) = headless_link();
        let (near, far) = UnixStream::pair().unwrap();
        let relaying = thread::spawn(move || relay(guest_end, near));
        let guest = GuestEnd::over_stream(far);

        guest.send(GuestToHost::Freeze).unwrap();
        guest.send(GuestToHost::FrameReset).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::Freeze));
        assert_eq!(host.recv(), Ok(GuestToHost::FrameReset));
        host.send(HostToGuest::Input(InputSnapshot::EMPTY)).unwrap();
        host.send(HostToGuest::Tick).unwrap();
        assert_eq!(guest.recv(), Ok(HostToGuest::Input(InputSnapshot::EMPTY)));
        assert_eq!(guest.recv(), Ok(HostToGuest::Tick));

        // The guest goes: the host drains what it sent and sees it gone.
        guest.send(GuestToHost::Shutdown).unwrap();
        drop(guest);
        assert_eq!(host.recv(), Ok(GuestToHost::Shutdown));
        assert!(host.recv().is_err());
        drop(host);
        relaying.join().unwrap();
    }

    #[test]
    fn the_relay_closes_the_connection_when_the_host_has_gone() {
        let (guest_end, host) = headless_link();
        let (near, far) = UnixStream::pair().unwrap();
        let relaying = thread::spawn(move || relay(guest_end, near));
        let guest = GuestEnd::over_stream(far);
        drop(host);
        assert!(guest.recv().is_err());
        relaying.join().unwrap();
    }

    #[test]
    fn a_signal_is_named() {
        assert_eq!(signal_name(libc::SIGKILL), "SIGKILL");
        assert_eq!(signal_name(libc::SIGSEGV), "SIGSEGV");
        assert_eq!(signal_name(63), "63");
    }
}
