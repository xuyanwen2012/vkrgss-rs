//! The statement of the version: the first bytes of every connection a guest process opens.
//!
//! A guest process and the program that started it are two executables of one release. So
//! that one of another release is refused by name, the guest states the version of its
//! executable before its first message: one line of ASCII, `vkxp-guest <version>` and a
//! line feed, [`MAX_LEN`] bytes at most. It is a line outside the messages of the protocol,
//! which carry the version of their encoding themselves.
//!
//! [`write`] is the guest's side and [`read`] the program's.

use std::io::{Read, Write};

/// The most bytes a statement has, the line feed included.
pub const MAX_LEN: usize = 64;

/// What the line starts with.
const PREFIX: &[u8] = b"vkxp-guest ";

/// A connection whose statement of the version is not the one the program expects.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HelloError {
    /// The connection does not start with a statement: the line is missing, too long or
    /// not of the form.
    #[error(
        "a process connected without stating the version of its executable; the program is \
         of version {own}"
    )]
    Missing {
        /// The version of the program.
        own: String,
    },
    /// The statement is of another version than the program's.
    #[error("a guest process of version {stated} connected to the program of version {own}")]
    Other {
        /// The version the guest stated.
        stated: String,
        /// The version of the program.
        own: String,
    },
}

/// States `version`, the version of the executable of this guest process, on a connection
/// it has just opened.
pub fn write(writer: &mut impl Write, version: &str) -> std::io::Result<()> {
    let mut line = PREFIX.to_vec();
    line.extend_from_slice(version.as_bytes());
    line.push(b'\n');
    let stated = version.bytes().all(|byte| byte.is_ascii_graphic()) && !version.is_empty();
    if !stated || line.len() > MAX_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("`{version}` cannot be stated as a version"),
        ));
    }
    writer.write_all(&line)
}

/// Reads the statement a connection starts with and checks that it is of `own`, the version
/// of the program. Reads byte by byte and nothing after the line feed: what follows is the
/// first message.
pub fn read(reader: &mut impl Read, own: &str) -> Result<(), HelloError> {
    let missing = || HelloError::Missing {
        own: own.to_owned(),
    };
    let mut line = Vec::new();
    loop {
        let mut byte = [0];
        reader.read_exact(&mut byte).map_err(|_| missing())?;
        if byte[0] == b'\n' {
            break;
        }
        line.push(byte[0]);
        // Something else than a statement is found out at its first byte.
        let of_the_form = PREFIX.starts_with(&line) || line.starts_with(PREFIX);
        if !of_the_form || line.len() >= MAX_LEN {
            return Err(missing());
        }
    }
    let stated = line.strip_prefix(PREFIX).ok_or_else(missing)?;
    if stated.is_empty() || !stated.iter().all(u8::is_ascii_graphic) {
        return Err(missing());
    }
    if stated != own.as_bytes() {
        return Err(HelloError::Other {
            stated: String::from_utf8_lossy(stated).into_owned(),
            own: own.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixStream;

    use super::*;

    #[test]
    fn a_statement_that_is_written_is_read() {
        let (mut guest, program) = UnixStream::pair().unwrap();
        write(&mut guest, "0.2.0").unwrap();
        // What follows the line is left where it is.
        guest.write_all(b"next").unwrap();
        assert_eq!(read(&mut &program, "0.2.0"), Ok(()));
        let mut rest = [0; 4];
        (&program).read_exact(&mut rest).unwrap();
        assert_eq!(&rest, b"next");
    }

    #[test]
    fn a_statement_is_one_line_of_at_most_64_bytes() {
        let mut line = Vec::new();
        write(&mut line, "0.2.0").unwrap();
        assert_eq!(line, b"vkxp-guest 0.2.0\n");
        let longest = "1".repeat(MAX_LEN - PREFIX.len() - 1);
        line.clear();
        write(&mut line, &longest).unwrap();
        assert_eq!(line.len(), MAX_LEN);
        assert_eq!(read(&mut line.as_slice(), &longest), Ok(()));
        for version in [format!("{longest}1"), String::new(), "0.2 .0".to_owned()] {
            assert!(write(&mut Vec::new(), &version).is_err(), "{version}");
        }
    }

    #[test]
    fn another_version_is_an_error_naming_both() {
        let (mut guest, program) = UnixStream::pair().unwrap();
        write(&mut guest, "0.0.0").unwrap();
        let error = read(&mut &program, "0.2.0").unwrap_err();
        assert_eq!(
            error,
            HelloError::Other {
                stated: "0.0.0".to_owned(),
                own: "0.2.0".to_owned(),
            }
        );
        let message = error.to_string();
        assert!(
            message.contains("0.0.0") && message.contains("0.2.0"),
            "{message}"
        );
    }

    #[test]
    fn what_is_not_a_statement_is_an_error_naming_the_version_of_the_program() {
        let too_long = format!("vkxp-guest {}\n", "1".repeat(MAX_LEN));
        let cases: [&[u8]; 6] = [
            b"",
            b"vkxp-guest",
            b"vkxp-guest \n",
            b"\x0c\x00\x00\x00VKXP",
            b"vkxp-host 0.2.0\n",
            too_long.as_bytes(),
        ];
        for mut bytes in cases {
            let error = read(&mut bytes, "0.2.0").unwrap_err();
            assert_eq!(
                error,
                HelloError::Missing {
                    own: "0.2.0".to_owned()
                }
            );
            assert!(error.to_string().contains("0.2.0"));
        }
    }
}
