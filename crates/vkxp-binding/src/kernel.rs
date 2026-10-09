//! The global functions of a game that are host functions, and the host functions the Ruby
//! prelude builds on, whose names start with `vkxp_`.

use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Arity, HostResult, Raise, Raised, Ruby};
use vkxp_vfs::FileKind;

use crate::guest::Guest;
use crate::rb;

/// `LoadError`.
fn load_error(_ruby: &Ruby) -> VALUE {
    // SAFETY: `&Ruby` proves the thread; the class globals are set once the interpreter runs.
    unsafe { ffi::rb_eLoadError }
}

/// `vkxp_locate(name, add_extension)`: the path of the Ruby file `name` inside the game
/// directory, for the prelude's `require` (which adds `.rb` to a name without it) and `load`
/// (which does not). Anything that is not a `.rb` file inside the game directory raises
/// `LoadError`.
fn locate(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let name = rb::text(ruby, args[0])?;
    let not_loadable = || Raise::new(load_error(ruby), format!("no such file to load -- {name}"));
    let mut file = name.clone();
    if !file.ends_with(".rb") {
        if !rb::truthy(args[1]) {
            return Err(not_loadable());
        }
        file.push_str(".rb");
    }
    // The game's files never resolve a path to something outside the game directory.
    let found = Guest::with(|guest| guest.files.resolve(FileKind::Data, &file));
    match found {
        Ok(path) => rb::string_new(ruby, path.as_os_str().as_encoded_bytes()),
        Err(_) => Err(not_loadable()),
    }
}

/// Defines the global functions.
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    ruby.define_global_function_with_arity(c"vkxp_locate", Arity::Fixed(2), locate)
}
