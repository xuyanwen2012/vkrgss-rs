//! The system calls a guest process denies itself before the first script runs.
//!
//! This is platform code for Linux and Android: a seccomp-BPF filter, written by hand with
//! the constants of `libc` (no crate stands between: the program is a dozen lines and the
//! one crate that would write it, `seccompiler`, brings a model of rules this filter does
//! not need). The filter answers `EPERM` to a fixed list of system calls no game needs
//! ([`DENIED`]), to `socket` of every family but `AF_UNIX`, to `clone` with a flag of
//! [`NAMESPACES`], to the x32 system calls on x86_64 and to every system call of another
//! architecture; it answers `ENOSYS` to `clone3`, whose flags live in memory the filter
//! cannot read, so that the C library falls back to `clone`; it allows the rest. It binds
//! every thread of the process (`SECCOMP_FILTER_FLAG_TSYNC`) and what they start later,
//! the copies of the guest process included, and it cannot be lifted.

use std::io;

use libc::{c_long, sock_filter, sock_fprog};

/// The architecture of the system calls the filter lets through, as `AUDIT_ARCH_*` of
/// `linux/audit.h` gives it.
#[cfg(target_arch = "x86_64")]
const ARCH: u32 = 0xc000_003e;
#[cfg(target_arch = "aarch64")]
const ARCH: u32 = 0xc000_00b7;

/// The bit of the system calls of the x32 ABI, `__X32_SYSCALL_BIT`.
#[cfg(target_arch = "x86_64")]
const X32_SYSCALL_BIT: u32 = 0x4000_0000;

/// `kexec_file_load`, which `libc` names for Linux on aarch64 but not for Android, where the
/// number is the same.
#[cfg(target_arch = "aarch64")]
const SYS_KEXEC_FILE_LOAD: c_long = 294;
#[cfg(target_arch = "x86_64")]
const SYS_KEXEC_FILE_LOAD: c_long = libc::SYS_kexec_file_load;

/// The system calls that are denied whatever their arguments: starting programs, reading
/// and writing other processes, loading code into the kernel, the kernel's key store,
/// mounts, namespaces, `io_uring` (which opens sockets of its own), and the calls that only
/// an administrator makes.
pub const DENIED: [c_long; 28] = [
    libc::SYS_execve,
    libc::SYS_execveat,
    libc::SYS_ptrace,
    libc::SYS_process_vm_readv,
    libc::SYS_process_vm_writev,
    libc::SYS_bpf,
    libc::SYS_perf_event_open,
    libc::SYS_mount,
    libc::SYS_umount2,
    libc::SYS_pivot_root,
    libc::SYS_chroot,
    libc::SYS_init_module,
    libc::SYS_finit_module,
    libc::SYS_delete_module,
    libc::SYS_kexec_load,
    SYS_KEXEC_FILE_LOAD,
    libc::SYS_keyctl,
    libc::SYS_add_key,
    libc::SYS_request_key,
    libc::SYS_userfaultfd,
    libc::SYS_unshare,
    libc::SYS_setns,
    libc::SYS_reboot,
    libc::SYS_swapon,
    libc::SYS_swapoff,
    libc::SYS_io_uring_setup,
    libc::SYS_io_uring_enter,
    libc::SYS_io_uring_register,
];

/// The flags of `clone` that make a new namespace, which `clone` is denied with.
pub const NAMESPACES: u32 = (libc::CLONE_NEWNS
    | libc::CLONE_NEWUSER
    | libc::CLONE_NEWPID
    | libc::CLONE_NEWNET
    | libc::CLONE_NEWIPC
    | libc::CLONE_NEWUTS
    | libc::CLONE_NEWCGROUP) as u32;

// The instruction classes and fields of classic BPF, from `linux/bpf_common.h`, which
// `libc` gives for Linux but not for Android.
const BPF_LD: u16 = 0x00;
const BPF_JMP: u16 = 0x05;
const BPF_RET: u16 = 0x06;
const BPF_W: u16 = 0x00;
const BPF_ABS: u16 = 0x20;
const BPF_JEQ: u16 = 0x10;
const BPF_JSET: u16 = 0x40;
#[cfg(target_arch = "x86_64")]
const BPF_JGE: u16 = 0x30;
const BPF_K: u16 = 0x00;

/// The offsets of the fields of `struct seccomp_data` the filter reads.
const NR_OFFSET: u32 = 0;
const ARCH_OFFSET: u32 = 4;
/// The lower half of the first argument, on a little-endian machine; `socket` reads its
/// family as an `int`, and the namespace flags of `clone` are in that half.
const FIRST_ARGUMENT_OFFSET: u32 = 16;

const RET_DENY: u32 = libc::SECCOMP_RET_ERRNO | libc::EPERM as u32;
const RET_ALLOW: u32 = libc::SECCOMP_RET_ALLOW;
const RET_NO_SYSTEM_CALL: u32 = libc::SECCOMP_RET_ERRNO | libc::ENOSYS as u32;

fn statement(code: u16, k: u32) -> sock_filter {
    sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}

fn jump(code: u16, k: u32, jt: u8, jf: u8) -> sock_filter {
    sock_filter { code, jt, jf, k }
}

/// The instructions of the filter.
fn program() -> Vec<sock_filter> {
    let mut program = vec![
        statement(BPF_LD | BPF_W | BPF_ABS, ARCH_OFFSET),
        jump(BPF_JMP | BPF_JEQ | BPF_K, ARCH, 1, 0),
        statement(BPF_RET | BPF_K, RET_DENY),
        statement(BPF_LD | BPF_W | BPF_ABS, NR_OFFSET),
    ];
    #[cfg(target_arch = "x86_64")]
    program.extend([
        jump(BPF_JMP | BPF_JGE | BPF_K, X32_SYSCALL_BIT, 0, 1),
        statement(BPF_RET | BPF_K, RET_DENY),
    ]);
    for number in DENIED {
        program.extend([
            jump(BPF_JMP | BPF_JEQ | BPF_K, number as u32, 0, 1),
            statement(BPF_RET | BPF_K, RET_DENY),
        ]);
    }
    program.extend([
        jump(BPF_JMP | BPF_JEQ | BPF_K, libc::SYS_clone3 as u32, 0, 1),
        statement(BPF_RET | BPF_K, RET_NO_SYSTEM_CALL),
        jump(BPF_JMP | BPF_JEQ | BPF_K, libc::SYS_clone as u32, 0, 4),
        statement(BPF_LD | BPF_W | BPF_ABS, FIRST_ARGUMENT_OFFSET),
        jump(BPF_JMP | BPF_JSET | BPF_K, NAMESPACES, 0, 1),
        statement(BPF_RET | BPF_K, RET_DENY),
        statement(BPF_RET | BPF_K, RET_ALLOW),
        jump(BPF_JMP | BPF_JEQ | BPF_K, libc::SYS_socket as u32, 0, 3),
        statement(BPF_LD | BPF_W | BPF_ABS, FIRST_ARGUMENT_OFFSET),
        jump(BPF_JMP | BPF_JEQ | BPF_K, libc::AF_UNIX as u32, 1, 0),
        statement(BPF_RET | BPF_K, RET_DENY),
        statement(BPF_RET | BPF_K, RET_ALLOW),
    ]);
    program
}

/// Puts the filter on every thread of the process, and reports once whether that could be
/// done. Without seccomp, or when a call fails, the guest goes on unfiltered.
pub fn filter() {
    match install() {
        Ok(()) => tracing::info!("the guest process filters its system calls with seccomp"),
        Err(error) => {
            tracing::warn!("the guest process does not filter its system calls: {error}");
        }
    }
}

/// Installs the filter, as [`filter`] does, without the report. `PR_SET_NO_NEW_PRIVS` is
/// set here unless Landlock has set it already.
pub fn install() -> io::Result<()> {
    let program = program();
    let fprog = sock_fprog {
        len: program.len() as u16,
        filter: program.as_ptr().cast_mut(),
    };
    // SAFETY: the calls take no pointer but the one to `fprog`, which points at `program`;
    // both live until the kernel has copied the filter.
    unsafe {
        if libc::prctl(libc::PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0) != 1
            && libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0
        {
            return Err(io::Error::last_os_error());
        }
        let result = libc::syscall(
            libc::SYS_seccomp,
            libc::SECCOMP_SET_MODE_FILTER,
            libc::SECCOMP_FILTER_FLAG_TSYNC,
            &fprog as *const sock_fprog,
        );
        match result {
            0 => Ok(()),
            // With `TSYNC` a positive result is a thread that could not take the filter.
            thread if thread > 0 => Err(io::Error::other(format!(
                "the thread {thread} cannot take the filter"
            ))),
            _ => Err(io::Error::last_os_error()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every jump of the filter lands inside it, and every path ends in a return.
    #[test]
    fn the_program_ends_in_returns() {
        let program = program();
        assert!(program.len() < usize::from(u16::MAX));
        for (index, instruction) in program.iter().enumerate() {
            if instruction.code & 0x07 == BPF_JMP {
                let far = index + 1 + usize::from(instruction.jt.max(instruction.jf));
                assert!(far < program.len(), "the jump at {index}");
            }
        }
        let last = program.last().unwrap();
        assert_eq!((last.code, last.k), (BPF_RET | BPF_K, RET_ALLOW));
    }
}
