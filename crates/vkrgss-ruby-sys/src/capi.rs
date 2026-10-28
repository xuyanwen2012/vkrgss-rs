//! The C API of Ruby 1.8.7 as the shared glue of `vkrgss-ruby-glue` uses it.

use std::ffi::{c_char, c_int, c_long};

use vkrgss_ruby_glue::Interpreter;

use crate::ffi::{self, AnyArgsFn, ArgvFn, ID, ProtectFn, VALUE};

/// The embedded Ruby 1.8.7, the interpreter that [`Ruby`](crate::Ruby) and
/// [`Root`](crate::Root) of this crate are for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CApi;

// SAFETY: every item forwards to the declaration of `ffi` of the same name, or to the
// variable or the function of this interpreter that its documentation names; the constants
// are those of `ffi`, which `tests/layout.rs` checks against the headers.
unsafe impl Interpreter for CApi {
    const QNIL: VALUE = ffi::Qnil;
    const QFALSE: VALUE = ffi::Qfalse;
    const QTRUE: VALUE = ffi::Qtrue;
    const TAG_RAISE: c_int = ffi::TAG_RAISE;
    const TAG_FATAL: c_int = ffi::TAG_FATAL;
    const T_STRING: c_int = ffi::T_STRING;
    const T_ARRAY: c_int = ffi::T_ARRAY;
    // A `throw` keeps its tag and value in the tag of its `catch`.
    const JUMP_IN_CURRENT_ERROR: bool = false;

    unsafe fn rb_protect(function: ProtectFn, arg: VALUE, state: *mut c_int) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_protect(function, arg, state) }
    }

    unsafe fn rb_jump_tag(state: c_int) -> ! {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_jump_tag(state) }
    }

    unsafe fn rb_exc_raise(exception: VALUE) -> ! {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_exc_raise(exception) }
    }

    unsafe fn rb_exc_new(class: VALUE, message: *const c_char, len: c_long) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_exc_new(class, message, len) }
    }

    unsafe fn exception_class() -> VALUE {
        // SAFETY: after start-up the class global is set and only read.
        unsafe { ffi::rb_eException }
    }

    unsafe fn argument_error_class() -> VALUE {
        // SAFETY: after start-up the class global is set and only read.
        unsafe { ffi::rb_eArgError }
    }

    unsafe fn errinfo() -> VALUE {
        // SAFETY: on the interpreter thread `ruby_errinfo` is only touched by the
        // interpreter, which is not running now.
        unsafe { ffi::ruby_errinfo }
    }

    unsafe fn restore_errinfo(value: VALUE) {
        // SAFETY: as for `errinfo`; the variable is one the collector marks, and assigning
        // it raises nothing.
        unsafe { ffi::ruby_errinfo = value };
    }

    unsafe fn set_errinfo(value: VALUE) {
        // SAFETY: as for `restore_errinfo`. Ruby 1.8.7 has no `rb_set_errinfo`, and its
        // variable takes any value.
        unsafe { ffi::ruby_errinfo = value };
    }

    unsafe fn rb_gc_register_address(address: *mut VALUE) {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_gc_register_address(address) }
    }

    unsafe fn rb_gc_unregister_address(address: *mut VALUE) {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_gc_unregister_address(address) }
    }

    unsafe fn rb_define_class(name: *const c_char, superclass: VALUE) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_define_class(name, superclass) }
    }

    unsafe fn rb_define_module(name: *const c_char) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_define_module(name) }
    }

    unsafe fn rb_define_class_under(outer: VALUE, name: *const c_char, superclass: VALUE) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_define_class_under(outer, name, superclass) }
    }

    unsafe fn rb_define_module_under(outer: VALUE, name: *const c_char) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_define_module_under(outer, name) }
    }

    unsafe fn rb_define_method(
        class: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    ) {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_define_method(class, name, function, argc) }
    }

    unsafe fn rb_define_private_method(
        class: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    ) {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_define_private_method(class, name, function, argc) }
    }

    unsafe fn rb_define_singleton_method(
        object: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    ) {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_define_singleton_method(object, name, function, argc) }
    }

    unsafe fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    ) {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_define_module_function(module, name, function, argc) }
    }

    unsafe fn rb_define_global_function(name: *const c_char, function: AnyArgsFn, argc: c_int) {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_define_global_function(name, function, argc) }
    }

    fn method_argv(function: ArgvFn) -> AnyArgsFn {
        ffi::method_argv(function)
    }

    unsafe fn rb_intern(name: *const c_char) -> ID {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_intern(name) }
    }

    unsafe fn rb_funcall2(recv: VALUE, method: ID, argc: c_int, argv: *const VALUE) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_funcall2(recv, method, argc, argv) }
    }

    unsafe fn rb_obj_class(object: VALUE) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_obj_class(object) }
    }

    unsafe fn rb_class_path(class: VALUE) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_class_path(class) }
    }

    unsafe fn rb_obj_as_string(object: VALUE) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_obj_as_string(object) }
    }

    unsafe fn rb_check_array_type(value: VALUE) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_check_array_type(value) }
    }

    unsafe fn rb_ary_new() -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_ary_new() }
    }

    unsafe fn rb_ary_entry(array: VALUE, index: c_long) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_ary_entry(array, index) }
    }

    unsafe fn rb_ary_push(array: VALUE, element: VALUE) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_ary_push(array, element) }
    }

    unsafe fn rb_str_new(bytes: *const c_char, len: c_long) -> VALUE {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_str_new(bytes, len) }
    }

    unsafe fn str_new_text(bytes: *const c_char, len: c_long) -> VALUE {
        // SAFETY: the caller keeps the contract of `rb_str_new`. A string of Ruby 1.8 has no
        // encoding.
        unsafe { ffi::rb_str_new(bytes, len) }
    }

    unsafe fn rb_exec_end_proc() {
        // SAFETY: the caller keeps the contract of the C function.
        unsafe { ffi::rb_exec_end_proc() }
    }

    unsafe fn rb_type(value: VALUE) -> c_int {
        // SAFETY: the caller guarantees a special constant or a live object.
        unsafe { ffi::rb_type(value) }
    }

    unsafe fn rstring_ptr(value: VALUE) -> *mut c_char {
        // SAFETY: the caller guarantees a live string.
        unsafe { ffi::rstring_ptr(value) }
    }

    unsafe fn rstring_len(value: VALUE) -> c_long {
        // SAFETY: the caller guarantees a live string.
        unsafe { ffi::rstring_len(value) }
    }

    unsafe fn rarray_ptr(value: VALUE) -> *mut VALUE {
        // SAFETY: the caller guarantees a live array.
        unsafe { ffi::rarray_ptr(value) }
    }

    unsafe fn rarray_len(value: VALUE) -> c_long {
        // SAFETY: the caller guarantees a live array.
        unsafe { ffi::rarray_len(value) }
    }

    fn long2fix(integer: c_long) -> VALUE {
        ffi::long2fix(integer)
    }

    unsafe fn top_self() -> VALUE {
        // SAFETY: on the interpreter thread, on which only the interpreter writes the
        // variable, once, at start-up.
        unsafe { ffi::ruby_top_self }
    }

    unsafe fn mark_game_scripts() {
        // SAFETY: on the interpreter thread; the function sets a flag and cannot raise.
        unsafe { ffi::ruby_vkxp_mark_game_scripts() };
    }
}
