//! What a guest reads of the settled options of a run.
//!
//! The program settles them from its command line, its environment and its configuration
//! files. A guest on a thread is given them in memory, and a guest process reads them from
//! the guest record ([`crate::record`]): one type for both.

use std::collections::BTreeMap;
use std::path::PathBuf;

use vkxp_vfs::{Encoding, Runtime};

/// What a guest reads of the settled options of a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuestSettings {
    /// The encoding of `Game.ini`, if one was given.
    pub encoding: Option<Encoding>,
    /// The runtime of the game, if one was given; otherwise `Game.ini` tells it.
    pub runtime: Option<Runtime>,
    /// The directory of each run-time package, by its name.
    pub rtps: BTreeMap<String, PathBuf>,
    /// The built-in scripts file, instead of the default location.
    pub builtin_scripts: Option<PathBuf>,
    /// Font directories, in the order given.
    pub font_dirs: Vec<PathBuf>,
    /// Whether a font that replaces another keeps its own line box.
    pub font_own_metrics: bool,
    /// Preload scripts, in the order given.
    pub preload: Vec<PathBuf>,
    /// Whether the game runs in debug mode.
    pub debug: bool,
    /// Whether the functions that start processes do so.
    pub process_functions: bool,
    /// Whether `p` is log-only.
    pub log_p: bool,
    /// Whether the frame time statistics are reported.
    pub frame_stats: bool,
    /// Whether a guest process confines itself, if that was given.
    pub sandbox: Option<bool>,
}
