//! Path resolution, RTP lookup, encrypted archive.
//!
//! Side: guest.
//!
//! A game names its files the way Windows lets it: any letter case, either slash, usually
//! without an extension, and expecting a run-time package (RTP) to supply whatever the game
//! directory lacks. This crate turns such names into real files on a case-sensitive file
//! system:
//!
//! - [`GameIni`] reads `Game.ini`: the script archive, the window title and the names of the
//!   run-time packages. Its text is UTF-8 or one of the legacy encodings in [`Encoding`].
//! - [`Runtime`] is the RGSS a game was made for; [`Runtime::detect`] tells it from the
//!   configuration file. The names of the run-time packages and of the archive follow it.
//! - [`LibraryVersion`] is the version of a runtime DLL, read from the bytes of the file,
//!   because the name of the file does not tell it.
//! - [`resolve_rtps`] maps those names to directories through a table the caller supplies,
//!   standing in for the Windows registry.
//! - [`GameFiles`] resolves script-style paths against the game directory and then the
//!   run-time packages, trying the extensions of the requested [`FileKind`]. A path never
//!   resolves to a file outside the directory it is searched in, whether through `..` or
//!   through a symbolic link.
//!
//! [`GameFiles::open`] also reads the encrypted archive of the game (`Game.rgssad` or
//! `Game.rgss2a`, RGSSAD version 1, or `Game.rgss3a`, RGSSAD version 3), whose entries come
//! before loose files. [`GameFiles::data_rgss3`] finds a data file of a game of RGSS3 by the
//! rules of its runtime, which differ from those of the other lookups. Nothing here writes files: saves go through Ruby's own file API.
//!
//! # Example
//!
//! ```
//! use std::collections::BTreeMap;
//! use vkrgss_vfs::{FileKind, GameFiles, GameIni, Runtime, VfsError, resolve_rtps};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // A game that takes its title picture from the run-time package.
//! let game = tempfile::tempdir()?;
//! let rtp = tempfile::tempdir()?;
//! std::fs::write(
//!     game.path().join("Game.ini"),
//!     "[Game]\r\nScripts=Data\\Scripts.rxdata\r\nTitle=Example\r\nRTP1=Standard\r\n",
//! )?;
//! std::fs::create_dir_all(rtp.path().join("Graphics/Titles"))?;
//! std::fs::write(rtp.path().join("Graphics/Titles/001-Title01.jpg"), b"picture")?;
//!
//! let ini = GameIni::read(game.path(), None)?;
//! assert_eq!(ini.title, "Example");
//!
//! let table = BTreeMap::from([("Standard".to_owned(), rtp.path().to_owned())]);
//! let runtime = Runtime::detect(&ini)?;
//! assert_eq!(runtime, Runtime::Rgss1);
//! let files = GameFiles::new(game.path(), resolve_rtps(ini.rtp_names(runtime), &table)?);
//!
//! // Wrong case, a backslash and no extension, as scripts write it.
//! let path = files.resolve(FileKind::Image, "graphics\\titles/001-title01")?;
//! assert_eq!(path, rtp.path().join("Graphics/Titles/001-Title01.jpg"));
//! assert_eq!(files.read(FileKind::Image, "Graphics/Titles/001-Title01")?, b"picture");
//!
//! // Data files never come from a run-time package, and the error carries the request.
//! let error = files.resolve(FileKind::Data, &ini.scripts).unwrap_err();
//! assert!(matches!(error, VfsError::NotFound { path } if path == "Data\\Scripts.rxdata"));
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod archive;
mod dir;
mod error;
mod files;
mod ini;
mod library_version;
mod rtp;
mod runtime;

#[doc(hidden)]
pub use archive::read_both_versions;
pub use error::VfsError;
pub use files::{DataFile, FileKind, GameFiles};
pub use ini::{Encoding, GameIni};
pub use library_version::LibraryVersion;
pub use rtp::resolve_rtps;
pub use runtime::Runtime;
