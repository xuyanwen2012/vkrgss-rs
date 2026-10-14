//! The global functions of a game that are host functions, and the host functions the Ruby
//! prelude builds on, whose names start with `vkxp_`.

use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{Arity, HostResult, Raise, Raised, Ruby};
use vkxp_vfs::FileKind;

use crate::error;
use crate::guest::Guest;
use crate::rb;
use crate::runtime;

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
        Ok(path) => Ok(rb::bytes_new(ruby, path.as_os_str().as_encoded_bytes())?),
        Err(_) => Err(not_loadable()),
    }
}

/// `load_data(path)`: the object `Marshal` loads from a data file of the game.
fn load_data(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let path = rb::text(ruby, args[0])?;
    let bytes = Guest::with(|guest| {
        // Scripts write data files through Ruby's own file API (`save_data`, saves), which
        // the cached directory listings do not see.
        guest.files.refresh();
        guest.files.read(FileKind::Data, &path)
    })
    .map_err(|error| error::to_raise(ruby, error.into()))?;
    let dump = rb::bytes_new(ruby, &bytes)?;
    let marshal = rb::class_at(ruby, c"Marshal")?;
    // SAFETY: `Marshal` is kept alive by its constant, and `dump` was just returned and is
    // in a local.
    Ok(unsafe { ruby.funcall(marshal, c"load", &[dump]) }?)
}

/// `vkxp_modal(text)`: shows the host a modal message, and the message panel until it was
/// dismissed. The prelude's `print` and `p` build on it.
fn modal(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let text = rb::text(ruby, args[0])?;
    Guest::with(|guest| guest.show_message(text));
    runtime::raise_pending(ruby)?;
    Ok(ffi::Qnil)
}

/// `vkxp_modal_p(text)`: `vkxp_modal` for `p`, which does nothing when `p` is log-only.
fn modal_p(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    if Guest::with(|guest| guest.runtime.run.log_p) {
        return Ok(ffi::Qnil);
    }
    modal(ruby, recv, args)
}

/// Defines the global functions.
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    ruby.define_global_function_with_arity(c"load_data", Arity::Fixed(1), load_data)?;
    ruby.define_global_function_with_arity(c"vkxp_locate", Arity::Fixed(2), locate)?;
    ruby.define_global_function_with_arity(c"vkxp_modal", Arity::Fixed(1), modal)?;
    ruby.define_global_function_with_arity(c"vkxp_modal_p", Arity::Fixed(1), modal_p)
}
