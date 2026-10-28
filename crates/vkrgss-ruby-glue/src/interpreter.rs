//! What the glue needs from an interpreter crate.

use std::ffi::{c_char, c_int, c_long};

use crate::{AnyArgsFn, ArgvFn, ID, ProtectFn, VALUE};

/// The C API of one embedded interpreter, as far as the glue uses it.
///
/// Each interpreter crate implements it on a unit type with its own declarations, and the
/// glue is compiled once for each: the trampolines and the protected bodies are monomorphised
/// per interpreter, and every item here is a direct call or a constant.
///
/// An item named after a function of the C API has its signature and its contract: it is
/// called on a thread that holds the interpreter, while the interpreter is alive, with live
/// values, and it may raise unless its documentation says otherwise. The other items say what
/// they do.
///
/// # Safety
///
/// An implementation forwards every item to the interpreter it embeds, and its constants are
/// the values of that interpreter's headers. The items documented as raising nothing must not
/// raise, since the glue calls them outside a protection.
pub unsafe trait Interpreter: Sized + 'static {
    /// `Qnil`.
    const QNIL: VALUE;
    /// `Qfalse`.
    const QFALSE: VALUE;
    /// `Qtrue`.
    const QTRUE: VALUE;
    /// The status of `rb_protect` for a raised exception.
    const TAG_RAISE: c_int;
    /// The status of `rb_protect` for a fatal error, which `rescue` does not catch.
    const TAG_FATAL: c_int;
    /// The type tag of a string.
    const T_STRING: c_int;
    /// The type tag of an array.
    const T_ARRAY: c_int;
    /// Whether a jump that is not a raise (a `throw`, a `break`) keeps what it carries in the
    /// current error until it is continued, as Ruby 1.9 does, rather than in the tag it jumps
    /// to, as Ruby 1.8 does.
    const JUMP_IN_CURRENT_ERROR: bool;

    /// `rb_protect`, which returns normally whatever `function` does.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_protect(function: ProtectFn, arg: VALUE, state: *mut c_int) -> VALUE;

    /// `rb_jump_tag`: continues a jump with this status.
    ///
    /// # Safety
    ///
    /// As the C function; there must be a handler to jump to.
    unsafe fn rb_jump_tag(state: c_int) -> !;

    /// `rb_exc_raise`.
    ///
    /// # Safety
    ///
    /// As the C function; there must be a handler to jump to.
    unsafe fn rb_exc_raise(exception: VALUE) -> !;

    /// `rb_exc_new`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_exc_new(class: VALUE, message: *const c_char, len: c_long) -> VALUE;

    /// The class `Exception`, `rb_eException`. Raises nothing.
    ///
    /// # Safety
    ///
    /// After start-up.
    unsafe fn exception_class() -> VALUE;

    /// The class `ArgumentError`, `rb_eArgError`. Raises nothing.
    ///
    /// # Safety
    ///
    /// After start-up.
    unsafe fn argument_error_class() -> VALUE;

    /// The current error: `$!`, or what a pending jump carries. Raises nothing.
    ///
    /// # Safety
    ///
    /// On a thread that holds the interpreter.
    unsafe fn errinfo() -> VALUE;

    /// Makes `value` the current error again. Raises nothing, whatever the value: it is
    /// `nil`, an exception, or a value that was the current error before, such as what a
    /// pending `throw` carries.
    ///
    /// # Safety
    ///
    /// On a thread that holds the interpreter; `value` is one of the above.
    unsafe fn restore_errinfo(value: VALUE);

    /// Sets `$!` as an assignment of a script does: `rb_set_errinfo`, which raises
    /// `TypeError` for a value that is neither `nil` nor an exception, where the interpreter
    /// has it, and otherwise an assignment of the variable.
    ///
    /// # Safety
    ///
    /// On a thread that holds the interpreter; `value` is a special constant or a live object.
    unsafe fn set_errinfo(value: VALUE);

    /// `rb_gc_register_address`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_gc_register_address(address: *mut VALUE);

    /// `rb_gc_unregister_address`. Raises nothing.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_gc_unregister_address(address: *mut VALUE);

    /// `rb_define_class`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_define_class(name: *const c_char, superclass: VALUE) -> VALUE;

    /// `rb_define_module`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_define_module(name: *const c_char) -> VALUE;

    /// `rb_define_class_under`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_define_class_under(outer: VALUE, name: *const c_char, superclass: VALUE) -> VALUE;

    /// `rb_define_module_under`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_define_module_under(outer: VALUE, name: *const c_char) -> VALUE;

    /// `rb_define_method`.
    ///
    /// # Safety
    ///
    /// As the C function: `function` has the signature Ruby calls for `argc`.
    unsafe fn rb_define_method(class: VALUE, name: *const c_char, function: AnyArgsFn, argc: c_int);

    /// `rb_define_private_method`.
    ///
    /// # Safety
    ///
    /// As the C function: `function` has the signature Ruby calls for `argc`.
    unsafe fn rb_define_private_method(
        class: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    );

    /// `rb_define_singleton_method`.
    ///
    /// # Safety
    ///
    /// As the C function: `function` has the signature Ruby calls for `argc`.
    unsafe fn rb_define_singleton_method(
        object: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    );

    /// `rb_define_module_function`.
    ///
    /// # Safety
    ///
    /// As the C function: `function` has the signature Ruby calls for `argc`.
    unsafe fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    );

    /// `rb_define_global_function`.
    ///
    /// # Safety
    ///
    /// As the C function: `function` has the signature Ruby calls for `argc`.
    unsafe fn rb_define_global_function(name: *const c_char, function: AnyArgsFn, argc: c_int);

    /// Converts a function with the `argc == -1` signature to the type the definition
    /// functions take.
    fn method_argv(function: ArgvFn) -> AnyArgsFn;

    /// `rb_intern`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_intern(name: *const c_char) -> ID;

    /// `rb_funcall2`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_funcall2(recv: VALUE, method: ID, argc: c_int, argv: *const VALUE) -> VALUE;

    /// `rb_obj_class`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_obj_class(object: VALUE) -> VALUE;

    /// `rb_class_path`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_class_path(class: VALUE) -> VALUE;

    /// `rb_obj_as_string`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_obj_as_string(object: VALUE) -> VALUE;

    /// `rb_check_array_type`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_check_array_type(value: VALUE) -> VALUE;

    /// `rb_ary_new`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_ary_new() -> VALUE;

    /// `rb_ary_entry`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_ary_entry(array: VALUE, index: c_long) -> VALUE;

    /// `rb_ary_push`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_ary_push(array: VALUE, element: VALUE) -> VALUE;

    /// `rb_str_new`: a string of bytes, labelled ASCII-8BIT where strings have an encoding.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_str_new(bytes: *const c_char, len: c_long) -> VALUE;

    /// A string of text: the string of `rb_str_new` where strings have no encoding, and
    /// otherwise one labelled UTF-8, valid or not, as the literals of a script are.
    ///
    /// # Safety
    ///
    /// As `rb_str_new`.
    unsafe fn str_new_text(bytes: *const c_char, len: c_long) -> VALUE;

    /// `rb_exec_end_proc`.
    ///
    /// # Safety
    ///
    /// As the C function.
    unsafe fn rb_exec_end_proc();

    /// `TYPE`: the type tag of any value. Raises nothing.
    ///
    /// # Safety
    ///
    /// `value` must be a special constant or a live heap object.
    unsafe fn rb_type(value: VALUE) -> c_int;

    /// `RSTRING_PTR`. Raises nothing.
    ///
    /// # Safety
    ///
    /// `value` must be a live string.
    unsafe fn rstring_ptr(value: VALUE) -> *mut c_char;

    /// `RSTRING_LEN`. Raises nothing.
    ///
    /// # Safety
    ///
    /// `value` must be a live string.
    unsafe fn rstring_len(value: VALUE) -> c_long;

    /// `RARRAY_PTR`. Raises nothing.
    ///
    /// # Safety
    ///
    /// `value` must be a live array.
    unsafe fn rarray_ptr(value: VALUE) -> *mut VALUE;

    /// `RARRAY_LEN`. Raises nothing.
    ///
    /// # Safety
    ///
    /// `value` must be a live array.
    unsafe fn rarray_len(value: VALUE) -> c_long;

    /// `LONG2FIX`.
    fn long2fix(integer: c_long) -> VALUE;

    /// The `main` object, `self` at the top level. Raises nothing.
    ///
    /// # Safety
    ///
    /// On a thread that holds the interpreter, after start-up.
    unsafe fn top_self() -> VALUE;

    /// Tells the interpreter that the game's scripts begin here, where its symbol numbers
    /// depend on it. Raises nothing.
    ///
    /// # Safety
    ///
    /// On a thread that holds the interpreter, after start-up.
    unsafe fn mark_game_scripts();
}
