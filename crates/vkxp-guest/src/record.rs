//! The guest record: what a program hands the guest process it starts.
//!
//! The program writes the record into the private directory of its socket before it starts
//! the guest process, and names the file as the value of the hidden option
//! ([`crate::role`]). The guest process reads it and neither the command line of the program
//! nor a configuration file.
//!
//! ```text
//! line 1   ASCII: "vkxp-guest-record <version>" and a line feed
//! rest     the record in the postcard encoding, the encoding of the protocol's messages
//! ```
//!
//! `<version>` is the package version of the program that wrote the record, under the rule
//! of the statement of the version ([`crate::hello`]): graphic ASCII, the whole line
//! [`hello::MAX_LEN`](crate::hello::MAX_LEN) bytes at most. There is no format number: a
//! guest reads only a record of its own version, so the body may change with any release.
//! A path is written as its bytes, never as text.
//!
//! [`GuestRecord::write`] is the program's side and [`GuestRecord::read`] the guest's.

use std::ffi::OsString;
use std::io::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vkxp_vfs::{Encoding, Runtime};

use crate::hello::MAX_LEN;
use crate::role::{GUEST_ROLE, GuestRole};
use crate::settings::GuestSettings;

/// What the first line starts with.
const PREFIX: &[u8] = b"vkxp-guest-record ";

/// The two inputs of the diagnostic level of the program, from which a guest process takes
/// the same level.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagnostics {
    /// The value of the environment variable of the level, as the program read it.
    pub variable: Option<String>,
    /// Whether `--verbose` was given.
    pub verbose: bool,
}

/// What a guest process needs of the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestRecord {
    /// The seed of the run and the socket of the program.
    pub role: GuestRole,
    /// The game directory.
    pub game_dir: PathBuf,
    /// What a guest reads of the settled options.
    pub settings: GuestSettings,
    /// The inputs of the diagnostic level.
    pub diagnostics: Diagnostics,
}

/// A guest record that cannot be used. Each message is the line an executable writes to the
/// error stream after `vkxp: `.
#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    /// The hidden option is not followed by exactly one argument.
    #[error(
        "{GUEST_ROLE} takes the path of a guest record and nothing else; the program gives \
         it to the guest process it starts"
    )]
    Arguments,
    /// The file cannot be read.
    #[error(
        "cannot read the guest record {}: {error}; {GUEST_ROLE} is given by the program to \
         the guest process it starts",
        path.display()
    )]
    Unreadable {
        /// The file that was named.
        path: PathBuf,
        /// What the system said.
        error: std::io::Error,
    },
    /// The file has no first line of the form, or a body that does not decode.
    #[error("{} is not a guest record of vkxp {own}", path.display())]
    Malformed {
        /// The file that was named.
        path: PathBuf,
        /// The version of the executable.
        own: String,
    },
    /// The record was written by a program of another version.
    #[error(
        "the guest record {} is of version {found}, and this executable is of version {own}",
        path.display()
    )]
    OtherVersion {
        /// The file that was named.
        path: PathBuf,
        /// The version the record states.
        found: String,
        /// The version of the executable.
        own: String,
    },
}

/// The body of the record as it is encoded: numbers, booleans, byte strings and lists.
#[derive(Serialize, Deserialize)]
struct Body {
    seed: u64,
    socket: Vec<u8>,
    game_dir: Vec<u8>,
    encoding: Option<u8>,
    runtime: Option<u8>,
    rtps: Vec<(String, Vec<u8>)>,
    builtin_scripts: Option<Vec<u8>>,
    font_dirs: Vec<Vec<u8>>,
    font_own_metrics: bool,
    preload: Vec<Vec<u8>>,
    debug: bool,
    process_functions: bool,
    log_p: bool,
    frame_stats: bool,
    sandbox: Option<bool>,
    variable: Option<String>,
    verbose: bool,
}

fn bytes(path: &Path) -> Vec<u8> {
    path.as_os_str().as_bytes().to_vec()
}

fn path(bytes: Vec<u8>) -> PathBuf {
    PathBuf::from(OsString::from_vec(bytes))
}

fn encoding_number(encoding: Encoding) -> u8 {
    match encoding {
        Encoding::Utf8 => 0,
        Encoding::Gbk => 1,
        Encoding::ShiftJis => 2,
        Encoding::Big5 => 3,
    }
}

fn encoding_of(number: u8) -> Option<Encoding> {
    [
        Encoding::Utf8,
        Encoding::Gbk,
        Encoding::ShiftJis,
        Encoding::Big5,
    ]
    .into_iter()
    .find(|encoding| encoding_number(*encoding) == number)
}

fn runtime_number(runtime: Runtime) -> u8 {
    match runtime {
        Runtime::Rgss1 => 1,
        Runtime::Rgss2 => 2,
        Runtime::Rgss3 => 3,
    }
}

fn runtime_of(number: u8) -> Option<Runtime> {
    Runtime::ALL
        .into_iter()
        .find(|runtime| runtime_number(*runtime) == number)
}

impl Body {
    fn of(record: &GuestRecord) -> Body {
        let GuestRecord {
            role,
            game_dir,
            settings,
            diagnostics,
        } = record;
        Body {
            seed: role.seed,
            socket: bytes(&role.socket),
            game_dir: bytes(game_dir),
            encoding: settings.encoding.map(encoding_number),
            runtime: settings.runtime.map(runtime_number),
            rtps: settings
                .rtps
                .iter()
                .map(|(name, dir)| (name.clone(), bytes(dir)))
                .collect(),
            builtin_scripts: settings.builtin_scripts.as_deref().map(bytes),
            font_dirs: settings.font_dirs.iter().map(|dir| bytes(dir)).collect(),
            font_own_metrics: settings.font_own_metrics,
            preload: settings.preload.iter().map(|file| bytes(file)).collect(),
            debug: settings.debug,
            process_functions: settings.process_functions,
            log_p: settings.log_p,
            frame_stats: settings.frame_stats,
            sandbox: settings.sandbox,
            variable: diagnostics.variable.clone(),
            verbose: diagnostics.verbose,
        }
    }

    /// The record of this body, unless it holds a number that names nothing.
    fn into_record(self) -> Option<GuestRecord> {
        let encoding = match self.encoding {
            Some(number) => Some(encoding_of(number)?),
            None => None,
        };
        let runtime = match self.runtime {
            Some(number) => Some(runtime_of(number)?),
            None => None,
        };
        Some(GuestRecord {
            role: GuestRole {
                seed: self.seed,
                socket: path(self.socket),
            },
            game_dir: path(self.game_dir),
            settings: GuestSettings {
                encoding,
                runtime,
                rtps: self
                    .rtps
                    .into_iter()
                    .map(|(name, dir)| (name, path(dir)))
                    .collect(),
                builtin_scripts: self.builtin_scripts.map(path),
                font_dirs: self.font_dirs.into_iter().map(path).collect(),
                font_own_metrics: self.font_own_metrics,
                preload: self.preload.into_iter().map(path).collect(),
                debug: self.debug,
                process_functions: self.process_functions,
                log_p: self.log_p,
                frame_stats: self.frame_stats,
                sandbox: self.sandbox,
            },
            diagnostics: Diagnostics {
                variable: self.variable,
                verbose: self.verbose,
            },
        })
    }
}

impl GuestRecord {
    /// The bytes of the file of this record, written by a program of `version`.
    fn encode(&self, version: &str) -> std::io::Result<Vec<u8>> {
        let mut file = PREFIX.to_vec();
        file.extend_from_slice(version.as_bytes());
        file.push(b'\n');
        let stated = version.bytes().all(|byte| byte.is_ascii_graphic()) && !version.is_empty();
        if !stated || file.len() > MAX_LEN {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("`{version}` cannot be stated as a version"),
            ));
        }
        postcard::to_extend(&Body::of(self), file).map_err(std::io::Error::other)
    }

    /// Writes the record as the file `path`, which must not exist yet and which only the
    /// user can read, for a guest of `version`: the package version of the program.
    pub fn write(&self, path: &Path, version: &str) -> std::io::Result<()> {
        let bytes = self.encode(version)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(&bytes)
    }

    /// Reads the record in the file `path`, as an executable of the version `own`.
    pub fn read(path: &Path, own: &str) -> Result<GuestRecord, RecordError> {
        let file = std::fs::read(path).map_err(|error| RecordError::Unreadable {
            path: path.to_owned(),
            error,
        })?;
        let malformed = || RecordError::Malformed {
            path: path.to_owned(),
            own: own.to_owned(),
        };
        let end = file.iter().position(|byte| *byte == b'\n');
        let end = end.filter(|end| *end < MAX_LEN).ok_or_else(malformed)?;
        let found = file[..end].strip_prefix(PREFIX).ok_or_else(malformed)?;
        if found.is_empty() || !found.iter().all(u8::is_ascii_graphic) {
            return Err(malformed());
        }
        if found != own.as_bytes() {
            return Err(RecordError::OtherVersion {
                path: path.to_owned(),
                found: String::from_utf8_lossy(found).into_owned(),
                own: own.to_owned(),
            });
        }
        let (body, rest) =
            postcard::take_from_bytes::<Body>(&file[end + 1..]).map_err(|_| malformed())?;
        if !rest.is_empty() {
            return Err(malformed());
        }
        body.into_record().ok_or_else(malformed)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    /// A record with a value for every setting.
    fn full(dir: &Path) -> GuestRecord {
        let not_utf8 = dir.join(OsStr::from_bytes(b"sch\xf6n"));
        GuestRecord {
            role: GuestRole {
                seed: u64::MAX - 7,
                socket: dir.join("guest.sock"),
            },
            game_dir: not_utf8.join("game"),
            settings: GuestSettings {
                encoding: Some(Encoding::ShiftJis),
                runtime: Some(Runtime::Rgss2),
                rtps: [
                    ("Standard".to_owned(), dir.join("rtp/Standard")),
                    ("RPGVX".to_owned(), not_utf8.join("RPGVX")),
                ]
                .into(),
                builtin_scripts: Some(dir.join("rpg_builtin_rgss2.rb")),
                font_dirs: vec![dir.join("fonts"), not_utf8.join("more fonts")],
                font_own_metrics: true,
                preload: vec![dir.join("second.rb"), dir.join("first.rb")],
                debug: true,
                process_functions: true,
                log_p: true,
                frame_stats: true,
                sandbox: Some(false),
            },
            diagnostics: Diagnostics {
                variable: Some("warn,vkxp_host=trace".to_owned()),
                verbose: true,
            },
        }
    }

    /// Scenario "Every setting".
    #[test]
    fn a_record_with_every_setting_is_read_as_it_was_written() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("guest.record");
        let record = full(dir.path());
        record.write(&file, VERSION).unwrap();
        assert_eq!(GuestRecord::read(&file, VERSION).unwrap(), record);
        // The other value of each setting that has two, and the ones that may be absent.
        for encoding in [Encoding::Utf8, Encoding::Gbk, Encoding::Big5] {
            for runtime in Runtime::ALL {
                let file = dir.path().join(format!("{encoding:?}-{runtime:?}"));
                let mut record = record.clone();
                record.settings.encoding = Some(encoding);
                record.settings.runtime = Some(runtime);
                record.settings.sandbox = Some(true);
                record.write(&file, VERSION).unwrap();
                assert_eq!(GuestRecord::read(&file, VERSION).unwrap(), record);
            }
        }
        let plain = GuestRecord {
            settings: GuestSettings::default(),
            diagnostics: Diagnostics::default(),
            ..record
        };
        let file = dir.path().join("plain.record");
        plain.write(&file, VERSION).unwrap();
        assert_eq!(GuestRecord::read(&file, VERSION).unwrap(), plain);
    }

    #[test]
    fn a_record_is_a_file_of_the_user_that_is_not_written_over() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("guest.record");
        let record = full(dir.path());
        record.write(&file, VERSION).unwrap();
        let mode = std::fs::metadata(&file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        let line = format!("vkxp-guest-record {VERSION}\n");
        assert!(std::fs::read(&file).unwrap().starts_with(line.as_bytes()));
        let again = record.write(&file, VERSION).unwrap_err();
        assert_eq!(again.kind(), std::io::ErrorKind::AlreadyExists);
        // A version that cannot be a line of the form is not written.
        for version in ["", "0.2.0 beta", &"9".repeat(MAX_LEN)] {
            let file = dir.path().join("other.record");
            assert!(record.write(&file, version).is_err(), "{version}");
            assert!(!file.exists());
        }
    }

    #[test]
    fn a_record_that_is_absent_cannot_be_read() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("absent.record");
        let error = GuestRecord::read(&absent, VERSION).unwrap_err();
        assert!(matches!(error, RecordError::Unreadable { .. }), "{error:?}");
        let message = error.to_string();
        assert!(
            message.contains("cannot read the guest record"),
            "{message}"
        );
        assert!(message.contains("absent.record"), "{message}");
        assert!(message.contains(GUEST_ROLE), "{message}");
    }

    #[test]
    fn a_file_without_the_first_line_is_not_a_record() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.txt");
        let body = full(dir.path()).encode(VERSION).unwrap();
        let body = &body[PREFIX.len() + VERSION.len() + 1..];
        let no_version = [b"vkxp-guest-record \n".as_slice(), body].concat();
        let long = format!("vkxp-guest-record {}\n", "9".repeat(MAX_LEN));
        for written in [
            b"".as_slice(),
            b"7=/tmp/guest.sock",
            b"vkxp-guest 0.2.0\n",
            body,
            &no_version,
            long.as_bytes(),
        ] {
            std::fs::write(&file, written).unwrap();
            let error = GuestRecord::read(&file, VERSION).unwrap_err();
            assert!(matches!(error, RecordError::Malformed { .. }), "{error:?}");
            assert_eq!(
                error.to_string(),
                format!("{} is not a guest record of vkxp {VERSION}", file.display())
            );
        }
    }

    #[test]
    fn a_body_that_does_not_decode_is_not_a_record() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("guest.record");
        let whole = full(dir.path()).encode(VERSION).unwrap();
        let line = PREFIX.len() + VERSION.len() + 1;
        let cut = &whole[..whole.len() - 3];
        let longer = [whole.as_slice(), b"\0"].concat();
        let noise = [&whole[..line], b"\xff\xff\xff\xff".as_slice()].concat();
        for written in [cut, &longer, &noise, &whole[..line]] {
            std::fs::write(&file, written).unwrap();
            let error = GuestRecord::read(&file, VERSION).unwrap_err();
            assert!(matches!(error, RecordError::Malformed { .. }), "{error:?}");
            assert!(error.to_string().contains("guest.record"), "{error}");
        }
    }

    #[test]
    fn a_record_of_another_version_is_refused_naming_both() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("guest.record");
        full(dir.path()).write(&file, "0.0.0").unwrap();
        let error = GuestRecord::read(&file, VERSION).unwrap_err();
        assert!(
            matches!(error, RecordError::OtherVersion { .. }),
            "{error:?}"
        );
        let message = error.to_string();
        assert!(message.contains("0.0.0"), "{message}");
        assert!(message.contains(VERSION), "{message}");
        assert!(message.contains("guest.record"), "{message}");
    }
}
