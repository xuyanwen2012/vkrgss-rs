//! The filter of the system calls, in a process of its own: the test starts this binary
//! again with one test, which installs the filter and then tries what it denies and what it
//! lets through. Scenario "Denied system calls" of the requirement "Confined guest process",
//! for the code in the guest, and the scenarios of the requirement "Signals of a confined
//! guest process", with Landlock too.

#![cfg(any(target_os = "linux", target_os = "android"))]

use std::fs;
use std::io;
use std::net::{TcpListener, UdpSocket};
use std::os::unix::net::{SocketAddr, UnixDatagram, UnixListener, UnixStream};
use std::os::unix::thread::JoinHandleExt;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use vkrgss_guest::{sandbox, seccomp};

/// The variable that makes [`filtered_child`] do its work.
const CHILD: &str = "VKRGSS_SECCOMP_CHILD";

/// What [`filtered_child`] writes when everything held.
const HELD: &str = "the filter held";

/// The variable that makes [`confined_child`] do its work.
const CONFINED_CHILD: &str = "VKRGSS_CONFINED_CHILD";

/// What [`confined_child`] writes when everything held.
const SIGNALS_HELD: &str = "the signals held";

/// The error number of the last call.
fn errno() -> i32 {
    io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// A system call with every argument -1, which no denied call takes as a request that does
/// anything, and its error number.
fn raw_call(number: libc::c_long) -> (libc::c_long, i32) {
    // SAFETY: with -1 as every argument the calls fail before they read memory, or are
    // answered by the filter.
    let result = unsafe { libc::syscall(number, -1, -1, -1, -1, -1, -1) };
    (result, errno())
}

fn assert_denied(error: io::Error, what: &str) {
    assert_eq!(error.raw_os_error(), Some(libc::EPERM), "{what}: {error}");
}

/// `clone` with `flags` and `CLONE_THREAD` but not `CLONE_SIGHAND`, which the kernel refuses
/// with `EINVAL` before it makes anything, and its error number.
fn refused_clone(flags: libc::c_int) -> (libc::c_long, i32) {
    let flags = libc::c_long::from(flags | libc::CLONE_THREAD);
    // SAFETY: the flags are invalid, so no process or thread is made.
    let result = unsafe { libc::syscall(libc::SYS_clone, flags, 0, 0, 0, 0) };
    (result, errno())
}

fn socket(family: libc::c_int) -> (libc::c_int, i32) {
    // SAFETY: plain integers.
    let fd = unsafe { libc::socket(family, libc::SOCK_STREAM, 0) };
    (fd, errno())
}

/// Starts this binary with the test `name` alone, with `variable` set, checks that it ran
/// and passed and that it wrote `held`, and passes on what it wrote.
fn run_alone(name: &str, variable: &str, held: &str) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(variable, "1")
        .output()
        .expect("the test binary starts again");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    print!("{stdout}");
    assert!(output.status.success(), "{stdout}{stderr}");
    assert!(stdout.contains(held), "{stdout}{stderr}");
}

/// Starts this binary with [`filtered_child`] alone and checks that it ran and passed.
#[test]
fn the_filter_denies_its_list_and_lets_a_game_work() {
    run_alone("filtered_child", CHILD, HELD);
}

/// Starts this binary with [`confined_child`] alone and checks that it ran and passed.
#[test]
fn a_confined_process_signals_its_own_threads_and_no_other_process() {
    run_alone("confined_child", CONFINED_CHILD, SIGNALS_HELD);
}

/// The work of the process that [`the_filter_denies_its_list_and_lets_a_game_work`] starts;
/// does nothing in any other.
#[test]
fn filtered_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    seccomp::install().expect("the filter is installed");

    // Starting a program.
    let error = Command::new("true").status().unwrap_err();
    assert_denied(error, "execve");
    // Every call of the list, whatever its arguments.
    for number in seccomp::DENIED {
        assert_eq!(raw_call(number), (-1, libc::EPERM), "system call {number}");
    }
    // `clone3`, which the C library replaces with `clone`, and `clone` with a namespace.
    assert_eq!(raw_call(libc::SYS_clone3), (-1, libc::ENOSYS), "clone3");
    for flag in [
        libc::CLONE_NEWNS,
        libc::CLONE_NEWUSER,
        libc::CLONE_NEWPID,
        libc::CLONE_NEWNET,
        libc::CLONE_NEWIPC,
        libc::CLONE_NEWUTS,
        libc::CLONE_NEWCGROUP,
    ] {
        assert_eq!(
            refused_clone(flag),
            (-1, libc::EPERM),
            "clone flag {flag:#x}"
        );
    }
    assert_eq!(
        refused_clone(0),
        (-1, libc::EINVAL),
        "clone reaches the kernel"
    );
    #[cfg(target_arch = "x86_64")]
    assert_eq!(
        raw_call(libc::SYS_getpid | 0x4000_0000),
        (-1, libc::EPERM),
        "an x32 system call"
    );
    // Sockets of other families than AF_UNIX.
    assert_denied(TcpListener::bind("127.0.0.1:0").unwrap_err(), "AF_INET");
    assert_denied(UdpSocket::bind("[::1]:0").unwrap_err(), "AF_INET6");
    for family in [
        libc::AF_INET,
        libc::AF_INET6,
        libc::AF_NETLINK,
        libc::AF_PACKET,
    ] {
        assert_eq!(socket(family), (-1, libc::EPERM), "family {family}");
    }

    // A Unix socket, by its address and as a pair. On Android SELinux does not let the
    // shell make a socket file in /data/local/tmp, where the test runs (`EACCES`, not the
    // filter's `EPERM`), so there the socket takes an abstract name.
    #[cfg(not(target_os = "android"))]
    let address = SocketAddr::from_pathname(dir.path().join("guest.sock")).unwrap();
    #[cfg(target_os = "android")]
    let address = {
        use std::os::android::net::SocketAddrExt;
        let name = format!("vkrgss-seccomp-{}", std::process::id());
        SocketAddr::from_abstract_name(name).unwrap()
    };
    let listener = UnixListener::bind_addr(&address).expect("AF_UNIX is allowed");
    let _client = UnixStream::connect_addr(&address).expect("a connection by the address");
    drop(listener.accept().expect("the connection is accepted"));
    UnixDatagram::unbound().expect("an unbound datagram socket");
    UnixStream::pair().expect("socketpair is allowed");
    // File I/O.
    let file = dir.path().join("Save1.rxdata");
    fs::write(dir.path().join("one.tmp"), "saved").unwrap();
    fs::rename(dir.path().join("one.tmp"), &file).unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "saved");
    fs::create_dir(dir.path().join("saves")).unwrap();
    fs::remove_file(&file).unwrap();
    // Threads.
    assert_eq!(std::thread::spawn(|| 6 * 7).join().unwrap(), 42);
    // A copy, which keeps the filter.
    // SAFETY: the child calls only async-signal-safe functions before it exits.
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork: {}", io::Error::last_os_error());
    if pid == 0 {
        let (result, errno) = raw_call(libc::SYS_execve);
        let code = if result == -1 && errno == libc::EPERM {
            7
        } else {
            1
        };
        // SAFETY: ends the copy without running anything of the parent.
        unsafe { libc::_exit(code) };
    }
    let mut status = 0;
    // SAFETY: waits for the copy started above.
    assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    assert!(libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 7);
    println!("{HELD}");
}

/// A process of the same user that [`confined_child`] tries to signal: a child that exits by
/// itself, with status 0, when the end of its pipe is closed.
struct Other {
    pid: libc::pid_t,
    pipe: libc::c_int,
}

impl Other {
    fn start() -> Other {
        let mut pipe = [0; 2];
        // SAFETY: `pipe` is valid for two descriptors.
        assert_eq!(unsafe { libc::pipe(pipe.as_mut_ptr()) }, 0, "pipe");
        // SAFETY: the child calls only async-signal-safe functions before it exits.
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork: {}", io::Error::last_os_error());
        if pid == 0 {
            let mut byte = 0_u8;
            // SAFETY: closes the end of the parent, waits for the end of the pipe and exits
            // without running anything of the parent.
            unsafe {
                libc::close(pipe[1]);
                while libc::read(pipe[0], (&raw mut byte).cast(), 1) != 0 {}
                libc::_exit(0);
            }
        }
        // SAFETY: the child's end, which this process does not use.
        unsafe { libc::close(pipe[0]) };
        Other { pid, pipe: pipe[1] }
    }

    /// Whether it still runs.
    fn running(&self) -> bool {
        let mut status = 0;
        // SAFETY: looks at a child of this process; `status` is valid for writes.
        unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) == 0 }
    }

    /// Lets it end, and says whether it exited by itself with status 0.
    fn finish(self) -> bool {
        let mut status = 0;
        // SAFETY: closes the pipe this process holds, and waits for a child of it.
        let waited = unsafe {
            libc::close(self.pipe);
            libc::waitpid(self.pid, &mut status, 0)
        };
        waited == self.pid && libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0
    }
}

/// How many times [`count_signal`] ran.
static HANDLED: AtomicUsize = AtomicUsize::new(0);

extern "C" fn count_signal(_: libc::c_int) {
    HANDLED.fetch_add(1, Ordering::SeqCst);
}

/// Waits until [`HANDLED`] is `count`, for a second at most, and says whether it is.
fn handled(count: usize) -> bool {
    let deadline = Instant::now() + Duration::from_secs(1);
    while HANDLED.load(Ordering::SeqCst) < count && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    HANDLED.load(Ordering::SeqCst) == count
}

/// The Landlock ABI of the running kernel, or why there is none.
fn landlock_abi() -> Result<libc::c_long, io::Error> {
    // SAFETY: `LANDLOCK_CREATE_RULESET_VERSION` (1) with no attribute only asks for the
    // version.
    let abi = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<libc::c_void>(),
            0,
            1,
        )
    };
    if abi < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(abi)
    }
}

/// The work of the process that
/// [`a_confined_process_signals_its_own_threads_and_no_other_process`] starts; does nothing in
/// any other. As in a guest process, a thread other than the main thread confines itself
/// with Landlock and installs the filter, which binds every thread, and then signals.
#[test]
fn confined_child() {
    if std::env::var_os(CONFINED_CHILD).is_none() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let other = Other::start();
    // SAFETY: an all-zero `sigaction` is valid; the handler only adds to an atomic, and
    // `sa_mask` is emptied before the call reads it.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = count_signal as extern "C" fn(libc::c_int) as libc::sighandler_t;
        libc::sigemptyset(&mut action.sa_mask);
        assert_eq!(
            libc::sigaction(libc::SIGUSR1, &action, std::ptr::null_mut()),
            0
        );
    }
    // SAFETY: takes nothing.
    let main_thread = unsafe { libc::pthread_self() } as usize;
    let abi = landlock_abi();

    let other_pid = other.pid;
    std::thread::spawn(move || {
        sandbox::confine(dir.path());
        let filtered = seccomp::install();

        // Scenario "Signal to another process".
        match &filtered {
            Ok(()) => {
                // SAFETY: plain integers. Without the filter SIGKILL ends a child of this
                // process, and signal 0 sends nothing.
                let killed = unsafe { libc::kill(other_pid, libc::SIGKILL) };
                assert_eq!(
                    (killed, errno()),
                    (-1, libc::EPERM),
                    "kill of another process"
                );
                // SAFETY: as above.
                let group = unsafe { libc::kill(0, 0) };
                assert_eq!((group, errno()), (-1, libc::EPERM), "kill of the group");
            }
            Err(error) => println!("skipped the filter: seccomp cannot be used: {error}"),
        }
        match abi {
            // `tgkill`, which the filter allows, to another process: the signal scope.
            Ok(abi) if abi >= 6 => {
                // SAFETY: plain integers; without the scope SIGKILL ends a child of this
                // process.
                let sent =
                    unsafe { libc::syscall(libc::SYS_tgkill, other_pid, other_pid, libc::SIGKILL) };
                assert_eq!(
                    (sent, errno()),
                    (-1, libc::EPERM),
                    "tgkill of another process"
                );
            }
            Ok(abi) => println!("skipped the signal scope: the kernel has Landlock ABI {abi}"),
            Err(error) => println!("skipped the signal scope: no Landlock: {error}"),
        }

        // Scenario "Own threads": to the calling thread, to a thread it starts, and to the
        // main thread, which is not confined.
        // SAFETY: SIGUSR1 has a handler.
        assert_eq!(unsafe { libc::raise(libc::SIGUSR1) }, 0, "raise");
        assert!(handled(1), "raise");
        let (started, wait) = std::sync::mpsc::channel::<()>();
        let (stop, stopped) = std::sync::mpsc::channel::<()>();
        let thread = std::thread::spawn(move || {
            started.send(()).unwrap();
            let _ = stopped.recv();
        });
        wait.recv().unwrap();
        for (target, count) in [(thread.as_pthread_t() as usize, 2), (main_thread, 3)] {
            // SAFETY: both threads run until this one is done with them.
            let sent = unsafe { libc::pthread_kill(target as libc::pthread_t, libc::SIGUSR1) };
            assert_eq!(sent, 0, "pthread_kill");
            assert!(handled(count), "pthread_kill to thread {count}");
        }
        stop.send(()).unwrap();
        thread.join().unwrap();
    })
    .join()
    .unwrap();

    assert!(other.running(), "the other process was ended");
    assert!(other.finish(), "the other process did not exit by itself");
    println!("{SIGNALS_HELD}");
}
