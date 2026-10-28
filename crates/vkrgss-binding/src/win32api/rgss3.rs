//! The class `Win32API` of RGSS3 with its import and export.

use std::ffi::{CStr, c_void};

use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{Arity, HostResult, Raise, Raised, Ruby};
use vkrgss_vfs::FileKind;

use crate::guest::Guest;
use crate::rb;

use super::table::{Arg, find, run_shim};

/// The system libraries of Windows that `Win32API.new` finds in a game of RGSS3, in lower
/// case without `.dll`. vkrgss loads none of them; a function of one is emulated or reported.
/// The list grows when a game shows a need.
const SYSTEM_LIBRARIES: &[&str] = &[
    "kernel32",
    "kernelbase",
    "user32",
    "gdi32",
    "advapi32",
    "shell32",
    "shlwapi",
    "ole32",
    "oleaut32",
    "comctl32",
    "comdlg32",
    "winmm",
    "wininet",
    "winhttp",
    "urlmon",
    "ws2_32",
    "wsock32",
    "msvcrt",
    "ntdll",
    "version",
    "imm32",
    "psapi",
    "dwmapi",
    "uxtheme",
    "msimg32",
    "gdiplus",
    "dsound",
    "ddraw",
    "dinput",
    "dinput8",
    "xinput1_3",
    "opengl32",
    "d3d9",
    "setupapi",
    "crypt32",
    "iphlpapi",
    "secur32",
    "netapi32",
    "mpr",
    "rpcrt4",
    "hid",
    "powrprof",
];

/// The instance variable, without `@` and so out of a script's sight, in which a `Win32API`
/// object of RGSS3 keeps the `Data` object that holds its [`Api`].
const STATE: &CStr = c"win32api";

/// What one character of an import declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Param {
    /// `l`, `i` or `n`: a 32-bit integer.
    Int,
    /// `p`: a pointer.
    Pointer,
}

/// What the first character of an export declares the value to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Export {
    /// `v`: the value is 0.
    Void,
    /// `p`: a String copied from the address.
    Pointer,
    /// Anything else: a signed 32-bit Integer.
    Int,
}

/// The state of a `Win32API` object of RGSS3.
#[derive(Debug, Clone)]
struct Api {
    /// The library as given.
    library: String,
    /// The function as given.
    function: String,
    params: Vec<Param>,
    export: Export,
}

/// The parameter one character of an import declares, in either case; `v` and every other
/// character declare none.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, API report 5.8.
fn param(character: u8) -> Option<Param> {
    match character.to_ascii_lowercase() {
        b'p' => Some(Param::Pointer),
        b'l' | b'i' | b'n' => Some(Param::Int),
        _ => None,
    }
}

/// What an export declares, by its first character.
fn export(declaration: &[u8]) -> Export {
    match declaration.first().map(u8::to_ascii_lowercase) {
        Some(b'v') => Export::Void,
        Some(b'p') => Export::Pointer,
        _ => Export::Int,
    }
}

/// Whether a game of RGSS3 finds the library `name`, with letter case ignored and with or
/// without `.dll`: a file of the game directory, the runtime library `Game.ini` names (by
/// its path, or by its file name alone), or one of [`SYSTEM_LIBRARIES`].
fn library_found(guest: &Guest, name: &str) -> bool {
    let normal = |name: &str| {
        let name = name.trim().replace('\\', "/").to_ascii_lowercase();
        name.strip_suffix(".dll").unwrap_or(&name).to_owned()
    };
    let wanted = normal(name);
    if SYSTEM_LIBRARIES.contains(&wanted.as_str()) {
        return true;
    }
    let runtime = normal(&guest.runtime.run.ini.library);
    let runtime_file = runtime.rsplit('/').next().unwrap_or_default();
    if !runtime.is_empty() && (wanted == runtime || wanted == runtime_file) {
        return true;
    }
    [name.to_owned(), format!("{name}.dll")].iter().any(|file| {
        guest
            .files
            .resolve(FileKind::Data, file)
            .is_ok_and(|path| path.is_file())
    })
}

/// The parameters an import declares: a String, each of its characters; an Array of
/// Strings, the first character of each; `nil`, none.
fn import(ruby: &Ruby, value: VALUE) -> Result<Vec<Param>, Raise> {
    if ffi::nil_p(value) {
        return Ok(Vec::new());
    }
    let Some(elements) = rb::array_elements(ruby, value) else {
        let declaration = rb::string_value(ruby, value)?;
        return Ok(rb::bytes(ruby, declaration)?
            .into_iter()
            .filter_map(param)
            .collect());
    };
    let mut params = Vec::with_capacity(elements.len());
    for element in elements {
        let declaration = rb::bytes(ruby, rb::string_value(ruby, element)?)?;
        params.extend(declaration.first().copied().and_then(param));
    }
    Ok(params)
}

unsafe extern "C" fn free_api(data: *mut c_void) {
    if !data.is_null() {
        // SAFETY: the collector passes the data pointer of an object `initialize` made with
        // this function, once; it is the box made there.
        drop(unsafe { Box::from_raw(data.cast::<Api>()) });
    }
}

/// `Win32API#initialize(library, function, import, export)`: the library and the function
/// are Strings, the import a String, an Array of Strings or `nil`, the export a String or
/// `nil`. A library the game does not find raises `RuntimeError` with `LoadLibrary: <name>`
/// and a line feed, as the original does for a missing DLL.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, API report 5.8.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let library = rb::text(ruby, rb::string_value(ruby, args[0])?)?;
    let function = rb::text(ruby, rb::string_value(ruby, args[1])?)?;
    let params = import(ruby, args[2])?;
    let export = if ffi::nil_p(args[3]) {
        Export::Int
    } else {
        export(&rb::bytes(ruby, rb::string_value(ruby, args[3])?)?)
    };
    if !Guest::with(ruby, |guest| library_found(guest, &library)) {
        // SAFETY: the class globals are set once the interpreter runs.
        let class = unsafe { ffi::rb_eRuntimeError };
        return Err(Raise::new(class, format!("LoadLibrary: {library}\n")));
    }
    let api = Box::into_raw(Box::new(Api {
        library,
        function,
        params,
        export,
    }));
    let data = rb::data_new(ruby, api.cast(), free_api)?;
    rb::ivar_set(ruby, recv, STATE, data)?;
    Ok(ffi::Qnil)
}

/// The state of the `Win32API` object `recv`.
fn api(ruby: &Ruby, recv: VALUE) -> Result<Api, Raise> {
    let state = rb::ivar_get(ruby, recv, STATE)?;
    let Some(data) = rb::data_of(state, free_api) else {
        // SAFETY: the class globals are set once the interpreter runs.
        let class = unsafe { ffi::rb_eRuntimeError };
        return Err(Raise::new(class, "uninitialized Win32API"));
    };
    // SAFETY: `data_of` found the box of an `Api` that `initialize` made, held by `state`,
    // which is in a local; it is copied before anything else runs.
    Ok(unsafe { &*data.cast::<Api>() }.clone())
}

/// An integer parameter: as Ruby converts to an unsigned long, then its low 32 bits, which
/// an entry reads with their sign. The RangeErrors so come at the 64-bit bounds of the
/// guest, not at 2**32 and -2**31: an object id, which a game passes to a library of its
/// own, is above 2**32 in the 64-bit guest.
fn int_param(ruby: &Ruby, value: VALUE) -> Result<i64, Raise> {
    Ok(i64::from(rb::ulong(ruby, value)? as u32 as i32))
}

/// A pointer parameter: `nil` is the null pointer, an Integer an address, a String a
/// buffer that must not be frozen; anything else is converted as Ruby converts to a String.
/// Returns the argument and the String to write a changed buffer back into, when that is the
/// argument itself; a String that `to_str` returned is read and not written back.
fn pointer_param(ruby: &Ruby, value: VALUE) -> Result<(Arg, Option<VALUE>), Raise> {
    if ffi::nil_p(value) {
        return Ok((Arg::Int(0), None));
    }
    // SAFETY: `value` is an argument of the call, and so live.
    let kind = unsafe { ffi::rb_type(value) };
    if kind == ffi::T_FIXNUM || kind == ffi::T_BIGNUM {
        return Ok((Arg::Int(int_param(ruby, value)?), None));
    }
    let string = rb::string_value(ruby, value)?;
    if rb::frozen(string) {
        // SAFETY: the class globals are set once the interpreter runs.
        let class = unsafe { ffi::rb_eRuntimeError };
        // The wording of the interpreter's check before it changes a string.
        return Err(Raise::new(class, "can't modify frozen string"));
    }
    let bytes = rb::bytes(ruby, string)?.into_boxed_slice();
    Ok((Arg::Buffer(bytes), (string == value).then_some(value)))
}

/// Writes the line that says a function is not emulated to `$stderr`, once per library and
/// function.
fn report(ruby: &Ruby, api: &Api) -> Result<(), Raise> {
    let name = format!("{} {}", api.library, api.function);
    if !Guest::with(ruby, |guest| guest.win32api.reported.insert(name.clone())) {
        return Ok(());
    }
    let line = format!("vkrgss: Win32API {name} is not emulated and returns 0\n");
    let line = rb::text_new(ruby, line.as_bytes())?;
    let stderr = rb::global(ruby, c"$stderr")?;
    // SAFETY: both values were just returned and are in locals.
    unsafe { ruby.funcall(stderr, c"write", &[line]) }?;
    Ok(())
}

/// `Win32API#call(*arguments)` and `Call`: the count must be the declared one; the value
/// follows the export (decision 6 of the change that made this class).
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1, API report 5.8.
fn call_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let api = api(ruby, recv)?;
    if args.len() != api.params.len() {
        // SAFETY: the class globals are set once the interpreter runs.
        let class = unsafe { ffi::rb_eRuntimeError };
        let message = format!(
            "wrong number of parameters: expected {}, got {}",
            api.params.len(),
            args.len()
        );
        return Err(Raise::new(class, message));
    }
    let mut converted = Vec::with_capacity(args.len());
    let mut strings = Vec::with_capacity(args.len());
    for (&value, &param) in args.iter().zip(&api.params) {
        let (arg, string) = match param {
            Param::Int => (Arg::Int(int_param(ruby, value)?), None),
            Param::Pointer => pointer_param(ruby, value)?,
        };
        converted.push(arg);
        strings.push(string);
    }
    let result = match find(&api.library, &api.function) {
        Some(shim) => Some(run_shim(ruby, shim, &strings, converted, true)?),
        None => {
            report(ruby, &api)?;
            None
        }
    };
    match (api.export, result) {
        (Export::Void, _) => Ok(rb::fix(0)),
        (Export::Pointer, Some(0)) => Err(Raise::new(rb::arg_error(ruby), "NULL pointer given")),
        // No value of an emulated function is an address the guest has.
        (Export::Pointer, _) => Ok(rb::bytes_new(ruby, b"")?),
        (Export::Int, result) => Ok(rb::fix(result.unwrap_or(0) as i32)),
    }
}

/// Defines `Win32API` of RGSS3 with the methods of the method list.
pub(super) fn define_rgss3(ruby: &Ruby) -> Result<(), Raised> {
    // SAFETY: `Object` is a live class.
    let class = unsafe { ruby.define_class(c"Win32API", rb::object_class(ruby)) }?;
    // SAFETY: the class was just defined.
    unsafe {
        ruby.define_private_method_with_arity(class, c"initialize", Arity::Fixed(4), initialize)
    }?;
    rb::method(ruby, class, c"call", -1, call_rgss3)?;
    rb::method(ruby, class, c"Call", -1, call_rgss3)
}
