//! The guest in a process of its own: the parent's side of it.
//!
//! The program makes a directory only the user can enter and listens on a Unix stream
//! socket in it. Beside the socket it writes the guest record of `vkrgss-guest`: the game
//! directory, the settings a guest reads, the seed, the socket and the inputs of the
//! diagnostic level. Then it starts the guest executable of the game's runtime, which lies
//! beside the program, with the hidden option [`GUEST_ROLE`] and the path of the record, and
//! with no other argument; it never starts itself. The child reads the record and
//! neither the command line of the program nor a configuration file, sets the game up as
//! the guest thread of the other form does, connects, and plays the game with the socket as
//! its link. The record is removed with the directory, when the run ends.
//!
//! This is platform code for Linux: it waits on several connections at once and ends
//! processes by their number. The Android app (`vkrgss-android`) compiles this file as a
//! module of its own: there the guest executables lie in the app's native library directory
//! under the names of libraries, and the socket directory is made in the app's private
//! directory (see [`guest_executable`]).
//!
//! The runners of the host know nothing of this. The guest they are given is
//! [`GuestProcess::relay`], which holds the guest end of their in-process link and hands
//! every message on.
//!
//! # States
//!
//! A state is a copy of the guest process that is parked (the copier of `vkrgss-guest`). Every
//! guest process opens its connection with the statement of its version, which has to be the
//! version of the program, and then `Attach`, a parked copy with the slot it is the state of, and
//! the relay keeps the connection of the newest copy of each slot. A request to load a
//! state does not reach a guest: the relay ends the process of the running guest, tells the
//! parked copy of the slot to resume, and hands on what that one sends from then on. The
//! outcome a guest reports when its game has ended is kept for the exit status, which so
//! does not depend on which process ended the game.
//!
//! When the relay ends it ends every process whose connection it holds, and waits until
//! those and the ones it let go of before are gone.
//!
//! # Identity
//!
//! The number of a process that attaches is the one the kernel gives for the other side of
//! its connection (`SO_PEERCRED`, on macOS `LOCAL_PEERPID`), and its `Attach` has to state
//! that number. A process of another user is refused, and so is a first process that is not
//! the child the program started. Only numbers taken this way are signalled.

use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use vkrgss_guest::hello;
use vkrgss_guest::record::{Diagnostics, GuestRecord};
use vkrgss_guest::role::{GUEST_ROLE, GuestRole};
use vkrgss_guest::settings::GuestSettings;
use vkrgss_proto::framing::{read_message, write_message};
use vkrgss_proto::link::{GuestEnd, GuestSender};
use vkrgss_proto::{GuestToHost, HostToGuest};
use vkrgss_vfs::Runtime;

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

/// The version of the program, which a guest process has to state.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The number of state slots.
const SLOTS: usize = 10;

/// A guest process that has attached, and its connection.
struct Attached {
    stream: UnixStream,
    /// The number of the process, as the kernel gave it for the other side of the
    /// connection ([`peer`]).
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

/// Sends SIGKILL to the process `pid`: the number the kernel gave for the other side of a
/// connection ([`peer`]), never one that a process stated. A number that is not positive,
/// which would name a group of processes, is refused.
fn kill(pid: u32) {
    match libc::pid_t::try_from(pid) {
        // SAFETY: `kill` takes no pointer. The number is that of a single process, which the
        // kernel named as the other side of a connection of this run (see `attached`).
        Ok(pid) if pid > 0 => unsafe {
            libc::kill(pid, libc::SIGKILL);
        },
        _ => tracing::warn!("no signal is sent to the process number {pid}"),
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
/// the directories named `vkrgss-<pid>-<8 hex digits>` that the user owns and whose process no
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
            let (pid, stamp) = name.strip_prefix("vkrgss-")?.split_once('-')?;
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

/// The process at the other side of a connection, as the kernel tells it.
struct Peer {
    pid: libc::pid_t,
    user: libc::uid_t,
}

/// The process that connected at the other side of `stream` and its user: `SO_PEERCRED`.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn peer(stream: &UnixStream) -> std::io::Result<Peer> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: `credentials` and `length` are valid for writes, and `length` holds the size
    // of `credentials`; the descriptor is open while `stream` is.
    let status = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut credentials).cast(),
            &mut length,
        )
    };
    if status != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(Peer {
        pid: credentials.pid,
        user: credentials.uid,
    })
}

/// The process that connected at the other side of `stream` and its user: `LOCAL_PEERPID`
/// and `getpeereid`.
#[cfg(target_os = "macos")]
fn peer(stream: &UnixStream) -> std::io::Result<Peer> {
    let mut pid: libc::pid_t = 0;
    let mut length = std::mem::size_of::<libc::pid_t>() as libc::socklen_t;
    // SAFETY: `pid` and `length` are valid for writes, and `length` holds the size of `pid`;
    // the descriptor is open while `stream` is.
    let status = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERPID,
            (&raw mut pid).cast(),
            &mut length,
        )
    };
    if status != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let (mut user, mut group) = (0, 0);
    // SAFETY: both are valid for writes; the descriptor is open while `stream` is.
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut user, &mut group) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(Peer { pid, user })
}

/// Takes the statement of the version and the first message of a process that has
/// connected: what it is. The process is the one the kernel names as the other side of the
/// connection; one of another user, one the kernel gives no positive number for, one that
/// attaches under another number than that and one of another version than the program's
/// are refused.
fn attached(stream: UnixStream) -> Result<(Attached, Option<u8>)> {
    let peer = peer(&stream).context("cannot tell which process connected")?;
    // SAFETY: `geteuid` takes nothing and cannot fail.
    let user = unsafe { libc::geteuid() };
    if peer.user != user {
        bail!(
            "a process of the user {} connected, not of the user {user} of the program",
            peer.user
        );
    }
    let Some(pid) = u32::try_from(peer.pid).ok().filter(|pid| *pid > 0) else {
        bail!(
            "a process connected whose number the kernel gives as {}",
            peer.pid
        );
    };
    stream.set_nonblocking(false)?;
    set_read_timeout(&stream, Some(ATTACH_WAIT))?;
    hello::read(&mut &stream, VERSION)?;
    let first = read_message::<GuestToHost>(&mut &stream)?;
    set_read_timeout(&stream, None)?;
    match first {
        Some(GuestToHost::Attach {
            pid: stated,
            parked,
        }) if stated == pid => Ok((Attached { stream, pid }, parked)),
        Some(GuestToHost::Attach { pid: stated, .. }) => {
            bail!("the process {pid} connected and attached as the process {stated}")
        }
        other => bail!("a process connected and did not attach: its first message is {other:?}"),
    }
}

/// Sets the read timeout of a connection. macOS refuses that (`EINVAL`) once the other end is
/// closed, as a guest whose game is short may have closed it before it is accepted; what it
/// sent can still be read then, and no read blocks.
fn set_read_timeout(stream: &UnixStream, timeout: Option<Duration>) -> std::io::Result<()> {
    match stream.set_read_timeout(timeout) {
        Err(error)
            if cfg!(target_os = "macos") && error.kind() == std::io::ErrorKind::InvalidInput =>
        {
            Ok(())
        }
        result => result,
    }
}

/// The file name of the guest executable of games of RGSS1 and RGSS2, which lies beside the
/// program.
#[cfg(not(target_os = "android"))]
pub(crate) const RGSS12_GUEST: &str = "vkrgss-guest-rgss12";

/// The file name of the guest executable of games of RGSS3, which lies beside the program.
#[cfg(not(target_os = "android"))]
pub(crate) const RGSS3_GUEST: &str = "vkrgss-guest-rgss3";

/// On Android, the name under which the APK installs the guest executable of games of RGSS1
/// and RGSS2 beside the app's library: the installer extracts only files named `lib*.so`.
/// The file is the executable all the same.
#[cfg(target_os = "android")]
pub(crate) const RGSS12_GUEST: &str = "libvkrgss_guest_rgss12.so";

/// On Android, the name under which the APK installs the guest executable of games of RGSS3.
#[cfg(target_os = "android")]
pub(crate) const RGSS3_GUEST: &str = "libvkrgss_guest_rgss3.so";

/// What a missing guest executable `name` lies beside, and how it gets there.
#[cfg(not(target_os = "android"))]
fn missing_guest(name: &str) -> String {
    format!(
        "the directory of the program: it is built with the program (`cargo build -p {name}`) \
         and lies beside it in the release archive"
    )
}

/// What a missing guest executable lies beside on Android, and how it gets there.
#[cfg(target_os = "android")]
fn missing_guest(_: &str) -> String {
    "the app's native library directory: the APK of `cargo xtask apk` installs it there".to_owned()
}

/// The executable the program starts as the guest process of a game of `runtime`: the guest
/// executable of its interpreter in the directory of the program's own file, links resolved
/// as the system resolves them. On Android the program's own file is the app's library,
/// which lies in the app's native library directory (`nativeLibraryDir`), where the APK
/// installs the guest executables too.
pub(crate) fn guest_executable(runtime: Runtime) -> Result<PathBuf> {
    guest_executable_of(&own_file()?, runtime)
}

/// The file of the program.
#[cfg(not(target_os = "android"))]
fn own_file() -> Result<PathBuf> {
    std::env::current_exe().context("cannot tell where the program is")
}

/// The file of the app's library, as the dynamic linker loaded it: the executable of the
/// process is the system's.
#[cfg(target_os = "android")]
fn own_file() -> Result<PathBuf> {
    use std::ffi::{CStr, OsStr};
    use std::os::unix::ffi::OsStrExt;

    let address = own_file as fn() -> Result<PathBuf> as *const libc::c_void;
    // SAFETY: `Dl_info` is plain data, for which all zeros is a valid value.
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    // SAFETY: `info` is valid for writes, and the address is that of code of this library.
    let found = unsafe { libc::dladdr(address, &mut info) };
    if found == 0 || info.dli_fname.is_null() {
        bail!("cannot tell where the app's library is");
    }
    // SAFETY: the dynamic linker's name of a loaded library is a NUL-terminated string that
    // lives as long as the library, which holds this code.
    let name = unsafe { CStr::from_ptr(info.dli_fname) };
    Ok(PathBuf::from(OsStr::from_bytes(name.to_bytes())))
}

/// [`guest_executable`] for the program `program`. A guest executable that is not there is
/// an error that names it and the directory.
fn guest_executable_of(program: &Path, runtime: Runtime) -> Result<PathBuf> {
    let name = match runtime {
        Runtime::Rgss1 | Runtime::Rgss2 => RGSS12_GUEST,
        Runtime::Rgss3 => RGSS3_GUEST,
    };
    let dir = program.parent().unwrap_or(Path::new("/"));
    let guest = dir.join(name);
    if !guest.is_file() {
        bail!(
            "the guest process of a game of {} is the guest executable {name}, and there is \
             no file {} in {}, {}",
            runtime.product(),
            guest.display(),
            dir.display(),
            missing_guest(name)
        );
    }
    Ok(guest)
}

/// The directory in which the program makes the directory of its socket: the user's runtime
/// directory, `XDG_RUNTIME_DIR`, or else the temporary directory.
#[cfg(not(target_os = "android"))]
pub(crate) fn socket_base() -> PathBuf {
    let variable = std::env::var_os("XDG_RUNTIME_DIR").filter(|value| !value.is_empty());
    variable.map_or_else(std::env::temp_dir, PathBuf::from)
}

/// What the program hands a guest process beside the socket: the rest of the guest record.
/// Every path is absolute.
pub(crate) struct Handed<'a> {
    /// The seed of the run.
    pub(crate) seed: u64,
    pub(crate) game_dir: &'a Path,
    /// What a guest reads of the settled options.
    pub(crate) settings: &'a GuestSettings,
    /// The inputs of the diagnostic level of the program.
    pub(crate) diagnostics: Diagnostics,
}

/// Writes the guest record of a guest process that connects to `socket`, as the file
/// `guest.record` beside the socket, and returns the command that starts `program` as that
/// guest process: with the hidden option and the path of the record, and no other argument.
fn guest_command(program: PathBuf, socket: PathBuf, handed: Handed<'_>) -> Result<Command> {
    let path = socket.with_file_name("guest.record");
    let record = GuestRecord {
        role: GuestRole {
            seed: handed.seed,
            socket,
        },
        game_dir: handed.game_dir.to_owned(),
        settings: handed.settings.clone(),
        diagnostics: handed.diagnostics,
    };
    // Written whole before the child exists, which so never sees a part of it.
    record
        .write(&path, VERSION)
        .with_context(|| format!("cannot write the guest record {}", path.display()))?;
    let mut command = Command::new(program);
    command.arg(GUEST_ROLE).arg(path);
    Ok(command)
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
    /// Starts the guest process of a run, for a game of `runtime`, hands it `handed` and the
    /// socket in the guest record, and waits for it to connect; one that states another
    /// version than the program's fails the start. The socket's directory is made in `base`
    /// ([`socket_base`] on Linux, the app's private directory on Android). To be called on
    /// the main thread, or on Android on the thread of `android_main`: the child is ended by
    /// the kernel when the thread that started it ends.
    pub(crate) fn start(runtime: Runtime, base: &Path, handed: Handed<'_>) -> Result<GuestProcess> {
        let program = guest_executable(runtime)?;
        sweep(base);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        let dir = base.join(format!("vkrgss-{}-{stamp:08x}", std::process::id()));
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
        let child = guest_command(program, socket, handed)
            .and_then(|mut command| command.spawn().context("cannot start the guest process"))
            .map_err(remove)?;
        let mut process = GuestProcess {
            child,
            dir,
            listener,
            first: None,
        };
        // An error drops the process, which ends the child.
        process.wait_for_first()?;
        Ok(process)
    }

    /// Waits for the child to connect and attach. A first process that attaches and is not
    /// the child is an error.
    fn wait_for_first(&mut self) -> Result<()> {
        let deadline = Instant::now() + GUEST_CONNECT;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    let (first, _) = attached(stream)?;
                    if first.pid != self.child.id() {
                        bail!(
                            "the first process that attached, {}, is not the guest process {} \
                             the program started",
                            first.pid,
                            self.child.id()
                        );
                    }
                    tracing::info!("guest process {} attached", first.pid);
                    self.first = Some(first);
                    return Ok(());
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error).context("cannot accept the guest process"),
            }
            if let Some(status) = self.child.try_wait()? {
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
                "vkrgss: the game's scripts did not end within {} seconds of being asked to \
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
                    eprintln!("vkrgss: the guest process {}", ended(status));
                }
                false
            }
            (None, _) => {
                eprintln!("vkrgss: the guest process ended without saying how the game went");
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
        kept_load: None,
        outcome: None,
        let_go: Vec::new(),
        dropped: Vec::new(),
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
        eprintln!("vkrgss: cannot relay to the guest process: {error}");
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
    /// A request to load the slot in `refilling`, kept until its new copy attaches.
    kept_load: Option<u8>,
    /// How the game went, as the guest that ended it reported.
    outcome: Option<bool>,
    /// The processes that were ended or told to exit while the relay ran.
    let_go: Vec<u32>,
    /// The kinds of message the running guest sent that only the relay makes, which were
    /// dropped and warned of.
    dropped: Vec<&'static str>,
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
                let read = self.running.as_ref().map(|running| running.pid);
                self.accept_copies();
                // Unless a copy that attached was loaded in place of the guest that could
                // be read: a load that waited for it.
                if self.running.as_ref().map(|running| running.pid) == read {
                    self.guest_message();
                }
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
            if self.kept_load.take() == Some(slot) {
                self.load(slot);
            }
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
        let message = read_message::<GuestToHost>(&mut &running.stream);
        if let Some(slot) = self.kept_load.take() {
            self.tell_host(GuestToHost::StateFailed { slot, load: true });
        }
        match message {
            Ok(Some(GuestToHost::Outcome { ok })) => self.outcome = Some(ok),
            Ok(Some(message)) => match only_made_here(&message) {
                Some(kind) => self.drop_message(kind),
                None => self.tell_host(message),
            },
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

    /// Drops a message of the running guest that only the relay makes, and warns of the
    /// first one of its `kind`.
    fn drop_message(&mut self, kind: &'static str) {
        if !self.dropped.contains(&kind) {
            tracing::warn!(
                "the guest process sent {kind}, which only the program makes; it is dropped"
            );
            self.dropped.push(kind);
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

    /// Loads the state of `slot`: ends the running guest and resumes the parked copy. A load
    /// of the slot whose copy was just resumed waits until that copy has left its new copy of
    /// itself; a second request replaces a waiting one.
    fn load(&mut self, slot: u8) {
        self.kept_load = None;
        if self.refilling == Some(slot) {
            self.kept_load = Some(slot);
            return;
        }
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

/// What `message` is, if it is one that only the relay makes and the running guest has no
/// business sending: an answer about states, or an attachment.
fn only_made_here(message: &GuestToHost) -> Option<&'static str> {
    match message {
        GuestToHost::StateSaved { .. } => Some("a saved answer"),
        GuestToHost::StateLoaded { .. } => Some("a loaded answer"),
        GuestToHost::StateFailed { .. } => Some("a failed answer"),
        GuestToHost::Attach { .. } => Some("an attachment"),
        _ => None,
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

    use vkrgss_host::{HostLink, headless_link};
    use vkrgss_proto::{FreezeRule, InputSnapshot};

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

        guest
            .send(GuestToHost::Freeze {
                rule: FreezeRule::Frame,
            })
            .unwrap();
        guest.send(GuestToHost::FrameReset).unwrap();
        assert_eq!(
            host.recv(),
            Ok(GuestToHost::Freeze {
                rule: FreezeRule::Frame
            })
        );
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

    /// A process that stands in for a guest process which connects by itself: a child of
    /// the test process that connects to the socket in a directory and then waits to be
    /// ended. The test writes and reads at the connection in its place, through the same
    /// open socket, and the kernel names the child as the other side of it. Ended, if it
    /// has not been waited for, when it is dropped.
    struct StandIn {
        pid: libc::pid_t,
        /// The status it ended with, once waited for.
        status: Option<libc::c_int>,
    }

    impl StandIn {
        /// Forks a stand-in that connects to the socket in `dir`, and returns it with the
        /// connection.
        fn connect(dir: &std::path::Path) -> (StandIn, UnixStream) {
            use std::os::fd::FromRawFd;
            use std::os::unix::ffi::OsStrExt;

            // SAFETY: plain integers; the descriptor is checked below.
            let socket = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
            assert!(socket >= 0, "socket: {}", std::io::Error::last_os_error());
            // SAFETY: the descriptor was just made and is owned by the stream from here on.
            let stream = unsafe { UnixStream::from_raw_fd(socket) };
            // SAFETY: `sockaddr_un` is plain data, for which all zeros is a valid value.
            let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
            address.sun_family = libc::AF_UNIX as libc::sa_family_t;
            let path = dir.join("guest.sock");
            let path = path.as_os_str().as_bytes();
            assert!(path.len() < address.sun_path.len(), "{path:?}");
            for (to, from) in address.sun_path.iter_mut().zip(path) {
                *to = *from as libc::c_char;
            }
            let length = std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t;
            #[cfg(target_os = "macos")]
            {
                address.sun_len = length as u8;
            }
            // SAFETY: plain integer.
            let open_max = unsafe { libc::sysconf(libc::_SC_OPEN_MAX) }.clamp(3, 1 << 16);
            let mut pipe = [0; 2];
            // SAFETY: `pipe` is valid for two descriptors.
            assert_eq!(unsafe { libc::pipe(pipe.as_mut_ptr()) }, 0, "pipe");

            // SAFETY: the child calls only async-signal-safe functions, with what was made
            // above, and never returns.
            let pid = unsafe { libc::fork() };
            assert!(pid >= 0, "fork: {}", std::io::Error::last_os_error());
            if pid == 0 {
                // SAFETY: `address` and `connected` are valid for reads of their sizes. The
                // child holds nothing of the test open afterwards, the descriptors that
                // other tests have open included, and waits to be ended.
                unsafe {
                    let connected =
                        u8::from(libc::connect(socket, (&raw const address).cast(), length) == 0);
                    libc::write(pipe[1], (&raw const connected).cast(), 1);
                    for descriptor in 0..open_max as libc::c_int {
                        libc::close(descriptor);
                    }
                    loop {
                        libc::pause();
                    }
                }
            }
            let mut connected = 0_u8;
            // SAFETY: the descriptors of the pipe are open, and `connected` is valid for one
            // byte.
            let read = unsafe {
                libc::close(pipe[1]);
                let read = libc::read(pipe[0], (&raw mut connected).cast(), 1);
                libc::close(pipe[0]);
                read
            };
            let stand_in = StandIn { pid, status: None };
            assert!(read == 1 && connected == 1, "the stand-in did not connect");
            (stand_in, stream)
        }

        fn id(&self) -> u32 {
            self.pid as u32
        }

        /// Whether it has not ended.
        fn running(&mut self) -> bool {
            if self.status.is_some() {
                return false;
            }
            let mut status = 0;
            // SAFETY: looks at a child of this process; `status` is valid for writes.
            let waited = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) };
            if waited == self.pid {
                self.status = Some(status);
            }
            waited == 0
        }

        /// Waits for it to end, and says whether SIGKILL ended it.
        fn killed(&mut self) -> bool {
            let status = *self.status.get_or_insert_with(|| {
                let mut status = 0;
                // SAFETY: waits for a child of this process; `status` is valid for writes.
                let waited = unsafe { libc::waitpid(self.pid, &mut status, 0) };
                assert_eq!(waited, self.pid, "waitpid");
                status
            });
            libc::WIFSIGNALED(status) && libc::WTERMSIG(status) == libc::SIGKILL
        }
    }

    impl Drop for StandIn {
        fn drop(&mut self) {
            if self.status.is_none() {
                // SAFETY: the child has not been waited for, so its number is still its own.
                unsafe { libc::kill(self.pid, libc::SIGKILL) };
                self.killed();
            }
        }
    }

    /// A stand-in that attaches as the state of `slot`, with the connection.
    fn park(dir: &std::path::Path, slot: u8) -> (StandIn, UnixStream) {
        let (process, mut stream) = StandIn::connect(dir);
        hello::write(&mut stream, VERSION).unwrap();
        let attach = GuestToHost::Attach {
            pid: process.id(),
            parked: Some(slot),
        };
        write_message(&mut stream, &attach).unwrap();
        (process, stream)
    }

    /// The attachment of a process `pid` that is not parked, framed.
    fn attachment(pid: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let attach = GuestToHost::Attach { pid, parked: None };
        write_message(&mut bytes, &attach).unwrap();
        bytes
    }

    /// Why what was written to one side of a pair of sockets is refused at the other.
    fn refused(written: &[u8]) -> String {
        use std::io::Write;

        let (mut guest, program) = UnixStream::pair().unwrap();
        guest.write_all(written).unwrap();
        match attached(program) {
            Ok((attached, _)) => panic!("process {} attached", attached.pid),
            Err(error) => format!("{error:#}"),
        }
    }

    /// Scenario "Other version".
    #[test]
    fn a_guest_process_of_another_version_is_refused_naming_both() {
        let mut written = Vec::new();
        hello::write(&mut written, "0.0.0").unwrap();
        written.extend(attachment(7));
        let error = refused(&written);
        assert!(error.contains("0.0.0"), "{error}");
        assert!(error.contains(VERSION), "{error}");
    }

    /// Scenario "No statement".
    #[test]
    fn a_process_that_states_no_version_is_refused_naming_the_version_of_the_program() {
        let error = refused(&attachment(7));
        assert!(error.contains(VERSION), "{error}");
        assert!(error.contains("without stating"), "{error}");
    }

    /// Scenario "Statement and message in one write".
    #[test]
    fn a_statement_and_an_attachment_written_at_once_are_both_read() {
        use std::io::Write;

        let (mut guest, program) = UnixStream::pair().unwrap();
        let mut written = Vec::new();
        hello::write(&mut written, VERSION).unwrap();
        written.extend(attachment(std::process::id()));
        // What a guest sends after it has attached is not taken with them.
        write_message(
            &mut written,
            &GuestToHost::Freeze {
                rule: FreezeRule::Frame,
            },
        )
        .unwrap();
        guest.write_all(&written).unwrap();
        let (attached, parked) = attached(program).unwrap_or_else(|error| panic!("{error:#}"));
        assert_eq!((attached.pid, parked), (std::process::id(), None));
        let next = read_message::<GuestToHost>(&mut &attached.stream).unwrap();
        assert_eq!(
            next,
            Some(GuestToHost::Freeze {
                rule: FreezeRule::Frame
            })
        );
    }

    /// Scenario "Stated number differs", for the attachment itself: the process at the
    /// other side of a pair of sockets is the test process, which states the number of
    /// another process, and 0.
    #[test]
    fn a_process_that_attaches_under_another_number_is_refused() {
        let mut other = idle();
        for stated in [other.id(), 0] {
            let mut written = Vec::new();
            hello::write(&mut written, VERSION).unwrap();
            written.extend(attachment(stated));
            let error = refused(&written);
            let expected = format!(
                "the process {} connected and attached as the process {stated}",
                std::process::id()
            );
            assert!(error.contains(&expected), "{error}");
        }
        assert_eq!(other.try_wait().unwrap(), None);
        other.kill().unwrap();
        other.wait().unwrap();
    }

    /// Scenario "Stated number differs", at the relay: a process that attaches as a state
    /// under the number of another process, or 0, is refused, signals nobody and fills no
    /// slot.
    #[test]
    fn the_relay_refuses_a_copy_that_states_another_number() {
        let dir = tempfile::tempdir().unwrap();
        let mut first = idle();
        let mut other = idle();
        let (host, guest, relay) = relaying(dir.path(), &first);
        for stated in [other.id(), 0] {
            let mut stream = UnixStream::connect(dir.path().join("guest.sock")).unwrap();
            hello::write(&mut stream, VERSION).unwrap();
            let attach = GuestToHost::Attach {
                pid: stated,
                parked: Some(0),
            };
            write_message(&mut stream, &attach).unwrap();
            // The relay closes the connection it refused.
            assert_eq!(read_message::<HostToGuest>(&mut &stream).unwrap(), None);
        }
        host.send(HostToGuest::LoadState { slot: 0 }).unwrap();
        assert_eq!(
            host.recv(),
            Ok(GuestToHost::StateFailed {
                slot: 0,
                load: true
            })
        );
        assert_eq!(other.try_wait().unwrap(), None);
        assert_eq!(first.try_wait().unwrap(), None);
        drop(guest);
        drop(host);
        let relayed = relay.join().unwrap();
        assert!(!relayed.gone.contains(&other.id()), "{:?}", relayed.gone);
        for process in [&mut first, &mut other] {
            process.kill().unwrap();
            process.wait().unwrap();
        }
    }

    /// Scenario "First connection from another process": the test process attaches first,
    /// in place of the child, which is ended with the guest process.
    #[test]
    fn a_first_process_that_is_not_the_child_fails_the_start() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let listener = UnixListener::bind(dir.path().join("guest.sock")).unwrap();
        listener.set_nonblocking(true).unwrap();
        let child = idle();
        let pid = child.id();
        let mut process = GuestProcess {
            child,
            dir: dir.path().join("sockets"),
            listener,
            first: None,
        };
        let mut stream = UnixStream::connect(dir.path().join("guest.sock")).unwrap();
        hello::write(&mut stream, VERSION).unwrap();
        stream.write_all(&attachment(std::process::id())).unwrap();

        let error = format!("{:#}", process.wait_for_first().unwrap_err());
        let expected = format!(
            "the first process that attached, {}, is not the guest process {pid}",
            std::process::id()
        );
        assert!(error.contains(&expected), "{error}");
        drop(process);
        assert!(!exists(pid));
    }

    /// Scenarios "XP and VX" and "VX Ace", and the error of a guest executable that is not
    /// there.
    #[test]
    fn the_guest_executable_lies_beside_the_program() {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("vkrgss");
        let runtimes = [
            (Runtime::Rgss1, RGSS12_GUEST, "RPG Maker XP"),
            (Runtime::Rgss2, RGSS12_GUEST, "RPG Maker VX"),
            (Runtime::Rgss3, RGSS3_GUEST, "RPG Maker VX Ace"),
        ];
        for (runtime, name, product) in runtimes {
            let message = guest_executable_of(&program, runtime)
                .unwrap_err()
                .to_string();
            assert!(message.contains(name), "{message}");
            assert!(
                message.contains(&dir.path().display().to_string()),
                "{message}"
            );
            assert!(message.contains(product), "{message}");
        }

        for name in [RGSS12_GUEST, RGSS3_GUEST] {
            std::fs::write(dir.path().join(name), "").unwrap();
        }
        for (runtime, name, _) in runtimes {
            assert_eq!(
                guest_executable_of(&program, runtime).unwrap(),
                dir.path().join(name)
            );
        }
    }

    /// The program does not start itself as a guest: the running executable, a test binary
    /// here, is asked for the guest executable beside it.
    #[test]
    fn the_guest_executable_of_xp_and_vx_is_not_the_program_itself() {
        let program = std::env::current_exe().unwrap();
        for runtime in [Runtime::Rgss1, Runtime::Rgss2] {
            let expected = program.with_file_name(RGSS12_GUEST);
            match guest_executable(runtime) {
                Ok(guest) => assert_eq!(guest, expected),
                Err(error) => assert!(error.to_string().contains(RGSS12_GUEST), "{error}"),
            }
        }
    }

    /// Scenario "Arguments of the guest process".
    #[test]
    fn the_guest_process_is_started_with_the_option_and_its_record_only() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("guest.sock");
        let settings = GuestSettings {
            preload: vec![dir.path().join("patch.rb")],
            log_p: true,
            ..GuestSettings::default()
        };
        let diagnostics = Diagnostics {
            variable: Some("info".to_owned()),
            verbose: true,
        };
        let handed = Handed {
            seed: 7,
            game_dir: dir.path(),
            settings: &settings,
            diagnostics: diagnostics.clone(),
        };
        let command = guest_command("/opt/vkrgss/guest".into(), socket.clone(), handed).unwrap();
        assert_eq!(command.get_program(), "/opt/vkrgss/guest");
        let record = dir.path().join("guest.record");
        let arguments: Vec<_> = command.get_args().collect();
        assert_eq!(arguments, [GUEST_ROLE.as_ref(), record.as_os_str()]);
        // The file is there before the process is, and holds what was handed.
        let role = GuestRole { seed: 7, socket };
        let written = GuestRecord {
            role,
            game_dir: dir.path().to_owned(),
            settings,
            diagnostics,
        };
        assert_eq!(GuestRecord::read(&record, VERSION).unwrap(), written);
    }

    /// Scenario "Left by a killed run".
    #[test]
    fn the_socket_directories_of_ended_runs_are_removed() {
        let base = tempfile::tempdir().unwrap();
        // A process that has ended and been waited for: its id names nobody.
        let mut ended = idle();
        ended.kill().unwrap();
        ended.wait().unwrap();
        let left = base.path().join(format!("vkrgss-{}-0000abcd", ended.id()));
        let running = base
            .path()
            .join(format!("vkrgss-{}-0000abcd", std::process::id()));
        let other = base.path().join("vkrgss-notes");
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
        let mut first = idle();
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
        let (mut replaced, old) = park(dir.path(), 3);
        guest
            .send(GuestToHost::Freeze {
                rule: FreezeRule::Frame,
            })
            .unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::StateSaved { slot: 3 }));
        assert_eq!(
            host.recv(),
            Ok(GuestToHost::Freeze {
                rule: FreezeRule::Frame
            })
        );
        // A second copy takes the slot, and the connection of the first ends.
        let (copy, parked) = park(dir.path(), 3);
        guest
            .send(GuestToHost::Freeze {
                rule: FreezeRule::Frame,
            })
            .unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::StateSaved { slot: 3 }));
        assert_eq!(
            host.recv(),
            Ok(GuestToHost::Freeze {
                rule: FreezeRule::Frame
            })
        );
        assert_eq!(read_message::<HostToGuest>(&mut &old).unwrap(), None);

        // Loading ends the running guest and resumes the copy, which is the guest now.
        host.send(HostToGuest::LoadState { slot: 3 }).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::StateLoaded { slot: 3 }));
        assert_eq!(first.wait().unwrap().signal(), Some(libc::SIGKILL));
        assert!(guest.recv().is_err());
        let resumed = GuestEnd::over_stream(parked);
        assert_eq!(resumed.recv(), Ok(HostToGuest::Resume));
        // The copy it leaves of itself fills the slot again without a word to the host.
        let (mut again, _refilled) = park(dir.path(), 3);
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
        assert!(again.killed());
        for pid in [first.id(), replaced.id(), again.id()] {
            assert!(relayed.gone.contains(&pid), "{pid} in {:?}", relayed.gone);
        }
        // A stand-in does not exit by itself: the replaced one was not ended.
        assert!(replaced.running());
    }

    /// Scenario "Guest sends a loaded answer", and the other messages that only the relay
    /// makes.
    #[test]
    fn the_relay_drops_answers_about_states_that_the_guest_sends() {
        let dir = tempfile::tempdir().unwrap();
        let mut process = idle();
        let (host, guest, relay) = relaying(dir.path(), &process);
        let made_here = [
            GuestToHost::StateLoaded { slot: 0 },
            GuestToHost::StateSaved { slot: 0 },
            GuestToHost::StateFailed {
                slot: 0,
                load: true,
            },
            GuestToHost::Attach {
                pid: process.id(),
                parked: Some(0),
            },
            GuestToHost::StateLoaded { slot: 0 },
        ];
        for message in made_here {
            guest.send(message).unwrap();
        }
        guest.send(GuestToHost::FrameReset).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::FrameReset));
        host.send(HostToGuest::Tick).unwrap();
        assert_eq!(guest.recv(), Ok(HostToGuest::Tick));

        // A failed save is dropped too, after a request to save as well as without one.
        host.send(HostToGuest::SaveState { slot: 1 }).unwrap();
        assert_eq!(guest.recv(), Ok(HostToGuest::SaveState { slot: 1 }));
        guest
            .send(GuestToHost::StateFailed {
                slot: 1,
                load: false,
            })
            .unwrap();
        guest.send(GuestToHost::FrameReset).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::FrameReset));

        drop(guest);
        assert!(host.recv().is_err());
        drop(host);
        relay.join().unwrap();
        assert_eq!(process.try_wait().unwrap(), None);
        process.kill().unwrap();
        process.wait().unwrap();
    }

    /// Saves the state of slot 0 from the guest at `guest`, the process `first`, and loads
    /// it, which ends `first`. Returns the copy that was resumed and its link.
    fn resumed_from_slot_0(
        dir: &std::path::Path,
        host: &HostLink,
        guest: &GuestEnd,
        first: &mut Child,
    ) -> (StandIn, GuestEnd) {
        let (copy, parked) = park(dir, 0);
        // Its maker goes on after the copy has attached.
        guest.send(GuestToHost::FrameReset).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::StateSaved { slot: 0 }));
        assert_eq!(host.recv(), Ok(GuestToHost::FrameReset));
        host.send(HostToGuest::LoadState { slot: 0 }).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::StateLoaded { slot: 0 }));
        assert_eq!(first.wait().unwrap().signal(), Some(libc::SIGKILL));
        let resumed = GuestEnd::over_stream(parked);
        assert_eq!(resumed.recv(), Ok(HostToGuest::Resume));
        (copy, resumed)
    }

    /// Scenario "Load again at once".
    #[test]
    fn a_load_of_the_slot_being_refilled_waits_for_its_new_copy() {
        let dir = tempfile::tempdir().unwrap();
        let mut first = idle();
        let (host, guest, relay) = relaying(dir.path(), &first);
        let (mut copy, resumed) = resumed_from_slot_0(dir.path(), &host, &guest, &mut first);

        // Asked again before the resumed guest has left its new copy: the tick after the
        // request reaches the guest, so the request has been taken, and is not answered.
        host.send(HostToGuest::LoadState { slot: 0 }).unwrap();
        host.send(HostToGuest::Tick).unwrap();
        assert_eq!(resumed.recv(), Ok(HostToGuest::Tick));
        let (mut again, refilled) = park(dir.path(), 0);
        assert_eq!(host.recv(), Ok(GuestToHost::StateLoaded { slot: 0 }));
        assert!(copy.killed());
        let loaded = GuestEnd::over_stream(refilled);
        assert_eq!(loaded.recv(), Ok(HostToGuest::Resume));
        loaded.send(GuestToHost::FrameReset).unwrap();
        assert_eq!(host.recv(), Ok(GuestToHost::FrameReset));

        drop(host);
        assert!(loaded.recv().is_err());
        drop(loaded);
        let relayed = relay.join().unwrap();
        assert_eq!(relayed.last, again.id());
        assert!(again.running());
    }

    /// A load of the slot being refilled fails, as one of an empty slot, when the resumed
    /// guest goes on without leaving a new copy.
    #[test]
    fn a_waiting_load_fails_when_the_resumed_guest_goes_on_without_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let mut first = idle();
        let (host, guest, relay) = relaying(dir.path(), &first);
        let (mut copy, resumed) = resumed_from_slot_0(dir.path(), &host, &guest, &mut first);

        host.send(HostToGuest::LoadState { slot: 0 }).unwrap();
        host.send(HostToGuest::Tick).unwrap();
        assert_eq!(resumed.recv(), Ok(HostToGuest::Tick));
        resumed.send(GuestToHost::FrameReset).unwrap();
        assert_eq!(
            host.recv(),
            Ok(GuestToHost::StateFailed {
                slot: 0,
                load: true
            })
        );
        assert_eq!(host.recv(), Ok(GuestToHost::FrameReset));

        drop(host);
        assert!(resumed.recv().is_err());
        drop(resumed);
        let relayed = relay.join().unwrap();
        assert_eq!(relayed.last, copy.id());
        assert!(copy.running());
    }

    #[test]
    fn a_signal_is_named() {
        assert_eq!(signal_name(libc::SIGKILL), "SIGKILL");
        assert_eq!(signal_name(libc::SIGSEGV), "SIGSEGV");
        assert_eq!(signal_name(63), "63");
    }
}
