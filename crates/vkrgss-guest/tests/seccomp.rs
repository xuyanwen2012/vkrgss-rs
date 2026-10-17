//! The filter of the system calls, in a process of its own: the test starts this binary
//! again with one test, which installs the filter and then tries what it denies and what it
//! lets through. Scenario "Denied system calls" of the requirement "Confined guest process",
//! for the code in the guest.

#![cfg(any(target_os = "linux", target_os = "android"))]

use std::fs;
use std::io;
use std::net::{TcpListener, UdpSocket};
use std::os::unix::net::{SocketAddr, UnixDatagram, UnixListener, UnixStream};
use std::process::Command;

use vkrgss_guest::seccomp;

/// The variable that makes [`filtered_child`] do its work.
const CHILD: &str = "VKRGSS_SECCOMP_CHILD";

/// What [`filtered_child`] writes when everything held.
const HELD: &str = "the filter held";

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

/// Starts this binary with [`filtered_child`] alone and checks that it ran and passed.
#[test]
fn the_filter_denies_its_list_and_lets_a_game_work() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "filtered_child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .output()
        .expect("the test binary starts again");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}{stderr}");
    assert!(stdout.contains(HELD), "{stdout}{stderr}");
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
