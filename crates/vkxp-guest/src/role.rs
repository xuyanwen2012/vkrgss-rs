//! The hidden option that makes an executable the guest process of a program.
//!
//! The program gives it to the executable it starts as its guest, with the value
//! `SEED=SOCKET`: the seed of the run and the Unix stream socket the program listens on.
//! Both sides of that convention are here: [`GuestRole::parse`] reads the value and
//! [`GuestRole::value`] writes it.

use std::path::PathBuf;

/// The option a program gives the executable it starts as its guest process, with the value
/// `SEED=SOCKET`. It is not part of a usage message.
pub const GUEST_ROLE: &str = "--guest-role";

/// What makes an executable the guest process of a program: the value of [`GUEST_ROLE`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestRole {
    /// The seed the parent settled on.
    pub seed: u64,
    /// The socket the parent listens on.
    pub socket: PathBuf,
}

/// A value of [`GUEST_ROLE`] that cannot be read or written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoleError {
    /// The value is not two parts around a `=`.
    #[error("`{0}` is not of the form SEED=SOCKET")]
    Form(String),
    /// The first part is not a number from 0 on.
    #[error("`{0}` is not a seed")]
    Seed(String),
    /// The socket cannot be an argument that is read as text.
    #[error("the socket path {} is not UTF-8", .0.display())]
    Socket(PathBuf),
}

impl GuestRole {
    /// Reads the value of [`GUEST_ROLE`]: `SEED=SOCKET` with neither part empty.
    pub fn parse(text: &str) -> Result<GuestRole, RoleError> {
        let (seed, socket) = text
            .split_once('=')
            .filter(|(seed, socket)| !seed.is_empty() && !socket.is_empty())
            .ok_or_else(|| RoleError::Form(text.to_owned()))?;
        let seed = seed.parse().map_err(|_| RoleError::Seed(seed.to_owned()))?;
        Ok(GuestRole {
            seed,
            socket: PathBuf::from(socket),
        })
    }

    /// The value of [`GUEST_ROLE`] that [`GuestRole::parse`] reads as this role.
    pub fn value(&self) -> Result<String, RoleError> {
        let socket = self
            .socket
            .to_str()
            .ok_or_else(|| RoleError::Socket(self.socket.clone()))?;
        Ok(format!("{}={socket}", self.seed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guest_role_is_a_hidden_option_with_a_seed_and_a_socket() {
        let role = GuestRole {
            seed: 7,
            socket: "/run/user/1/vkxp-1/guest.sock".into(),
        };
        assert_eq!(
            GuestRole::parse("7=/run/user/1/vkxp-1/guest.sock"),
            Ok(role.clone())
        );
        assert_eq!(role.value().unwrap(), "7=/run/user/1/vkxp-1/guest.sock");
        assert_eq!(GuestRole::parse(&role.value().unwrap()), Ok(role));
        assert_eq!(
            GuestRole::parse("seven=/s"),
            Err(RoleError::Seed("seven".to_owned()))
        );
        assert_eq!(
            GuestRole::parse("/s"),
            Err(RoleError::Form("/s".to_owned()))
        );
    }
}
