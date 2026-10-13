//! The guest role: what makes a process the guest of a program.
//!
//! Side: guest.
//!
//! A game's scripts can run in a process of their own, which a program starts and which
//! plays the game over a socket of that program. This crate is what such a process does
//! apart from running the game, so that an executable other than the program can be a
//! guest too:
//!
//! - [`role`]: the hidden option that names the guest record.
//! - [`record`]: the guest record, which the program writes and the guest process reads,
//!   and [`settings`]: what a guest reads of the settled options of a run.
//! - [`diagnostics`]: the subscriber that writes the diagnostics of an executable.
//! - [`Attached`]: the attachment to the program's socket.
//! - [`hello`]: the statement of the version that every connection of a guest starts with.
//! - [`copier`]: the copies of the process that are its states.
//! - [`sandbox`]: the restriction a guest process puts on itself.
//! - [`signals`]: the signal settings of a guest.
//!
//! The crate links no interpreter and no binding: the executable that links it gives it the
//! function that runs the game, [`play_in_process`] or, for a guest that is a thread of the
//! program, [`play_on_thread`].
//!
//! # Order of a guest process
//!
//! The steps of the executable and of this crate alternate, and no type enforces their
//! order. A guest process does this, on its main thread unless said otherwise:
//!
//! 1. [`signals::die_with_parent`].
//! 2. The executable opens the game's files, enters the game directory, calls
//!    [`signals::block_virtual_timer`] while the main thread is the only one, and starts its
//!    interpreter.
//! 3. [`Attached::connect`], with the package version of the executable.
//! 4. Unless the game may start processes: [`Attached::copier`], which the executable gives
//!    its binding.
//! 5. [`Attached::into_link`], which the game is played over.
//! 6. On the interpreter thread: [`play_in_process`] around the function that runs the game.
//!
//! A guest on a thread does the second step and then, on the interpreter thread,
//! [`play_on_thread`] around the function that runs the game.
//!
//! `unsafe` is denied here and allowed in the modules that call the platform directly.

#![deny(unsafe_code)]

#[allow(unsafe_code)]
pub mod copier;
pub mod diagnostics;
pub mod hello;
pub mod record;
pub mod role;
pub mod sandbox;
pub mod settings;
#[allow(unsafe_code)]
pub mod signals;

use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

use vkxp_proto::GuestToHost;
use vkxp_proto::framing::{FramingError, write_message};
use vkxp_proto::link::GuestEnd;

use crate::copier::ForkCopier;
use crate::role::GuestRole;

/// Why a guest process could not attach to its program.
#[derive(Debug, thiserror::Error)]
pub enum AttachError {
    /// The socket of the role could not be connected to.
    #[error("cannot connect to the socket {} of the parent process", socket.display())]
    Connect {
        /// The socket of the role.
        socket: PathBuf,
        /// What the system said.
        source: std::io::Error,
    },
    /// The statement of the version could not be written.
    #[error("cannot attach to the parent process")]
    Statement(#[source] std::io::Error),
    /// The first message could not be written.
    #[error("cannot attach to the parent process")]
    Attach(#[source] FramingError),
}

/// Opens a connection of a guest process to the program that listens on `socket`, and
/// states `version` on it. Every connection of a guest process and of its copies is opened
/// here; the first message is the caller's to write.
fn open(socket: &Path, version: &str) -> Result<UnixStream, AttachError> {
    let mut stream = UnixStream::connect(socket).map_err(|source| AttachError::Connect {
        socket: socket.to_owned(),
        source,
    })?;
    hello::write(&mut stream, version).map_err(AttachError::Statement)?;
    Ok(stream)
}

/// A guest process that has attached to its program, and its connection.
#[derive(Debug)]
pub struct Attached {
    stream: UnixStream,
    socket: PathBuf,
    version: String,
}

impl Attached {
    /// Connects to the socket of `role`, states `version`, which is the package version of
    /// the executable, and attaches as the guest process the program has started.
    pub fn connect(role: &GuestRole, version: &str) -> Result<Attached, AttachError> {
        let mut stream = open(&role.socket, version)?;
        let attach = GuestToHost::Attach {
            pid: std::process::id(),
            parked: None,
        };
        write_message(&mut stream, &attach).map_err(AttachError::Attach)?;
        Ok(Attached {
            stream,
            socket: role.socket.clone(),
            version: version.to_owned(),
        })
    }

    /// Makes the copier of this connection, for a guest that can be saved and loaded, and
    /// has the kernel take the copies away when they end ([`signals::ignore_children`]).
    /// Not for a game that may start processes, whose children have to be waited for. A
    /// copy calls `restart_timer` on the interpreter thread: what the interpreter of the
    /// executable needs after a `fork`.
    pub fn copier(&self, restart_timer: fn()) -> ForkCopier {
        signals::ignore_children();
        ForkCopier::new(
            self.socket.clone(),
            self.version.clone(),
            &self.stream,
            restart_timer,
        )
    }

    /// The link the game is played over. The thread that plays reads and writes the socket
    /// itself.
    pub fn into_link(self) -> GuestEnd {
        GuestEnd::over_stream(self.stream)
    }
}

/// Plays the game of a guest process. To be called on the interpreter thread: lets
/// SIGVTALRM reach it, confines the thread to writing below the directory `confine`, if
/// there is one, runs `game`, which says whether the game ended without a failure, and ends
/// the process there if it is a copy that was resumed, which has no other thread to go
/// back to.
pub fn play_in_process(confine: Option<&Path>, game: impl FnOnce() -> bool) -> bool {
    signals::unblock_virtual_timer();
    if let Some(game_dir) = confine {
        sandbox::confine(game_dir);
    }
    let ok = game();
    copier::exit_if_copy(ok);
    ok
}

/// Plays the game of a guest that is a thread of the program. To be called on the
/// interpreter thread: lets SIGVTALRM reach it and runs `game`.
pub fn play_on_thread<R>(game: impl FnOnce() -> R) -> R {
    signals::unblock_virtual_timer();
    game()
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    /// The text of the error is the one the program has always written, and the cause is
    /// not part of it: the program writes the causes itself.
    #[test]
    fn a_socket_that_is_not_there_is_an_error_naming_it() {
        let dir = std::env::temp_dir().join(format!("vkxp-guest-absent-{}", std::process::id()));
        let role = GuestRole {
            seed: 0,
            socket: dir.join("guest.sock"),
        };
        let error = Attached::connect(&role, "0.2.0").unwrap_err();
        assert_eq!(
            error.to_string(),
            format!(
                "cannot connect to the socket {} of the parent process",
                role.socket.display()
            )
        );
        assert!(error.source().is_some());
    }
}
