//! Hand-written declarations of the Ruby 1.9.2 C API.
//!
//! Everything here mirrors `ruby/ruby.h`, `ruby/intern.h` and `ruby/encoding.h` of the
//! vendored tree for the one supported target, x86_64 Linux, where `VALUE`, `ID`, `long` and
//! pointers all have 64 bits. The layouts and the constants are checked against the C headers
//! by `tests/layout.rs`, and so is the signature of every function declared here.
//!
//! The names are those `vkxp-ruby-sys` declares for Ruby 1.8.7, so that code written against
//! one compiles against the other; the crate documentation lists what differs.
//!
//! # Safety
//!
//! Every function here must be called on the interpreter thread, after the interpreter has
//! been started, with live Ruby values. Most of them can raise a Ruby exception, which is a
//! `longjmp`: see the crate documentation for the two rules that follow from that.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use std::ffi::{c_char, c_double, c_int, c_long, c_longlong, c_ulong, c_void};

/// A Ruby value: an immediate (fixnum, symbol, `nil`, `true`, `false`) or a pointer to a heap
/// object that starts with an [`RBasic`].
pub type VALUE = usize;

/// An interned name.
pub type ID = usize;

/// `VALUE (*)(ANYARGS)`: a method implementation whose real signature depends on the `argc`
/// it is registered with. Build one with [`method_argv`] or by transmuting a function of the
/// matching C signature.
pub type AnyArgsFn = unsafe extern "C" fn() -> VALUE;

/// The signature Ruby calls for a method registered with `argc == -1`.
pub type ArgvFn = unsafe extern "C" fn(argc: c_int, argv: *const VALUE, recv: VALUE) -> VALUE;

/// `RUBY_DATA_FUNC`: the mark and free callbacks of a `T_DATA` object.
pub type DataFn = unsafe extern "C" fn(data: *mut c_void);

/// The callback of [`rb_protect`].
pub type ProtectFn = unsafe extern "C" fn(arg: VALUE) -> VALUE;

/// Converts a function with the `argc == -1` signature to the type the definition functions
/// take.
pub fn method_argv(function: ArgvFn) -> AnyArgsFn {
    // SAFETY: both are C function pointers of the same size and ABI. Ruby casts the pointer
    // back to the `(int, VALUE *, VALUE)` signature before calling it when the method was
    // registered with `argc == -1`, which is what the callers of this helper do.
    unsafe { std::mem::transmute::<ArgvFn, AnyArgsFn>(function) }
}

// ---------------------------------------------------------------------------------------------
// Special constants and flags
// ---------------------------------------------------------------------------------------------

/// `false`.
pub const Qfalse: VALUE = 0;
/// `true`.
pub const Qtrue: VALUE = 2;
/// `nil`.
pub const Qnil: VALUE = 4;
/// The placeholder for "no value", never visible to Ruby code.
pub const Qundef: VALUE = 6;

/// The low bit that marks a fixnum.
pub const FIXNUM_FLAG: VALUE = 0x01;
/// The two low bits that are zero in every heap pointer.
pub const IMMEDIATE_MASK: VALUE = 0x03;
/// The low byte that marks a symbol.
pub const SYMBOL_FLAG: VALUE = 0x0e;
/// The largest fixnum.
pub const FIXNUM_MAX: c_long = c_long::MAX >> 1;
/// The smallest fixnum.
pub const FIXNUM_MIN: c_long = c_long::MIN >> 1;

pub const T_NONE: c_int = 0x00;
pub const T_OBJECT: c_int = 0x01;
pub const T_CLASS: c_int = 0x02;
pub const T_MODULE: c_int = 0x03;
pub const T_FLOAT: c_int = 0x04;
pub const T_STRING: c_int = 0x05;
pub const T_REGEXP: c_int = 0x06;
pub const T_ARRAY: c_int = 0x07;
pub const T_HASH: c_int = 0x08;
pub const T_STRUCT: c_int = 0x09;
pub const T_BIGNUM: c_int = 0x0a;
pub const T_FILE: c_int = 0x0b;
pub const T_DATA: c_int = 0x0c;
pub const T_MATCH: c_int = 0x0d;
pub const T_COMPLEX: c_int = 0x0e;
pub const T_RATIONAL: c_int = 0x0f;
pub const T_NIL: c_int = 0x11;
pub const T_TRUE: c_int = 0x12;
pub const T_FALSE: c_int = 0x13;
pub const T_SYMBOL: c_int = 0x14;
pub const T_FIXNUM: c_int = 0x15;
pub const T_UNDEF: c_int = 0x1b;
pub const T_NODE: c_int = 0x1c;
pub const T_ICLASS: c_int = 0x1d;
pub const T_ZOMBIE: c_int = 0x1e;
/// The bits of [`RBasic::flags`] that hold the type tag.
pub const T_MASK: c_ulong = 0x1f;

pub const FL_MARK: c_ulong = 1 << 5;
pub const FL_FINALIZE: c_ulong = 1 << 7;
pub const FL_TAINT: c_ulong = 1 << 8;
pub const FL_UNTRUSTED: c_ulong = 1 << 9;
pub const FL_EXIVAR: c_ulong = 1 << 10;
pub const FL_FREEZE: c_ulong = 1 << 11;
/// The first of the flag bits that each type uses for itself.
pub const FL_USHIFT: u32 = 12;
/// On a class: it is a singleton class.
pub const FL_SINGLETON: c_ulong = 1 << FL_USHIFT;
/// On a string or an array: the buffer belongs to another object (`aux.shared`).
pub const ELTS_SHARED: c_ulong = 1 << (FL_USHIFT + 2);
/// On a string: the bytes are in a buffer of their own, not inside the object.
pub const RSTRING_NOEMBED: c_ulong = 1 << (FL_USHIFT + 1);
/// On a string whose bytes are inside the object: the bits that hold the length.
pub const RSTRING_EMBED_LEN_MASK: c_ulong = 0x1f << RSTRING_EMBED_LEN_SHIFT;
/// Where the length of such a string starts in the flags.
pub const RSTRING_EMBED_LEN_SHIFT: u32 = FL_USHIFT + 2;
/// The longest string that is kept inside the object, in bytes.
pub const RSTRING_EMBED_LEN_MAX: usize = 23;
/// On an array: the elements are inside the object.
pub const RARRAY_EMBED_FLAG: c_ulong = 1 << (FL_USHIFT + 1);
/// On an array whose elements are inside the object: the bits that hold the length.
pub const RARRAY_EMBED_LEN_MASK: c_ulong = 0x3 << RARRAY_EMBED_LEN_SHIFT;
/// Where the length of such an array starts in the flags.
pub const RARRAY_EMBED_LEN_SHIFT: u32 = FL_USHIFT + 3;
/// The longest array that is kept inside the object, in elements.
pub const RARRAY_EMBED_LEN_MAX: usize = 3;
/// On a plain object: the instance variables are inside the object.
pub const ROBJECT_EMBED: c_ulong = 1 << (FL_USHIFT + 1);
/// How many instance variables a plain object keeps inside itself.
pub const ROBJECT_EMBED_LEN_MAX: usize = 3;

/// The status [`rb_protect`] reports for a raised exception.
pub const TAG_RAISE: c_int = 0x6;
/// The status [`rb_protect`] reports for a `fatal` error.
pub const TAG_FATAL: c_int = 0x8;

// ---------------------------------------------------------------------------------------------
// Object layouts
// ---------------------------------------------------------------------------------------------

/// The header of every heap object.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RBasic {
    /// The type tag ([`T_MASK`]) and the `FL_*` flags.
    pub flags: c_ulong,
    /// The class, which may be a singleton or an included-module proxy.
    pub klass: VALUE,
}

/// The instance variables of a plain object when they are not inside it.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RObjectHeap {
    /// How many slots `ivptr` has.
    pub numiv: c_long,
    pub ivptr: *mut VALUE,
    /// `struct st_table *`: the table of the class that maps a name to its slot.
    pub iv_index_tbl: *mut c_void,
}

/// The two forms of a plain object, told apart by [`ROBJECT_EMBED`].
#[repr(C)]
#[derive(Clone, Copy)]
pub union RObjectAs {
    pub heap: RObjectHeap,
    pub ary: [VALUE; ROBJECT_EMBED_LEN_MAX],
}

/// A plain object (`T_OBJECT`): up to three instance variables inside the object, or a
/// pointer to more, indexed through a table of the class.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RObject {
    pub basic: RBasic,
    pub r#as: RObjectAs,
}

/// A `Float`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RFloat {
    pub basic: RBasic,
    /// `float_value` in the C header.
    pub value: c_double,
}

/// The layouts of strings and arrays, which have two forms each in Ruby 1.9.
///
/// Read a string or an array through [`rstring_ptr`], [`rstring_len`], [`rarray_ptr`] and
/// [`rarray_len`], which choose the form as the macros of the C header do. The types are
/// public for `tests/layout.rs`, which compares them with the header.
pub mod layout {
    use std::ffi::{c_char, c_long};

    use super::{RARRAY_EMBED_LEN_MAX, RBasic, RSTRING_EMBED_LEN_MAX, VALUE};

    /// The capacity of an owned buffer, or the owner of a shared one
    /// ([`ELTS_SHARED`](super::ELTS_SHARED)).
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union RAux {
        pub capa: c_long,
        pub shared: VALUE,
    }

    /// A string whose bytes are in a buffer of their own.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct RStringHeap {
        pub len: c_long,
        pub ptr: *mut c_char,
        pub aux: RAux,
    }

    /// The two forms of a string, told apart by [`RSTRING_NOEMBED`](super::RSTRING_NOEMBED).
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union RStringAs {
        pub heap: RStringHeap,
        pub ary: [c_char; RSTRING_EMBED_LEN_MAX + 1],
    }

    /// `struct RString`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct RString {
        pub basic: RBasic,
        pub r#as: RStringAs,
    }

    /// An array whose elements are in a buffer of their own. Note that `aux` and `ptr` are
    /// in the opposite order to [`RStringHeap`].
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct RArrayHeap {
        pub len: c_long,
        pub aux: RAux,
        pub ptr: *mut VALUE,
    }

    /// The two forms of an array, told apart by
    /// [`RARRAY_EMBED_FLAG`](super::RARRAY_EMBED_FLAG).
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub union RArrayAs {
        pub heap: RArrayHeap,
        pub ary: [VALUE; RARRAY_EMBED_LEN_MAX],
    }

    /// `struct RArray`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct RArray {
        pub basic: RBasic,
        pub r#as: RArrayAs,
    }
}

/// An encoding (`rb_encoding`, which is `OnigEncodingType`). Only pointers to it are used.
#[repr(C)]
pub struct rb_encoding {
    _opaque: [u8; 0],
}

/// A wrapped host pointer (`T_DATA`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RData {
    pub basic: RBasic,
    /// Called during the mark phase with `data`; marks the values the host object refers to.
    pub dmark: Option<DataFn>,
    /// Called when the object is collected.
    pub dfree: Option<DataFn>,
    pub data: *mut c_void,
}

// ---------------------------------------------------------------------------------------------
// Helpers that are macros or inline functions in C
// ---------------------------------------------------------------------------------------------

/// `FIXNUM_P`: is the value a fixnum?
#[inline]
pub const fn fixnum_p(value: VALUE) -> bool {
    value & FIXNUM_FLAG != 0
}

/// `FIXABLE`: does the integer fit a fixnum?
#[inline]
pub const fn fixable(integer: c_long) -> bool {
    integer >= FIXNUM_MIN && integer <= FIXNUM_MAX
}

/// `INT2FIX` / `LONG2FIX`: the fixnum for an integer. The caller checks [`fixable`]; an
/// integer outside the range loses its top bit.
#[inline]
pub const fn long2fix(integer: c_long) -> VALUE {
    ((integer as VALUE) << 1) | FIXNUM_FLAG
}

/// `FIX2LONG`: the integer of a fixnum.
#[inline]
pub const fn fix2long(value: VALUE) -> c_long {
    (value as c_long) >> 1
}

/// `SYMBOL_P`: is the value a symbol?
#[inline]
pub const fn symbol_p(value: VALUE) -> bool {
    value & 0xff == SYMBOL_FLAG
}

/// `ID2SYM`: the symbol for an interned name.
#[inline]
pub const fn id2sym(id: ID) -> VALUE {
    (id << 8) | SYMBOL_FLAG
}

/// `SYM2ID`: the interned name of a symbol.
#[inline]
pub const fn sym2id(symbol: VALUE) -> ID {
    symbol >> 8
}

/// `NIL_P`.
#[inline]
pub const fn nil_p(value: VALUE) -> bool {
    value == Qnil
}

/// `RTEST`: is the value neither `nil` nor `false`?
#[inline]
pub const fn rtest(value: VALUE) -> bool {
    value & !Qnil != 0
}

/// `IMMEDIATE_P`: is the value a fixnum, a symbol, `true` or `Qundef`?
#[inline]
pub const fn immediate_p(value: VALUE) -> bool {
    value & IMMEDIATE_MASK != 0
}

/// `SPECIAL_CONST_P`: is the value anything but a pointer to a heap object?
#[inline]
pub const fn special_const_p(value: VALUE) -> bool {
    immediate_p(value) || !rtest(value)
}

/// `BUILTIN_TYPE`: the type tag of a heap object.
///
/// # Safety
///
/// `value` must be a live heap object, not a special constant.
#[inline]
pub unsafe fn builtin_type(value: VALUE) -> c_int {
    // SAFETY: the caller guarantees `value` points to a live object, which starts with RBasic.
    let flags = unsafe { (*(value as *const RBasic)).flags };
    (flags & T_MASK) as c_int
}

/// `TYPE` / `rb_type`: the type tag of any value.
///
/// # Safety
///
/// `value` must be a special constant or a live heap object.
#[inline]
pub unsafe fn rb_type(value: VALUE) -> c_int {
    if fixnum_p(value) {
        T_FIXNUM
    } else if value == Qnil {
        T_NIL
    } else if value == Qfalse {
        T_FALSE
    } else if value == Qtrue {
        T_TRUE
    } else if value == Qundef {
        T_UNDEF
    } else if symbol_p(value) {
        T_SYMBOL
    } else {
        // SAFETY: every special constant was handled above, so this is a live heap object.
        unsafe { builtin_type(value) }
    }
}

/// `CLASS_OF` / `rb_class_of`: the class of any value, which may be a singleton class.
///
/// # Safety
///
/// `value` must be a special constant other than `Qundef`, or a live heap object, and the
/// interpreter must be initialised.
#[inline]
pub unsafe fn rb_class_of(value: VALUE) -> VALUE {
    // SAFETY: the class globals are written once by `ruby_init`, which has run; the last arm
    // reads the header of what the caller guarantees is a live heap object.
    unsafe {
        if fixnum_p(value) {
            rb_cFixnum
        } else if value == Qnil {
            rb_cNilClass
        } else if value == Qfalse {
            rb_cFalseClass
        } else if value == Qtrue {
            rb_cTrueClass
        } else if symbol_p(value) {
            rb_cSymbol
        } else {
            (*(value as *const RBasic)).klass
        }
    }
}

/// `LONG2NUM`: a fixnum when the integer fits, a `Bignum` otherwise.
///
/// # Safety
///
/// Interpreter thread only; may allocate.
#[inline]
pub unsafe fn long2num(integer: c_long) -> VALUE {
    if fixable(integer) {
        long2fix(integer)
    } else {
        // SAFETY: the caller is on the interpreter thread.
        unsafe { rb_int2inum(integer) }
    }
}

/// `NUM2LONG`: the integer of a numeric value. Raises `TypeError` or `RangeError` otherwise.
///
/// # Safety
///
/// Interpreter thread only; `value` must be live; may raise.
#[inline]
pub unsafe fn num2long(value: VALUE) -> c_long {
    if fixnum_p(value) {
        fix2long(value)
    } else {
        // SAFETY: the caller guarantees a live value on the interpreter thread.
        unsafe { rb_num2long(value) }
    }
}

/// `RSTRING_LEN`: the length of a string in bytes, for the embedded and the heap form.
///
/// # Safety
///
/// `value` must be a live `T_STRING`.
#[inline]
pub unsafe fn rstring_len(value: VALUE) -> c_long {
    // SAFETY: the caller guarantees a live string, whose layout is `layout::RString`; the
    // flag says which member of the union is in use.
    unsafe {
        let string = value as *const layout::RString;
        let flags = (*string).basic.flags;
        if flags & RSTRING_NOEMBED == 0 {
            ((flags & RSTRING_EMBED_LEN_MASK) >> RSTRING_EMBED_LEN_SHIFT) as c_long
        } else {
            (*string).r#as.heap.len
        }
    }
}

/// `RSTRING_PTR`: the bytes of a string, for the embedded and the heap form. The bytes are
/// not necessarily NUL-terminated text: [`rstring_len`] rules.
///
/// # Safety
///
/// `value` must be a live `T_STRING`. The pointer is valid until the string is modified or
/// collected; for the embedded form it points into the object itself.
#[inline]
pub unsafe fn rstring_ptr(value: VALUE) -> *mut c_char {
    // SAFETY: as for `rstring_len`. The embedded bytes are addressed without a reference to
    // the object being made.
    unsafe {
        let string = value as *mut layout::RString;
        if (*string).basic.flags & RSTRING_NOEMBED == 0 {
            (&raw mut (*string).r#as.ary).cast::<c_char>()
        } else {
            (*string).r#as.heap.ptr
        }
    }
}

/// `RARRAY_LEN`: the number of elements of an array, for the embedded and the heap form.
///
/// # Safety
///
/// `value` must be a live `T_ARRAY`.
#[inline]
pub unsafe fn rarray_len(value: VALUE) -> c_long {
    // SAFETY: the caller guarantees a live array, whose layout is `layout::RArray`; the
    // flag says which member of the union is in use.
    unsafe {
        let array = value as *const layout::RArray;
        let flags = (*array).basic.flags;
        if flags & RARRAY_EMBED_FLAG != 0 {
            ((flags & RARRAY_EMBED_LEN_MASK) >> RARRAY_EMBED_LEN_SHIFT) as c_long
        } else {
            (*array).r#as.heap.len
        }
    }
}

/// `RARRAY_PTR`: the elements of an array, for the embedded and the heap form.
///
/// # Safety
///
/// `value` must be a live `T_ARRAY`. The pointer is valid until the array is modified or
/// collected; for the embedded form it points into the object itself.
#[inline]
pub unsafe fn rarray_ptr(value: VALUE) -> *mut VALUE {
    // SAFETY: as for `rarray_len`. The embedded elements are addressed without a reference
    // to the object being made.
    unsafe {
        let array = value as *mut layout::RArray;
        if (*array).basic.flags & RARRAY_EMBED_FLAG != 0 {
            (&raw mut (*array).r#as.ary).cast::<VALUE>()
        } else {
            (*array).r#as.heap.ptr
        }
    }
}

/// `DATA_PTR(value)`: the host pointer of a wrapped object.
///
/// # Safety
///
/// `value` must be a live `T_DATA`.
#[inline]
pub unsafe fn data_ptr(value: VALUE) -> *mut c_void {
    // SAFETY: the caller guarantees a live T_DATA, whose layout is RData.
    unsafe { (*(value as *const RData)).data }
}

// ---------------------------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------------------------

// The interpreter writes all of these, most of them once during `ruby_init`, so they are
// declared mutable. Read them by value on the interpreter thread.
unsafe extern "C" {
    pub static mut rb_mKernel: VALUE;
    pub static mut rb_mComparable: VALUE;
    pub static mut rb_mEnumerable: VALUE;
    pub static mut rb_mErrno: VALUE;
    pub static mut rb_mFileTest: VALUE;
    pub static mut rb_mGC: VALUE;
    pub static mut rb_mMath: VALUE;
    pub static mut rb_mProcess: VALUE;

    pub static mut rb_cObject: VALUE;
    pub static mut rb_cArray: VALUE;
    pub static mut rb_cBignum: VALUE;
    pub static mut rb_cClass: VALUE;
    pub static mut rb_cData: VALUE;
    pub static mut rb_cFalseClass: VALUE;
    pub static mut rb_cFile: VALUE;
    pub static mut rb_cFixnum: VALUE;
    pub static mut rb_cFloat: VALUE;
    pub static mut rb_cHash: VALUE;
    pub static mut rb_cInteger: VALUE;
    pub static mut rb_cIO: VALUE;
    pub static mut rb_cModule: VALUE;
    pub static mut rb_cNilClass: VALUE;
    pub static mut rb_cNumeric: VALUE;
    pub static mut rb_cProc: VALUE;
    pub static mut rb_cRange: VALUE;
    pub static mut rb_cRegexp: VALUE;
    pub static mut rb_cString: VALUE;
    pub static mut rb_cStruct: VALUE;
    pub static mut rb_cSymbol: VALUE;
    pub static mut rb_cThread: VALUE;
    pub static mut rb_cTime: VALUE;
    pub static mut rb_cTrueClass: VALUE;

    pub static mut rb_eException: VALUE;
    pub static mut rb_eStandardError: VALUE;
    pub static mut rb_eSystemExit: VALUE;
    pub static mut rb_eInterrupt: VALUE;
    pub static mut rb_eSignal: VALUE;
    pub static mut rb_eFatal: VALUE;
    pub static mut rb_eArgError: VALUE;
    pub static mut rb_eEOFError: VALUE;
    pub static mut rb_eIndexError: VALUE;
    pub static mut rb_eRangeError: VALUE;
    pub static mut rb_eIOError: VALUE;
    pub static mut rb_eRuntimeError: VALUE;
    pub static mut rb_eSecurityError: VALUE;
    pub static mut rb_eSystemCallError: VALUE;
    pub static mut rb_eThreadError: VALUE;
    pub static mut rb_eTypeError: VALUE;
    pub static mut rb_eZeroDivError: VALUE;
    pub static mut rb_eNotImpError: VALUE;
    pub static mut rb_eNoMemError: VALUE;
    pub static mut rb_eNoMethodError: VALUE;
    pub static mut rb_eFloatDomainError: VALUE;
    pub static mut rb_eLocalJumpError: VALUE;
    pub static mut rb_eSysStackError: VALUE;
    pub static mut rb_eRegexpError: VALUE;
    pub static mut rb_eScriptError: VALUE;
    pub static mut rb_eNameError: VALUE;
    pub static mut rb_eSyntaxError: VALUE;
    pub static mut rb_eLoadError: VALUE;

    pub static mut rb_stdin: VALUE;
    pub static mut rb_stdout: VALUE;
    pub static mut rb_stderr: VALUE;

    /// When not 0 before [`ruby_init`]: no timer thread is created while the main thread is
    /// the only Ruby thread. Added to `thread_pthread.c` by a vkxp patch.
    pub static mut ruby_vkxp_lazy_timer_thread: c_int;
    /// When not 0 before [`ruby_init`]: the seed of `String#hash` and the other hashes is
    /// [`ruby_vkxp_hashseed`]. Added to `random.c` by a vkxp patch.
    pub static mut ruby_vkxp_hashseed_fixed: c_int;
    /// The fixed seed of the hashes. Added to `random.c` by a vkxp patch.
    pub static mut ruby_vkxp_hashseed: c_ulong;
    /// When not 0 before [`ruby_init_stack`]: the size of the machine stack the interpreter
    /// may use, in bytes below the address given there. Added to `thread_pthread.c` by a vkxp
    /// patch.
    pub static mut ruby_vkxp_stack_size: usize;
}

// ---------------------------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------------------------

unsafe extern "C" {
    // --- Start and stop -----------------------------------------------------------------------

    /// Records the address as the base of the stack that the collector scans, and the size
    /// of that stack (see [`ruby_vkxp_stack_size`]). It must lie in a frame that encloses
    /// every frame that will ever hold a Ruby value.
    pub fn ruby_init_stack(addr: *mut VALUE);
    /// Initialises the interpreter. Replaces the process's signal handlers and blocks every
    /// signal on the calling thread.
    pub fn ruby_init();
    /// Runs the initialisation function of a compiled-in extension as if `name` had been
    /// required, and marks `name` as loaded. Exported by `load.c` but declared in no header.
    pub fn ruby_init_ext(name: *const c_char, init: unsafe extern "C" fn());
    /// Sets `$0`.
    pub fn ruby_script(name: *const c_char);
    /// Runs `at_exit` blocks and finalisers. Ruby threads other than the calling one keep
    /// running; [`ruby_cleanup`] is what ends an interpreter.
    pub fn ruby_finalize();
    /// Ends the interpreter: runs `at_exit` blocks, terminates the other Ruby threads, runs
    /// finalisers and stops the timer thread. Returns the exit status for `status`. The
    /// interpreter cannot be used afterwards.
    pub fn ruby_cleanup(status: c_int) -> c_int;
    /// The initialisation function of the compiled-in `Zlib` extension.
    pub fn Init_zlib();
    /// Registers the encoding UTF-16LE.
    pub fn Init_utf_16le();
    /// Registers the transcoders between UTF-8 and Shift_JIS and Windows-31J.
    pub fn Init_japanese_sjis();
    /// Registers the transcoders between UTF-8 and the single-byte encodings.
    pub fn Init_single_byte();
    /// Registers the transcoders between UTF-8 and UTF-16 and UTF-32.
    pub fn Init_utf_16_32();

    // --- Definition ---------------------------------------------------------------------------

    pub fn rb_define_class(name: *const c_char, superclass: VALUE) -> VALUE;
    pub fn rb_define_module(name: *const c_char) -> VALUE;
    pub fn rb_define_class_under(outer: VALUE, name: *const c_char, superclass: VALUE) -> VALUE;
    pub fn rb_define_module_under(outer: VALUE, name: *const c_char) -> VALUE;
    pub fn rb_define_method(class: VALUE, name: *const c_char, function: AnyArgsFn, argc: c_int);
    pub fn rb_define_private_method(
        class: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    );
    pub fn rb_define_singleton_method(
        object: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    );
    pub fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        function: AnyArgsFn,
        argc: c_int,
    );
    pub fn rb_define_global_function(name: *const c_char, function: AnyArgsFn, argc: c_int);
    pub fn rb_define_alloc_func(class: VALUE, function: unsafe extern "C" fn(VALUE) -> VALUE);
    pub fn rb_define_const(class: VALUE, name: *const c_char, value: VALUE);
    pub fn rb_define_global_const(name: *const c_char, value: VALUE);
    pub fn rb_define_attr(class: VALUE, name: *const c_char, read: c_int, write: c_int);
    pub fn rb_define_alias(class: VALUE, new_name: *const c_char, old_name: *const c_char);
    pub fn rb_undef_method(class: VALUE, name: *const c_char);
    pub fn rb_include_module(class: VALUE, module: VALUE);
    pub fn rb_extend_object(object: VALUE, module: VALUE);
    pub fn rb_singleton_class(object: VALUE) -> VALUE;

    // --- Names, classes and constants ---------------------------------------------------------

    pub fn rb_intern(name: *const c_char) -> ID;
    pub fn rb_id2name(id: ID) -> *const c_char;
    pub fn rb_to_id(name: VALUE) -> ID;
    pub fn rb_obj_class(object: VALUE) -> VALUE;
    pub fn rb_class2name(class: VALUE) -> *const c_char;
    pub fn rb_obj_classname(object: VALUE) -> *const c_char;
    pub fn rb_class_path(class: VALUE) -> VALUE;
    pub fn rb_path2class(path: *const c_char) -> VALUE;
    pub fn rb_obj_is_kind_of(object: VALUE, class: VALUE) -> VALUE;
    pub fn rb_obj_is_instance_of(object: VALUE, class: VALUE) -> VALUE;
    pub fn rb_respond_to(object: VALUE, method: ID) -> c_int;
    pub fn rb_const_get(class: VALUE, name: ID) -> VALUE;
    pub fn rb_const_set(class: VALUE, name: ID, value: VALUE);
    pub fn rb_const_defined(class: VALUE, name: ID) -> c_int;

    // --- Calling ------------------------------------------------------------------------------

    /// Calls a method, ignoring its visibility. The arguments are `argc` values.
    pub fn rb_funcall(recv: VALUE, method: ID, argc: c_int, ...) -> VALUE;
    /// Calls a method, ignoring its visibility.
    pub fn rb_funcall2(recv: VALUE, method: ID, argc: c_int, argv: *const VALUE) -> VALUE;
    /// Calls a public method.
    pub fn rb_funcall3(recv: VALUE, method: ID, argc: c_int, argv: *const VALUE) -> VALUE;
    pub fn rb_call_super(argc: c_int, argv: *const VALUE) -> VALUE;
    pub fn rb_class_new_instance(argc: c_int, argv: *mut VALUE, class: VALUE) -> VALUE;
    pub fn rb_obj_alloc(class: VALUE) -> VALUE;
    pub fn rb_block_given_p() -> c_int;
    pub fn rb_block_proc() -> VALUE;
    pub fn rb_yield(value: VALUE) -> VALUE;
    pub fn rb_eval_string(source: *const c_char) -> VALUE;
    pub fn rb_eval_string_protect(source: *const c_char, state: *mut c_int) -> VALUE;
    pub fn rb_equal(left: VALUE, right: VALUE) -> VALUE;
    pub fn rb_eql(left: VALUE, right: VALUE) -> c_int;
    pub fn rb_obj_id(object: VALUE) -> VALUE;
    pub fn rb_obj_dup(object: VALUE) -> VALUE;
    pub fn rb_obj_clone(object: VALUE) -> VALUE;
    pub fn rb_obj_freeze(object: VALUE) -> VALUE;
    pub fn rb_check_frozen(object: VALUE);

    // --- Conversion ---------------------------------------------------------------------------

    pub fn rb_num2long(value: VALUE) -> c_long;
    pub fn rb_num2ulong(value: VALUE) -> c_ulong;
    pub fn rb_num2ll(value: VALUE) -> c_longlong;
    pub fn rb_num2dbl(value: VALUE) -> c_double;
    pub fn rb_int2inum(integer: c_long) -> VALUE;
    pub fn rb_uint2inum(integer: c_ulong) -> VALUE;
    pub fn rb_ll2inum(integer: c_longlong) -> VALUE;
    pub fn rb_float_new(value: c_double) -> VALUE;
    pub fn rb_Integer(value: VALUE) -> VALUE;
    pub fn rb_Float(value: VALUE) -> VALUE;
    pub fn rb_String(value: VALUE) -> VALUE;
    pub fn rb_Array(value: VALUE) -> VALUE;
    pub fn rb_check_type(value: VALUE, type_tag: c_int);
    pub fn rb_check_string_type(value: VALUE) -> VALUE;
    pub fn rb_check_array_type(value: VALUE) -> VALUE;

    // --- Strings ------------------------------------------------------------------------------

    /// A new string of these bytes, labelled ASCII-8BIT.
    pub fn rb_str_new(bytes: *const c_char, len: c_long) -> VALUE;
    /// A new string of these bytes, labelled with `encoding`.
    pub fn rb_enc_str_new(bytes: *const c_char, len: c_long, encoding: *mut rb_encoding) -> VALUE;
    /// The encoding UTF-8.
    pub fn rb_utf8_encoding() -> *mut rb_encoding;
    /// The `Encoding` object of an encoding.
    pub fn rb_enc_from_encoding(encoding: *mut rb_encoding) -> VALUE;
    /// `Encoding.default_external = encoding`.
    pub fn rb_enc_set_default_external(encoding: VALUE);
    pub fn rb_str_new2(text: *const c_char) -> VALUE;
    pub fn rb_str_buf_new(capacity: c_long) -> VALUE;
    pub fn rb_str_cat(string: VALUE, bytes: *const c_char, len: c_long) -> VALUE;
    pub fn rb_str_append(string: VALUE, other: VALUE) -> VALUE;
    pub fn rb_str_dup(string: VALUE) -> VALUE;
    pub fn rb_str_resize(string: VALUE, len: c_long) -> VALUE;
    pub fn rb_str_intern(string: VALUE) -> VALUE;
    /// `StringValue`: converts `*value` to a string in place and returns it.
    pub fn rb_string_value(value: *mut VALUE) -> VALUE;
    /// `StringValuePtr`: as [`rb_string_value`], returning the bytes.
    pub fn rb_string_value_ptr(value: *mut VALUE) -> *mut c_char;
    /// `StringValueCStr`: as [`rb_string_value_ptr`], raising if the string contains a NUL.
    pub fn rb_string_value_cstr(value: *mut VALUE) -> *mut c_char;
    pub fn rb_obj_as_string(object: VALUE) -> VALUE;
    pub fn rb_inspect(object: VALUE) -> VALUE;
    pub fn rb_any_to_s(object: VALUE) -> VALUE;

    // --- Arrays and hashes --------------------------------------------------------------------

    pub fn rb_ary_new() -> VALUE;
    pub fn rb_ary_new2(capacity: c_long) -> VALUE;
    pub fn rb_ary_new4(len: c_long, elements: *const VALUE) -> VALUE;
    pub fn rb_ary_push(array: VALUE, element: VALUE) -> VALUE;
    pub fn rb_ary_pop(array: VALUE) -> VALUE;
    pub fn rb_ary_shift(array: VALUE) -> VALUE;
    pub fn rb_ary_unshift(array: VALUE, element: VALUE) -> VALUE;
    pub fn rb_ary_entry(array: VALUE, index: c_long) -> VALUE;
    pub fn rb_ary_store(array: VALUE, index: c_long, element: VALUE);
    pub fn rb_ary_clear(array: VALUE) -> VALUE;
    pub fn rb_ary_dup(array: VALUE) -> VALUE;
    pub fn rb_ary_concat(array: VALUE, other: VALUE) -> VALUE;
    pub fn rb_hash_new() -> VALUE;
    pub fn rb_hash_aref(hash: VALUE, key: VALUE) -> VALUE;
    pub fn rb_hash_aset(hash: VALUE, key: VALUE, value: VALUE) -> VALUE;
    pub fn rb_hash_delete(hash: VALUE, key: VALUE) -> VALUE;

    // --- Exceptions and protection ------------------------------------------------------------

    /// Raises a new exception of `class` with a `printf`-style message. Does not return: it
    /// jumps to the innermost handler.
    pub fn rb_raise(class: VALUE, format: *const c_char, ...) -> !;
    /// Raises an exception object. Does not return.
    pub fn rb_exc_raise(exception: VALUE) -> !;
    /// Continues a jump that [`rb_protect`] stopped. Does not return.
    pub fn rb_jump_tag(state: c_int) -> !;
    pub fn rb_exc_new(class: VALUE, message: *const c_char, len: c_long) -> VALUE;
    pub fn rb_exc_new2(class: VALUE, message: *const c_char) -> VALUE;
    pub fn rb_exc_new3(class: VALUE, message: VALUE) -> VALUE;
    /// `$!`: the exception being handled, or `nil`. After [`rb_protect`] reports a raise
    /// this is the exception; set it back with [`rb_set_errinfo`] once it has been taken.
    pub fn rb_errinfo() -> VALUE;
    /// Sets `$!`. The value must be `nil` or an exception, or the call raises `TypeError`.
    pub fn rb_set_errinfo(exception: VALUE);
    /// The `main` object, `self` at the top level. Defined by `vm.c`; declared in no public
    /// header.
    pub fn rb_vm_top_self() -> VALUE;
    /// Calls `function(arg)` and stops any jump out of it. `*state` is 0 when it returned
    /// normally; otherwise the result is `nil` and, for a raise, [`rb_errinfo`] is the
    /// exception.
    pub fn rb_protect(function: ProtectFn, arg: VALUE, state: *mut c_int) -> VALUE;
    pub fn rb_ensure(
        body: AnyArgsFn,
        body_arg: VALUE,
        ensure: AnyArgsFn,
        ensure_arg: VALUE,
    ) -> VALUE;
    pub fn rb_notimplement() -> !;

    // --- Garbage collection -------------------------------------------------------------------

    /// Makes the collector treat `*address` as a root until it is unregistered. The address
    /// must stay valid for that long.
    pub fn rb_gc_register_address(address: *mut VALUE);
    pub fn rb_gc_unregister_address(address: *mut VALUE);
    /// As [`rb_gc_register_address`], for an address that is never unregistered.
    pub fn rb_global_variable(address: *mut VALUE);
    /// Marks a value as reachable. Only for use inside a `dmark` callback.
    pub fn rb_gc_mark(value: VALUE);
    /// Runs a collection now.
    pub fn rb_gc();
    /// `GC.start`.
    pub fn rb_gc_start() -> VALUE;
    /// `Data_Wrap_Struct`: wraps a host pointer in a new `T_DATA` object of `class`.
    pub fn rb_data_object_alloc(
        class: VALUE,
        data: *mut c_void,
        mark: Option<DataFn>,
        free: Option<DataFn>,
    ) -> VALUE;

    // --- Variables ----------------------------------------------------------------------------

    pub fn rb_gv_get(name: *const c_char) -> VALUE;
    pub fn rb_gv_set(name: *const c_char, value: VALUE) -> VALUE;
    pub fn rb_iv_get(object: VALUE, name: *const c_char) -> VALUE;
    pub fn rb_iv_set(object: VALUE, name: *const c_char, value: VALUE) -> VALUE;
    pub fn rb_ivar_get(object: VALUE, name: ID) -> VALUE;
    pub fn rb_ivar_set(object: VALUE, name: ID, value: VALUE) -> VALUE;
    pub fn rb_ivar_defined(object: VALUE, name: ID) -> VALUE;
    /// As [`rb_ivar_get`], without the warning for an unset variable.
    pub fn rb_attr_get(object: VALUE, name: ID) -> VALUE;

    // --- Marshal ------------------------------------------------------------------------------

    /// `Marshal.dump(object, port)`; with `port` `nil` the result is a string.
    pub fn rb_marshal_dump(object: VALUE, port: VALUE) -> VALUE;
    /// `Marshal.load(port)`, where `port` is a string or an IO.
    pub fn rb_marshal_load(port: VALUE) -> VALUE;
}
