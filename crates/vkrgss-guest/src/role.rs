//! The hidden option that makes an executable the guest process of a program.
//!
//! The program gives it to the executable it starts as its guest, as the first argument,
//! with the path of the guest record ([`crate::record`]) as the only other one. The record
//! holds the role itself: the seed of the run and the Unix stream socket the program
//! listens on.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::record::RecordError;

/// The option a program gives the executable it starts as its guest process, with the path
/// of the guest record. It is not part of a usage message.
pub const GUEST_ROLE: &str = "--guest-role";

/// What makes an executable the guest process of a program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestRole {
    /// The seed the parent settled on.
    pub seed: u64,
    /// The socket the parent listens on.
    pub socket: PathBuf,
}

/// The path of the guest record that `arguments` name: the arguments of an executable
/// without its own name. `None` when the first of them is not [`GUEST_ROLE`], which is an
/// executable that is not a guest process. The option takes exactly one more argument, and
/// nothing is read as text: a path need not be UTF-8.
pub fn record_path(
    arguments: impl IntoIterator<Item = OsString>,
) -> Option<Result<PathBuf, RecordError>> {
    let mut arguments = arguments.into_iter();
    if arguments.next()? != GUEST_ROLE {
        return None;
    }
    Some(match (arguments.next(), arguments.next()) {
        (Some(path), None) => Ok(PathBuf::from(path)),
        _ => Err(RecordError::Arguments),
    })
}

#[cfg(test)]
mod tests {
    use std::os::unix::ffi::OsStringExt;

    use super::*;

    fn words(words: &[&str]) -> Vec<OsString> {
        words.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_guest_role_is_the_first_argument_and_names_a_record() {
        let named = record_path(words(&[GUEST_ROLE, "/run/user/1/vkrgss-1/guest.record"]));
        let path = named.unwrap().unwrap();
        assert_eq!(path, PathBuf::from("/run/user/1/vkrgss-1/guest.record"));
        // A path is taken as it is.
        let bytes = OsString::from_vec(b"/run/sch\xf6n/guest.record".to_vec());
        let named = record_path([OsString::from(GUEST_ROLE), bytes.clone()]);
        assert_eq!(named.unwrap().unwrap(), PathBuf::from(bytes));

        // An executable that is not a guest process.
        assert!(record_path(words(&[])).is_none());
        assert!(record_path(words(&["game"])).is_none());
        assert!(record_path(words(&["game", GUEST_ROLE, "/r"])).is_none());
    }

    #[test]
    fn the_guest_role_takes_one_argument_and_nothing_else() {
        for arguments in [
            words(&[GUEST_ROLE]),
            words(&[GUEST_ROLE, "/r", "--verbose"]),
            words(&[GUEST_ROLE, "/r", "game"]),
        ] {
            let error = record_path(arguments).unwrap().unwrap_err();
            assert!(matches!(error, RecordError::Arguments), "{error:?}");
            assert!(error.to_string().contains(GUEST_ROLE), "{error}");
        }
    }
}
