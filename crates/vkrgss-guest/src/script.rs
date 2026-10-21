//! The hidden option that makes an executable run a script for a program.
//!
//! A program that does not link the interpreter a script needs replaces itself with the
//! executable that does, with `exec`, and gives it this option as the first argument and
//! then, in this order, the runtime, whether it confines itself (`on`, `off` or `default`),
//! whether the functions that start processes do so (`on` or `off`), whether informational
//! messages are written (`on` or `off`), the file and the arguments of the script. The
//! level of the diagnostics is otherwise the environment variable of the program, which the
//! executable inherits. Nothing but the runtime is read as text: a path or an argument
//! need not be UTF-8.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use vkrgss_vfs::Runtime;

/// The option a program gives the executable that runs its script. It is not part of a
/// usage message.
pub const SCRIPT_ROLE: &str = "--script-role";

/// What a script runs with, as a program settles it, and what the arguments of
/// [`SCRIPT_ROLE`] hand the executable that runs the script for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptRole {
    /// The runtime whose interpreter runs the script.
    pub runtime: Runtime,
    /// Whether the process confines itself, if that was given.
    pub sandbox: Option<bool>,
    /// Whether the functions that start processes do so.
    pub process_functions: bool,
    /// Whether informational and debug messages are written too.
    pub verbose: bool,
    /// The file of the script, as an absolute path.
    pub file: PathBuf,
    /// The arguments of the script.
    pub arguments: Vec<OsString>,
}

/// The arguments of [`SCRIPT_ROLE`] do not have its form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "{SCRIPT_ROLE} takes a runtime, three settings, a file and its arguments; the program \
     gives it to the executable that runs its script"
)]
pub struct ScriptRoleError;

/// The word of a setting.
fn word(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

/// The setting of a word: `on` or `off`.
fn setting(word: &OsStr) -> Result<bool, ScriptRoleError> {
    match word.to_str() {
        Some("on") => Ok(true),
        Some("off") => Ok(false),
        _ => Err(ScriptRoleError),
    }
}

impl ScriptRole {
    /// The arguments of the executable, the option first.
    pub fn arguments(&self) -> Vec<OsString> {
        let sandbox = self.sandbox.map_or("default", word);
        let mut arguments: Vec<OsString> = [
            SCRIPT_ROLE,
            self.runtime.name(),
            sandbox,
            word(self.process_functions),
            word(self.verbose),
        ]
        .map(OsString::from)
        .into();
        arguments.push(self.file.clone().into_os_string());
        arguments.extend(self.arguments.iter().cloned());
        arguments
    }

    /// The role that `arguments`, the arguments of an executable without its own name, give
    /// it. `None` when the first of them is not [`SCRIPT_ROLE`], which is an executable that
    /// runs no script for a program.
    pub fn read(
        arguments: impl IntoIterator<Item = OsString>,
    ) -> Option<Result<ScriptRole, ScriptRoleError>> {
        let mut arguments = arguments.into_iter();
        if arguments.next()? != SCRIPT_ROLE {
            return None;
        }
        Some(Self::read_settings(arguments))
    }

    /// The role of the arguments after [`SCRIPT_ROLE`].
    fn read_settings(
        mut arguments: impl Iterator<Item = OsString>,
    ) -> Result<ScriptRole, ScriptRoleError> {
        let mut next = || arguments.next().ok_or(ScriptRoleError);
        let runtime = next()?;
        let runtime = Runtime::ALL
            .into_iter()
            .find(|known| runtime == known.name())
            .ok_or(ScriptRoleError)?;
        let sandbox = next()?;
        let sandbox = match sandbox.to_str() {
            Some("default") => None,
            _ => Some(setting(&sandbox)?),
        };
        let process_functions = setting(&next()?)?;
        let verbose = setting(&next()?)?;
        let file = PathBuf::from(next()?);
        Ok(ScriptRole {
            runtime,
            sandbox,
            process_functions,
            verbose,
            file,
            arguments: arguments.collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::ffi::OsStringExt;

    use super::*;

    fn words(words: &[&str]) -> Vec<OsString> {
        words.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_role_is_read_back_from_its_arguments() {
        let role = ScriptRole {
            runtime: Runtime::Rgss3,
            sandbox: None,
            process_functions: true,
            verbose: false,
            file: PathBuf::from(OsString::from_vec(b"/s/sch\xf6n.rb".to_vec())),
            arguments: words(&["a", "--verbose", ""]),
        };
        let arguments = role.arguments();
        assert_eq!(
            arguments[..5],
            words(&[SCRIPT_ROLE, "rgss3", "default", "on", "off"])
        );
        assert_eq!(ScriptRole::read(arguments), Some(Ok(role.clone())));
        for sandbox in [Some(true), Some(false)] {
            let role = ScriptRole {
                sandbox,
                verbose: true,
                arguments: Vec::new(),
                ..role.clone()
            };
            assert_eq!(ScriptRole::read(role.arguments()), Some(Ok(role)));
        }
    }

    #[test]
    fn other_arguments_are_no_role_or_a_wrong_one() {
        assert_eq!(ScriptRole::read(words(&[])), None);
        assert_eq!(ScriptRole::read(words(&["--guest-role", "/r"])), None);
        for arguments in [
            words(&[SCRIPT_ROLE]),
            words(&[SCRIPT_ROLE, "rgss4", "on", "on", "on", "/s.rb"]),
            words(&[SCRIPT_ROLE, "rgss1", "maybe", "on", "on", "/s.rb"]),
            words(&[SCRIPT_ROLE, "rgss1", "on", "on", "on"]),
        ] {
            assert_eq!(ScriptRole::read(arguments), Some(Err(ScriptRoleError)));
        }
    }
}
