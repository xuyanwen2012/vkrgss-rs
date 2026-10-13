//! The guest in a process of its own: the parent's side of it.
//!
//! The program starts itself again with the hidden option [`GUEST_ROLE`], which names a Unix
//! stream socket the parent listens on, in a directory only the user can enter, and carries
//! the seed of the run. The child settles its configuration from the same arguments, sets
//! the game up as the guest thread of the other form does, connects, and plays the game with
//! the socket as its link.
//!
//! This is platform code for Linux: it waits on several connections at once and ends
//! processes by their number.
//!
//! The runners of the host know nothing of this. The guest they are given is
//! [`GuestProcess::relay`], which holds the guest end of their in-process link and hands
//! every message on.
//!
//! # States
//!
//! A state is a copy of the guest process that is parked (`copier`). Every guest process
//! opens its connection with `Attach`, a parked copy with the slot it is the state of, and
//! the relay keeps the connection of the newest copy of each slot. A request to load a
//! state does not reach a guest: the relay ends the process of the running guest, tells the
//! parked copy of the slot to resume, and hands on what that one sends from then on. The
//! outcome a guest reports when its game has ended is kept for the exit status, which so
//! does not depend on which process ended the game.
//!
//! When the relay ends it ends every process whose connection it holds, and waits until
//! those and the ones it let go of before are gone.

use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use vkxp_guest::role::{GUEST_ROLE, GuestRole};
use vkxp_proto::framing::{read_message, write_message};
use vkxp_proto::link::{GuestEnd, GuestSender};
use vkxp_proto::{GuestToHost, HostToGuest};

use crate::GUEST_WAIT;

/// How long the parent waits for the guest process to connect. The guest connects once the
/// game's files are open and the interpreter runs.
const GUEST_CONNECT: Duration = Duration::from_secs(30);

/// How long a process that has connected may take to say what it is.
const ATTACH_WAIT: Duration = Duration::from_secs(5);

/// How long the relay waits, when it ends, for the processes it has ended to be gone.
const GONE_WAIT: Duration = Duration::from_secs(2);

/// How often a wait looks at the guest process.
const POLL: Duration = Duration::from_millis(5);

/// The number of state slots.
const SLOTS: usize = 10;

/// A guest process that has attached, and its connection.
struct Attached {
    stream: UnixStream,
    pid: u32,
}

impl Attached {
    /// Ends the process, if it is still the one at the other side of the connection.
    fn end(&self) {
        // A process that is gone has closed its side; its number may be another's by now.
        if !readable(self.stream.as_fd()) || self.is_running() {
            kill(self.pid);
        }
    }

    /// Whether the other side has sent something that is not the end of the connection.
    fn is_running(&self) -> bool {
        let mut byte = [0];
        // SAFETY: `byte` is valid for one byte, and the descriptor is open while `self` is.
        let peeked = unsafe {
            libc::recv(
                self.stream.as_raw_fd(),
                byte.as_mut_ptr().cast(),
                1,
                libc::MSG_PEEK | libc::MSG_DONTWAIT,
            )
        };
        peeked > 0
    }
}

/// Sends SIGKILL to the process `pid`.
fn kill(pid: u32) {
    if let Ok(pid) = libc::pid_t::try_from(pid) {
        // SAFETY: `kill` takes no pointer. The caller names a process of this run.
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
}

/// Whether a process `pid` exists.
fn exists(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // SAFETY: `kill` takes no pointer, and signal 0 is sent to nobody.
    let found = unsafe { libc::kill(pid, 0) };
    found == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

/// Removes the socket directories in `base` that earlier runs of this user left behind:
/// the directories named `vkxp-<pid>-<8 hex digits>` that the user owns and whose process no
/// longer exists. Housekeeping: what cannot be read or removed is left.
fn sweep(base: &Path) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    // SAFETY: `geteuid` takes nothing and cannot fail.
    let user = unsafe { libc::geteuid() };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let pid = name.to_str().and_then(|name| {
            let (pid, stamp) = name.strip_prefix("vkxp-")?.split_once('-')?;
            let stamp = stamp.len() == 8 && stamp.bytes().all(|byte| byte.is_ascii_hexdigit());
            let pid = pid.bytes().all(|byte| byte.is_ascii_digit()).then_some(pid);
            pid.filter(|_| stamp)?.parse::<u32>().ok()
        });
        if pid.is_none_or(exists) {
            continue;
        }
        // Of the entry itself: a link is not followed.
        let ours = std::fs::symlink_metadata(entry.path())
            .is_ok_and(|entry| entry.is_dir() && entry.uid() == user);
        if ours {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Waits until one of `descriptors` can be read, or has ended, or `timeout` has passed, and
/// says of each whether it can.
fn wait_readable<const N: usize>(
    descriptors: [Option<BorrowedFd<'_>>; N],
    timeout: Option<Duration>,
) -> [bool; N] {
    let mut polled = descriptors.map(|descriptor| libc::pollfd {
        // A negative descriptor is ignored.
        fd: descriptor.map_or(-1, |descriptor| descriptor.as_raw_fd()),
        events: libc::POLLIN,
        revents: 0,
    });
    let timeout = timeout.map_or(-1, |timeout| {
        libc::c_int::try_from(timeout.as_millis().max(1)).unwrap_or(libc::c_int::MAX)
    });
    // SAFETY: `polled` is valid for its `N` entries, and the descriptors are borrowed for
    // the time of the call.
    let ready = unsafe { libc::poll(polled.as_mut_ptr(), N as libc::nfds_t, timeout) };
    // An interrupted wait says that nothing can be read, and the caller waits again.
    polled.map(|entry| ready > 0 && entry.revents != 0)
}

/// Whether `descriptor` can be read now, or has ended.
fn readable(descriptor: BorrowedFd<'_>) -> bool {
    wait_readable([Some(descriptor)], Some(Duration::ZERO))[0]
}

/// Takes the first message of a process that has connected: what it is.
fn attached(stream: UnixStream) -> Result<(Attached, Option<u8>)> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(ATTACH_WAIT))?;
    let first = read_message::<GuestToHost>(&mut &stream)?;
    stream.set_read_timeout(None)?;
    match first {
        Some(GuestToHost::Attach { pid, parked }) => Ok((Attached { stream, pid }, parked)),
        other => bail!("a process connected and did not attach: its first message is {other:?}"),
    }
}

/// A running guest process, the socket its copies connect to, and the directory of that
/// socket. Dropping it ends the process if it still runs and removes the directory.
pub(crate) struct GuestProcess {
    child: Child,
    dir: PathBuf,
    listener: UnixListener,
    /// The connection of the child, until the relay takes it.
    first: Option<Attached>,
}

impl GuestProcess {
    /// Starts the guest process of a run with this seed and waits for it to connect. To be
    /// called on the main thread, before the working directory changes: the child reads the
    /// arguments of this process as this process did, and it is ended by the kernel when
    /// the thread that started it ends.
    pub(crate) fn start(seed: u64) -> Result<GuestProcess> {
        let variable = std::env::var_os("XDG_RUNTIME_DIR").filter(|value| !value.is_empty());
        let base = variable.map_or_else(std::env::temp_dir, PathBuf::from);
        sweep(&base);
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
        let role = GuestRole { seed, socket }
            .value()
            .map_err(|error| remove(error.into()))?;
        let child = std::env::current_exe()
            .and_then(|program| {
                Command::new(program)
                    .args(std::env::args_os().skip(1))
                    .args([GUEST_ROLE, &role])
                    .spawn()
            })
            .context("cannot start the guest process")
            .map_err(remove)?;
        let mut process = GuestProcess {
            child,
            dir,
            listener,
            first: None,
        };

        let deadline = Instant::now() + GUEST_CONNECT;
        loop {
            match process.listener.accept() {
                Ok((stream, _)) => {
                    let (first, _) = attached(stream)?;
                    tracing::info!("guest process {} attached", first.pid);
                    process.first = Some(first);
                    return Ok(process);
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

    /// Stands in for the guest at the guest end of an in-process link: hands what the
    /// running guest process sends to the host and what the host sends to that process, and
    /// keeps the states; see the module documentation. When the connection of the running
    /// guest ends, the host sees the guest as gone.
    ///
    /// Returns once the host has gone and the running guest process has ended, and says
    /// whether the game ended without a failure. A guest process that is still there
    /// [`GUEST_WAIT`] after the host went is ended, which is a failure if it has not
    /// reported how its game went. A guest process that has closed its connection ends by
    /// itself and is waited for, for as long again at most: it may still be writing to the
    /// error stream, its frame time statistics for one. Every process of the run is gone
    /// when this returns.
    pub(crate) fn relay(mut self, link: GuestEnd) -> bool {
        let first = self.first.take();
        let relayed = relay(link, &self.listener, first, self.child.id());
        let Relayed {
            outcome,
            stuck,
            left,
            last,
            mut gone,
        } = relayed;
        let first_ran_last = last == self.child.id();

        if left && first_ran_last {
            // Ending it now would cut off what it writes after its last message.
            let deadline = Instant::now() + GUEST_WAIT;
            while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < deadline {
                thread::sleep(POLL);
            }
        }
        let _ = self.child.kill();
        let status = self.child.wait();
        gone.retain(|pid| *pid != self.child.id());
        let deadline = Instant::now() + GONE_WAIT;
        while gone.iter().any(|pid| exists(*pid)) && Instant::now() < deadline {
            thread::sleep(POLL);
        }

        if stuck {
            eprintln!(
                "vkxp: the game's scripts did not end within {} seconds of being asked to \
                 quit; giving up",
                GUEST_WAIT.as_secs()
            );
            return false;
        }
        match (outcome, status) {
            (Some(ok), _) => ok,
            // The child was the guest to the end and went without a word. One that exits
            // with a status has reported its failure itself.
            (None, Ok(status)) if first_ran_last => {
                if status.signal().is_some() {
                    eprintln!("vkxp: the guest process {}", ended(status));
                }
                false
            }
            (None, _) => {
                eprintln!("vkxp: the guest process ended without saying how the game went");
                false
            }
        }
    }
}

/// How a relay ended.
struct Relayed {
    /// How the game went, as the guest that ended it reported.
    outcome: Option<bool>,
    /// Whether the guest was given up on: the host had gone and the guest had not reported.
    stuck: bool,
    /// Whether the running guest closed its connection, or ended, by itself.
    left: bool,
    /// The process that was the running guest last.
    last: u32,
    /// The processes that the relay ended or told to exit.
    gone: Vec<u32>,
}

/// Relays between `link` and the guest processes: `first`, the process `first_pid`, which
/// has attached, and the copies that connect to `listener`. See [`GuestProcess::relay`].
fn relay(
    link: GuestEnd,
    listener: &UnixListener,
    first: Option<Attached>,
    first_pid: u32,
) -> Relayed {
    let (to_host, from_host) = link.split();
    let mut relay = Relay {
        listener,
        to_host: Some(to_host),
        last: first_pid,
        running: first,
        slots: Default::default(),
        refilling: None,
        outcome: None,
        let_go: Vec::new(),
    };
    // The host's messages come through a channel, which cannot be waited for together with
    // the connections: a thread puts them on a socket that can.
    let ran = UnixStream::pair().and_then(|(near, far)| {
        thread::scope(|scope| {
            scope.spawn(move || {
                while let Ok(message) = from_host.recv() {
                    if write_message(&mut &near, &message).is_err() {
                        break;
                    }
                }
                // Dropping `near` tells the relay that the host has gone.
            });
            relay.run(&far)
        })
    });
    let stuck = ran.unwrap_or_else(|error| {
        eprintln!("vkxp: cannot relay to the guest process: {error}");
        false
    });
    let last = relay.last;
    let left = relay.running.is_none();
    let (outcome, gone) = relay.finish();
    Relayed {
        outcome,
        stuck,
        left,
        last,
        gone,
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

/// What the relay keeps while it runs.
struct Relay<'a> {
    listener: &'a UnixListener,
    /// The host's side, until the running guest has gone.
    to_host: Option<GuestSender>,
    /// The guest process whose messages are handed on.
    running: Option<Attached>,
    /// The process that is the running guest, or was it last.
    last: u32,
    /// The parked copy of each slot.
    slots: [Option<Attached>; SLOTS],
    /// The slot whose copy was resumed and is expected to leave a new copy of itself.
    refilling: Option<u8>,
    /// How the game went, as the guest that ended it reported.
    outcome: Option<bool>,
    /// The processes that were ended or told to exit while the relay ran.
    let_go: Vec<u32>,
}

impl Relay<'_> {
    /// Hands messages on until the host has gone and the running guest has ended, or
    /// [`GUEST_WAIT`] has passed since the host went. Returns whether the guest was given
    /// up on: it had not reported how its game went by then.
    fn run(&mut self, host: &UnixStream) -> std::io::Result<bool> {
        // Since when the host is gone.
        let mut host_gone: Option<Instant> = None;
        loop {
            let waited = match host_gone {
                None => None,
                // The guest ends by itself: it may have more to write than its outcome.
                Some(_) if self.running.is_none() => return Ok(false),
                Some(since) => match GUEST_WAIT.checked_sub(since.elapsed()) {
                    Some(left) => Some(left),
                    None => return Ok(self.outcome.is_none()),
                },
            };
            let running = self.running.as_ref().map(|running| running.stream.as_fd());
            let host_side = host_gone.is_none().then_some(host.as_fd());
            // The connection of the guest is looked at before the socket: when a message
            // of the guest can be read, a copy it made before it sent that has connected.
            let [from_guest, from_host, _] =
                wait_readable([running, host_side, Some(self.listener.as_fd())], waited);
            if from_guest {
                // Before the message, so that a copy is known before what its maker sent
                // after making it.
                self.accept_copies();
                self.guest_message();
            } else {
                self.accept_copies();
            }
            if from_host {
                match read_message::<HostToGuest>(&mut &*host) {
                    Ok(Some(message)) => self.host_message(message),
                    Ok(None) | Err(_) => {
                        host_gone = Some(Instant::now());
                        self.to_host = None;
                        // The guest reads the end of its link as a request to quit, and
                        // can still say how its game went.
                        if let Some(running) = &self.running {
                            let _ = running.stream.shutdown(std::net::Shutdown::Write);
                        }
                    }
                }
            }
        }
    }

    /// Takes the processes that have connected: the copies of guest processes.
    fn accept_copies(&mut self) {
        while let Ok((stream, _)) = self.listener.accept() {
            match attached(stream) {
                Ok((copy, Some(slot))) if usize::from(slot) < SLOTS => self.parked(copy, slot),
                Ok((copy, _)) => {
                    tracing::warn!("guest process {} attached without a slot", copy.pid);
                    copy.end();
                    self.let_go.push(copy.pid);
                }
                Err(error) => tracing::warn!("{error:#}"),
            }
        }
    }

    /// A copy has attached as the state of `slot`.
    fn parked(&mut self, copy: Attached, slot: u8) {
        tracing::info!(
            "guest process {} attached as the state of slot {slot}",
            copy.pid
        );
        // The copy the slot had sees the end of its connection and exits.
        if let Some(replaced) = self.slots[usize::from(slot)].replace(copy) {
            self.let_go.push(replaced.pid);
        }
        if self.refilling == Some(slot) {
            self.refilling = None;
        } else {
            self.tell_host(GuestToHost::StateSaved { slot });
        }
    }

    /// Takes one message of the running guest, or the end of its connection.
    fn guest_message(&mut self) {
        let Some(running) = &self.running else {
            return;
        };
        // A resumed copy that has not left a copy of itself by now has left none.
        self.refilling = None;
        match read_message::<GuestToHost>(&mut &running.stream) {
            Ok(Some(GuestToHost::Outcome { ok })) => self.outcome = Some(ok),
            Ok(Some(message)) => self.tell_host(message),
            ended => {
                if let Err(error) = ended
                    && self.to_host.is_some()
                {
                    tracing::warn!("the connection to the guest process failed: {error}");
                }
                // The host learns that the guest has gone.
                self.running = None;
                self.to_host = None;
            }
        }
    }

    /// Takes one message of the host.
    fn host_message(&mut self, message: HostToGuest) {
        match message {
            HostToGuest::LoadState { slot } => self.load(slot),
            message => {
                // A guest that is gone is found out by the end of its connection.
                if let Some(running) = &self.running {
                    let _ = write_message(&mut &running.stream, &message);
                }
            }
        }
    }

    /// Loads the state of `slot`: ends the running guest and resumes the parked copy.
    fn load(&mut self, slot: u8) {
        let parked = self.slots.get_mut(usize::from(slot)).and_then(Option::take);
        // A copy that is gone has closed its connection, which a parked one never writes to.
        let Some(parked) = parked.filter(|parked| !readable(parked.stream.as_fd())) else {
            self.tell_host(GuestToHost::StateFailed { slot, load: true });
            return;
        };
        // Killed, not asked to quit: nothing of the game runs after the decision to load.
        if let Some(running) = self.running.take() {
            kill(running.pid);
            self.let_go.push(running.pid);
        }
        let resumed = write_message(&mut &parked.stream, &HostToGuest::Resume);
        tracing::info!(
            "guest process {} resumes the state of slot {slot}",
            parked.pid
        );
        self.last = parked.pid;
        self.running = Some(parked);
        self.refilling = Some(slot);
        match resumed {
            Ok(()) => self.tell_host(GuestToHost::StateLoaded { slot }),
            // Found out by the end of the connection.
            Err(error) => tracing::warn!("cannot resume the state of slot {slot}: {error}"),
        }
    }

    fn tell_host(&mut self, message: GuestToHost) {
        if let Some(to_host) = &self.to_host
            && to_host.send(message).is_err()
        {
            self.to_host = None;
        }
    }

    /// Ends every process whose connection the relay holds. Returns how the game went and
    /// the processes that are to be gone: these and the ones let go of before.
    fn finish(mut self) -> (Option<bool>, Vec<u32>) {
        let held = self.running.iter().chain(self.slots.iter().flatten());
        for process in held {
            process.end();
            self.let_go.push(process.pid);
        }
        (self.outcome, self.let_go)
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

#[cfg(test)]
mod tests {
    use std::process::Stdio;

    use vkxp_host::{HostLink, headless_link};
    use vkxp_proto::InputSnapshot;

    use super::*;

    /// A process that does nothing, to be ended in place of a guest.
    fn idle() -> Child {
        Command::new("sleep")
            .arg("600")
            .stdin(Stdio::null())
            .spawn()
            .expect("sleep starts")
    }

    /// A relay on a thread of its own, for a first guest that is `process` at the other
    /// side of the returned connection.
    fn relaying(
        dir: &std::path::Path,
        process: &Child,
    ) -> (HostLink, GuestEnd, thread::JoinHandle<Relayed>) {
        let listener = UnixListener::bind(dir.join("guest.sock")).unwrap();
        listener.set_nonblocking(true).unwrap();
        let (guest_end, host) = headless_link();
        let (near, far) = UnixStream::pair().unwrap();
        let pid = process.id();
        let first = Attached { stream: near, pid };
        let relay = thread::spawn(move || relay(guest_end, &listener, Some(first), pid));
        (host, GuestEnd::over_stream(far), relay)
    }

    #[test]
    fn the_relay_hands_messages_on_in_both_directions_and_keeps_the_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let mut process = idle();
        let (host, guest, relay) = relaying(dir.path(), &process);

        guest.send(GuestToHost::Freeze).unwrap();
        guest.send(GuestToHost::FrameReset).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::Freeze));
        assert_eq!(host.recv(), Ok(GuestToHost::FrameReset));
        host.send(HostToGuest::Input(InputSnapshot::EMPTY)).unwrap();
        host.send(HostToGuest::Tick).unwrap();
        assert_eq!(guest.recv(), Ok(HostToGuest::Input(InputSnapshot::EMPTY)));
        assert_eq!(guest.recv(), Ok(HostToGuest::Tick));

        // The guest goes: the host gets what it sent but the outcome, and sees it gone.
        guest.send(GuestToHost::Outcome { ok: true }).unwrap();
        guest.send(GuestToHost::Shutdown).unwrap();
        drop(guest);
        assert_eq!(host.recv(), Ok(GuestToHost::Shutdown));
        assert!(host.recv().is_err());
        drop(host);
        let relayed = relay.join().unwrap();
        assert_eq!(relayed.outcome, Some(true));
        assert!(!relayed.stuck);
        assert!(relayed.left);
        assert_eq!(relayed.last, process.id());
        // A guest that has closed its connection is not ended by its number.
        assert_eq!(process.try_wait().unwrap(), None);
        process.kill().unwrap();
        process.wait().unwrap();
    }

    #[test]
    fn the_relay_tells_the_guest_that_the_host_has_gone() {
        let dir = tempfile::tempdir().unwrap();
        let mut process = idle();
        let (host, guest, relay) = relaying(dir.path(), &process);
        drop(host);
        assert!(guest.recv().is_err());
        // The guest can still say how its game went.
        guest.send(GuestToHost::Outcome { ok: false }).unwrap();
        drop(guest);
        let relayed = relay.join().unwrap();
        assert_eq!(relayed.outcome, Some(false));
        process.kill().unwrap();
        process.wait().unwrap();
    }

    /// A guest process writes after its last message, its frame time statistics for one.
    /// The marker stands for that: without the wait the process is ended long before it
    /// is written.
    #[test]
    fn a_guest_process_that_has_closed_its_connection_is_left_to_end_by_itself() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("written");
        let child = Command::new("sh")
            .args(["-c", "sleep 0.3 && : > \"$0\""])
            .arg(&marker)
            .stdin(Stdio::null())
            .spawn()
            .expect("sh starts");
        let listener = UnixListener::bind(dir.path().join("guest.sock")).unwrap();
        listener.set_nonblocking(true).unwrap();
        let (near, far) = UnixStream::pair().unwrap();
        let pid = child.id();
        let process = GuestProcess {
            child,
            dir: dir.path().join("sockets"),
            listener,
            first: Some(Attached { stream: near, pid }),
        };
        let (guest_end, host) = headless_link();
        drop(host);
        let guest = GuestEnd::over_stream(far);
        guest.send(GuestToHost::Outcome { ok: true }).unwrap();
        drop(guest);

        assert!(process.relay(guest_end));
        assert!(marker.is_file());
    }

    /// A copy that attaches as the state of `slot` under the number of `process`.
    fn park(dir: &std::path::Path, process: &Child, slot: u8) -> UnixStream {
        let mut stream = UnixStream::connect(dir.join("guest.sock")).unwrap();
        let attach = GuestToHost::Attach {
            pid: process.id(),
            parked: Some(slot),
        };
        write_message(&mut stream, &attach).unwrap();
        stream
    }

    /// Scenario "Left by a killed run".
    #[test]
    fn the_socket_directories_of_ended_runs_are_removed() {
        let base = tempfile::tempdir().unwrap();
        // A process that has ended and been waited for: its id names nobody.
        let mut ended = idle();
        ended.kill().unwrap();
        ended.wait().unwrap();
        let left = base.path().join(format!("vkxp-{}-0000abcd", ended.id()));
        let running = base
            .path()
            .join(format!("vkxp-{}-0000abcd", std::process::id()));
        let other = base.path().join("vkxp-notes");
        for dir in [&left, &running] {
            std::fs::create_dir(dir).unwrap();
            std::fs::write(dir.join("guest.sock"), b"").unwrap();
        }
        std::fs::write(&other, b"kept").unwrap();

        sweep(base.path());
        assert!(!left.exists());
        assert!(running.join("guest.sock").exists());
        assert!(other.exists());
    }

    #[test]
    fn the_relay_keeps_states_by_slot_and_loads_them() {
        let dir = tempfile::tempdir().unwrap();
        let (mut first, mut replaced, mut copy, mut again) = (idle(), idle(), idle(), idle());
        let (host, guest, relay) = relaying(dir.path(), &first);

        // An empty slot.
        host.send(HostToGuest::LoadState { slot: 3 }).unwrap();
        assert_eq!(
            host.recv(),
            Ok(GuestToHost::StateFailed {
                slot: 3,
                load: true
            })
        );
        // A request to save goes to the guest, whose copy attaches.
        host.send(HostToGuest::SaveState { slot: 3 }).unwrap();
        assert_eq!(guest.recv(), Ok(HostToGuest::SaveState { slot: 3 }));
        let old = park(dir.path(), &replaced, 3);
        guest.send(GuestToHost::Freeze).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::StateSaved { slot: 3 }));
        assert_eq!(host.recv(), Ok(GuestToHost::Freeze));
        // A second copy takes the slot, and the connection of the first ends.
        let parked = park(dir.path(), &copy, 3);
        guest.send(GuestToHost::Freeze).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::StateSaved { slot: 3 }));
        assert_eq!(host.recv(), Ok(GuestToHost::Freeze));
        assert_eq!(read_message::<HostToGuest>(&mut &old).unwrap(), None);

        // Loading ends the running guest and resumes the copy, which is the guest now.
        host.send(HostToGuest::LoadState { slot: 3 }).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::StateLoaded { slot: 3 }));
        assert_eq!(first.wait().unwrap().signal(), Some(libc::SIGKILL));
        assert!(guest.recv().is_err());
        let resumed = GuestEnd::over_stream(parked);
        assert_eq!(resumed.recv(), Ok(HostToGuest::Resume));
        // The copy it leaves of itself fills the slot again without a word to the host.
        let _refilled = park(dir.path(), &again, 3);
        resumed.send(GuestToHost::FrameReset).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::FrameReset));
        host.send(HostToGuest::Tick).unwrap();
        assert_eq!(resumed.recv(), Ok(HostToGuest::Tick));

        resumed.send(GuestToHost::Outcome { ok: false }).unwrap();
        resumed.send(GuestToHost::Shutdown).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::Shutdown));
        drop(host);
        // The host has gone and the guest still runs: it is told, and ended with the rest.
        assert!(resumed.recv().is_err());
        drop(resumed);
        let relayed = relay.join().unwrap();
        assert_eq!(relayed.outcome, Some(false));
        assert_eq!(relayed.last, copy.id());
        // The parked copy is ended; the replaced one was only told to exit.
        assert_eq!(again.wait().unwrap().signal(), Some(libc::SIGKILL));
        for pid in [first.id(), replaced.id(), again.id()] {
            assert!(relayed.gone.contains(&pid), "{pid} in {:?}", relayed.gone);
        }
        for process in [&mut replaced, &mut copy] {
            let _ = process.kill();
            process.wait().unwrap();
        }
    }

    #[test]
    fn a_signal_is_named() {
        assert_eq!(signal_name(libc::SIGKILL), "SIGKILL");
        assert_eq!(signal_name(libc::SIGSEGV), "SIGSEGV");
        assert_eq!(signal_name(63), "63");
    }
}
