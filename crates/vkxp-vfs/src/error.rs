//! Errors of the game configuration file and of path resolution.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Why a game file could not be found or read.
#[derive(Debug, Error)]
pub enum VfsError {
    /// The game directory holds no `Game.ini` under any letter case.
    #[error("no Game.ini in {}", dir.display())]
    IniNotFound { dir: PathBuf },
    /// The `Game` section of the configuration file lacks a value that is required.
    #[error("Game.ini has no `{key}` value in its [Game] section")]
    MissingKey { key: &'static str },
    /// The configuration file is not valid in the encoding the caller named, or in any of the
    /// encodings tried.
    #[error("Game.ini cannot be decoded as {tried}")]
    IniEncoding { tried: String },
    /// `Library` and `Scripts` of the configuration file name different runtimes, or neither
    /// names one.
    #[error(
        "Game.ini does not tell the runtime of the game: `Library={library}` and \
         `Scripts={scripts}` name different runtimes or none"
    )]
    UndecidedRuntime { library: String, scripts: String },
    /// The configuration file names a run-time package that the caller's table does not contain.
    #[error("run-time package `{name}` is not configured")]
    UnknownRtp { name: String },
    /// The table maps the run-time package to something that is not a directory.
    #[error("run-time package `{name}` points to {}, which is not a directory", dir.display())]
    RtpDirectoryMissing { name: String, dir: PathBuf },
    /// Neither the game directory nor a run-time package holds the file. `path` is the path
    /// exactly as it was requested.
    #[error("file not found: {path}")]
    NotFound { path: String },
    /// The encrypted archive of the game is not a complete RGSSAD version 1 archive.
    #[error("damaged archive {}: {reason}", path.display())]
    DamagedArchive { path: PathBuf, reason: &'static str },
    /// The file or directory exists but could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
}
