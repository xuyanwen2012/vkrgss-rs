//! The restriction a guest process puts on itself before the first script runs.
//!
//! This is platform code for Linux: the rules are those of Landlock, made with the
//! `landlock` crate in its best-effort mode. The guest then reads every file as before,
//! writes, creates, removes and renames only below the game directory, writes the device
//! `/dev/null`, executes nothing, where the kernel has those rules neither binds nor connects
//! a TCP socket, and where it has the signal scope (ABI 6) signals no process outside the
//! confinement. The rules bind the calling thread and what it starts later, the copies of the
//! guest process included; connecting to a Unix socket by its path, which those copies do,
//! is not among them.

use std::path::Path;

use landlock::{
    ABI, AccessFs, AccessNet, LandlockStatus, PathBeneath, PathFd, PathFdError, RestrictionStatus,
    Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetError, RulesetStatus, Scope,
    path_beneath_rules,
};

/// The device every process may write.
const NULL_DEVICE: &str = "/dev/null";

/// Restricts the calling thread, which has to be the one that will run the interpreter, and
/// reports once what the kernel could not do. Without Landlock, or when a call fails, the
/// guest runs unconfined.
pub fn confine(game_dir: &Path) {
    match restrict(game_dir) {
        Ok(status) => report(&status),
        Err(error) => {
            tracing::warn!("the guest process is not confined: {}", chain(&error));
        }
    }
}

/// Why the rules could not be made or put on the thread.
#[derive(Debug, thiserror::Error)]
enum RestrictError {
    #[error(transparent)]
    Ruleset(#[from] RulesetError),
    #[error(transparent)]
    Path(#[from] PathFdError),
}

/// An error and its causes in one line, each after a colon.
fn chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(error) = cause {
        text.push_str(&format!(": {error}"));
        cause = error.source();
    }
    text
}

fn restrict(game_dir: &Path) -> Result<RestrictionStatus, RestrictError> {
    // Writing in every form the fourth ABI knows: later ones add rights for device
    // requests and for connecting to Unix sockets, which stay as they are.
    let writing = AccessFs::from_write(ABI::V4);
    let status = Ruleset::default()
        .handle_access(writing | AccessFs::Execute)?
        .handle_access(AccessNet::BindTcp | AccessNet::ConnectTcp)?
        .scope(Scope::Signal)?
        .create()?
        .add_rule(PathBeneath::new(PathFd::new(game_dir)?, writing))?
        .add_rules(path_beneath_rules(
            [NULL_DEVICE],
            AccessFs::WriteFile | AccessFs::Truncate,
        ))?
        .restrict_self()?;
    Ok(status)
}

/// What an ABI older than the sixth leaves unrestricted; nothing from the sixth on.
fn missing(abi: ABI) -> &'static str {
    match abi {
        ABI::Unsupported => "everything",
        ABI::V1 | ABI::V2 => "truncating files, TCP connections and signals to other processes",
        ABI::V3 => "TCP connections and signals to other processes",
        ABI::V4 | ABI::V5 => "signals to other processes",
        _ => "",
    }
}

fn report(status: &RestrictionStatus) {
    let abi = match status.landlock {
        LandlockStatus::Available { effective_abi, .. } => effective_abi,
        LandlockStatus::NotEnabled | LandlockStatus::NotImplemented => ABI::Unsupported,
    };
    match status.ruleset {
        RulesetStatus::NotEnforced => {
            let why = match status.landlock {
                LandlockStatus::NotEnabled => "Landlock is not enabled in this kernel",
                LandlockStatus::NotImplemented => "this kernel has no Landlock",
                LandlockStatus::Available { .. } => "the kernel enforces none of the rules",
            };
            tracing::warn!("the guest process is not confined: {why}");
        }
        RulesetStatus::PartiallyEnforced => tracing::info!(
            "the guest process is confined with Landlock ABI {}, which does not restrict {}",
            abi as i32,
            missing(abi)
        ),
        RulesetStatus::FullyEnforced => tracing::info!(
            "the guest process is confined with Landlock ABI {}",
            abi as i32
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rights_an_older_kernel_lacks_are_named() {
        assert!(missing(ABI::V1).contains("TCP") && missing(ABI::V1).contains("truncating"));
        assert!(missing(ABI::V2).contains("truncating"));
        assert!(missing(ABI::V2).contains("signals"));
        assert_eq!(
            missing(ABI::V3),
            "TCP connections and signals to other processes"
        );
        assert_eq!(missing(ABI::V4), "signals to other processes");
        assert_eq!(missing(ABI::V5), "signals to other processes");
        assert_eq!(missing(ABI::V6), "");
        assert_eq!(missing(ABI::V9), "");
    }
}
