//! The global functions of a game that are host functions, and the host functions the Ruby
//! prelude builds on, whose names start with `vkrgss_`.

use std::path::Path;

use vkrgss_proto::GuestToHost;
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{Arity, HostResult, Raise, Raised, Ruby};
use vkrgss_vfs::{DataFile, FileKind, Runtime};

use crate::guest::Guest;
use crate::{audio, error, rb, runtime, stats};

/// `LoadError`.
fn load_error(_ruby: &Ruby) -> VALUE {
    // SAFETY: `&Ruby` proves the thread; the class globals are set once the interpreter runs.
    unsafe { ffi::rb_eLoadError }
}

/// `vkrgss_locate(name, add_extension)`: the path of the Ruby file `name` inside the game
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

/// `vkrgss_relative(file, name)`: for `require_relative` of RGSS3, `name` taken relative to
/// the directory of `file`, the file the call is made from, as a name below the game
/// directory for `require`: `name` itself when `file` has no directory, as a script
/// (`{0001}`) has not, and `LoadError` when that directory is not inside the game directory.
fn relative(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let file = rb::text(ruby, args[0])?;
    let name = rb::text(ruby, args[1])?;
    let Some((directory, _)) = file.rsplit_once(['/', '\\']) else {
        return Ok(rb::text_new(ruby, name.as_bytes())?);
    };
    let game = Guest::with(|guest| guest.files.game_dir().to_owned());
    let below = Path::new(directory)
        .strip_prefix(&game)
        .map_err(|_| Raise::new(load_error(ruby), format!("no such file to load -- {name}")))?;
    let joined = below.join(&name);
    Ok(rb::text_new(ruby, joined.as_os_str().as_encoded_bytes())?)
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

unsafe extern "C" fn string_value_body(data: VALUE) -> VALUE {
    // SAFETY: `string_value` passes the address of a live value in its frame; the call
    // converts it in place, raising for what does not convert, and holds nothing to drop.
    unsafe { ffi::rb_string_value(data as *mut VALUE) }
}

/// The `String` of an argument as the interpreter converts one (`StringValue`): a `String`
/// as it is, an object through its `to_str`, and `TypeError` for anything else, with
/// `nil`, `true` and `false` named so in the message.
fn string_value(_ruby: &Ruby, value: VALUE) -> Result<VALUE, Raised> {
    let mut value = value;
    // SAFETY: `&Ruby` proves the thread; `string_value_body` fits the contract of `protect`.
    unsafe { vkrgss_ruby_sys::protect(&mut value, string_value_body) }
}

/// The name of the encoding of a string.
fn encoding_name(ruby: &Ruby, string: VALUE) -> Result<Vec<u8>, Raised> {
    // SAFETY: `string` is live by the caller's word, and `encoding` was just returned and is
    // in a local.
    let encoding = unsafe { ruby.funcall(string, c"encoding", &[]) }?;
    // SAFETY: as above.
    let name = unsafe { ruby.funcall(encoding, c"name", &[]) }?;
    Ok(rb::string_bytes(ruby, name).unwrap_or_default())
}

/// The name of a data file a game of RGSS3 gives `load_data` or `save_data`, checked as the
/// original runtime 3.0.1 checks it before anything is looked up or written
/// (`ace-v1-api-REPORT.md` section 0.2): converted as a `String` argument, `ArgumentError`
/// for a NUL byte, and `Encoding::UndefinedConversionError` for a name labelled ASCII-8BIT
/// whose bytes are not valid UTF-8, naming the first byte that is not. Other bytes that are
/// not valid UTF-8 are replaced.
fn data_name(ruby: &Ruby, value: VALUE) -> Result<String, Raise> {
    let name = string_value(ruby, value)?;
    let bytes = rb::bytes(ruby, name)?;
    if bytes.contains(&0) {
        return Err(Raise::new(rb::arg_error(ruby), "string contains null byte"));
    }
    if let Err(invalid) = std::str::from_utf8(&bytes)
        && encoding_name(ruby, name)? == b"ASCII-8BIT"
    {
        let byte = bytes[invalid.valid_up_to()];
        let class = rb::class_at(ruby, c"Encoding::UndefinedConversionError")?;
        let message = format!("\"\\x{byte:02X}\" from ASCII-8BIT to UTF-8");
        return Err(Raise::new(class, message));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// `vkrgss_check_data_name(name)`: raises what [`data_name`] raises for the name, before the
/// `save_data` of the prelude of RGSS3 writes anything; `nil` otherwise.
fn check_data_name(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    data_name(ruby, args[0])?;
    Ok(ffi::Qnil)
}

/// `vkrgss_label_utf8(object)`: labels a `String` whose encoding is ASCII-8BIT as UTF-8, a
/// frozen one too, which stays frozen, and returns the object; anything else is returned
/// as it is. The proc `load_data` of RGSS3 hands `Marshal.load` calls it for each object.
/// The encoding is read and set with the functions of the C API, as the original does, so
/// no method a script may redefine is called and the frozen flag is not looked at. Over
/// Ruby 1.8.7, which has no encodings and runs no game of RGSS3, it returns the object.
fn label_utf8(_ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let object = args[0];
    // SAFETY: a value that is no special constant has a type in its header.
    if ffi::special_const_p(object) || unsafe { ffi::builtin_type(object) } != ffi::T_STRING {
        return Ok(object);
    }
    #[cfg(vkrgss_ruby_19)]
    // SAFETY: `&Ruby` proves the thread and `object` is a live string, for which reading
    // and setting the encoding index raises nothing.
    unsafe {
        if ffi::rb_enc_get_index(object) == ffi::rb_ascii8bit_encindex() {
            ffi::rb_enc_associate_index(object, ffi::rb_utf8_encindex());
        }
    }
    Ok(object)
}

/// The proc `load_data` of RGSS3 hands `Marshal.load`, which calls `vkrgss_label_utf8` for
/// each object. It is made at the first call and kept in an instance variable of `Object`
/// whose name scripts cannot reach.
fn label_proc(ruby: &Ruby) -> Result<VALUE, Raised> {
    const NAME: &std::ffi::CStr = c"vkrgss_label_proc";
    let holder = rb::object_class(ruby);
    let kept = rb::ivar_get(ruby, holder, NAME)?;
    if kept != ffi::Qnil {
        return Ok(kept);
    }
    let made = ruby.eval(
        "proc { |object| vkrgss_label_utf8(object) }",
        "(prelude)",
        1,
    )?;
    rb::ivar_set(ruby, holder, NAME, made)?;
    Ok(made)
}

/// `load_data(name)` of RGSS3 (`ace-v1-api-REPORT.md` sections 2.1 and 2.2): the name is
/// checked ([`data_name`]) and looked up as a data file of RGSS3
/// ([`GameFiles::data_rgss3`](vkrgss_vfs::GameFiles::data_rgss3)) from the current directory
/// of the process, and `Marshal.load` is called through Ruby with the bytes of an entry of
/// the archive as a `String`, or with a loose file opened `"rb"`, which is closed
/// afterwards, also when `Marshal.load` raises, and with [`label_proc`]. A directory raises
/// `Errno::EACCES`, a name not found `Errno::ENOENT`.
fn load_data_rgss3(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let name = data_name(ruby, args[0])?;
    let current = std::env::current_dir().unwrap_or_default();
    let found = Guest::with(|guest| {
        // As for `load_data`: the listings do not see what Ruby wrote.
        guest.files.refresh();
        guest.files.data_rgss3(&name, &current)
    })
    .map_err(|error| error::to_raise(ruby, error.into()))?;
    let marshal = rb::class_at(ruby, c"Marshal")?;
    let label = label_proc(ruby)?;
    match found {
        DataFile::Entry(bytes) => {
            let dump = rb::bytes_new(ruby, &bytes)?;
            // SAFETY: `Marshal` is kept alive by its constant, the others are in locals.
            Ok(unsafe { ruby.funcall(marshal, c"load", &[dump, label]) }?)
        }
        DataFile::Loose(path) => {
            let class = rb::class_at(ruby, c"File")?;
            let path = rb::bytes_new(ruby, path.as_os_str().as_encoded_bytes())?;
            let mode = rb::bytes_new(ruby, b"rb")?;
            // SAFETY: `File` is kept alive by its constant, the others are in locals.
            let file = unsafe { ruby.funcall(class, c"open", &[path, mode]) }?;
            // SAFETY: as above; the exception of a failed load stays in the local `loaded`.
            let loaded = unsafe { ruby.funcall(marshal, c"load", &[file, label]) };
            // SAFETY: as above.
            let closed = unsafe { ruby.funcall(file, c"close", &[]) };
            let object = loaded?;
            closed?;
            Ok(object)
        }
    }
}

/// `vkrgss_modal(text)`: shows the host a modal message, and the message panel until it was
/// dismissed. The prelude's `print` and `p` build on it.
fn modal(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let text = rb::text(ruby, args[0])?;
    Guest::with(|guest| guest.show_message(text));
    runtime::raise_pending(ruby)?;
    Ok(ffi::Qnil)
}

/// `vkrgss_modal_p(text)`: `vkrgss_modal` for `p`, which does nothing when `p` is log-only.
fn modal_p(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    if Guest::with(|guest| guest.runtime.run.log_p) {
        return Ok(ffi::Qnil);
    }
    modal(ruby, recv, args)
}

/// `vkrgss_exit_now(status)`: ends a game of RGSS3 at once, for `exit!` and `Process.exit!`
/// of its prelude: the audio stops, the frame statistics are finished, the outcome (a
/// success for `true` and 0) and the shutdown message are sent, and the process ends with
/// the status, `true` 0, `false` 1 and an Integer as it is. No `ensure` clause and nothing
/// else of the interpreter runs. A game of RGSS3 always has its guest in a process of its
/// own, so this ends the guest only.
fn exit_now(ruby: &Ruby, _recv: VALUE, args: &[VALUE]) -> HostResult {
    let status = match args[0] {
        ffi::Qtrue => 0,
        ffi::Qfalse => 1,
        value => rb::int(ruby, value)?,
    };
    Guest::with(|guest| {
        audio::stop_all(guest);
        guest.send(GuestToHost::Outcome { ok: status == 0 });
        guest.send(GuestToHost::Shutdown);
        guest.disconnect();
    });
    stats::finish();
    std::process::exit(status)
}

/// Defines the global functions, and those of RGSS3 in a game of RGSS3.
pub(crate) fn define(ruby: &Ruby) -> Result<(), Raised> {
    ruby.define_global_function_with_arity(c"vkrgss_locate", Arity::Fixed(2), locate)?;
    ruby.define_global_function_with_arity(c"vkrgss_modal", Arity::Fixed(1), modal)?;
    ruby.define_global_function_with_arity(c"vkrgss_modal_p", Arity::Fixed(1), modal_p)?;
    if Guest::with(|guest| guest.runtime.run.runtime) != Runtime::Rgss3 {
        return ruby.define_global_function_with_arity(c"load_data", Arity::Fixed(1), load_data);
    }
    // A function of the C API with a fixed count, so that a wrong count reads `wrong number
    // of arguments(0 for 1)`, as measured on the original runtime of RGSS3.
    ruby.define_global_function_with_arity(c"load_data", Arity::Fixed(1), load_data_rgss3)?;
    ruby.define_global_function_with_arity(c"vkrgss_label_utf8", Arity::Fixed(1), label_utf8)?;
    ruby.define_global_function_with_arity(
        c"vkrgss_check_data_name",
        Arity::Fixed(1),
        check_data_name,
    )?;
    ruby.define_global_function_with_arity(c"vkrgss_relative", Arity::Fixed(2), relative)?;
    ruby.define_global_function_with_arity(c"vkrgss_exit_now", Arity::Fixed(1), exit_now)
}
