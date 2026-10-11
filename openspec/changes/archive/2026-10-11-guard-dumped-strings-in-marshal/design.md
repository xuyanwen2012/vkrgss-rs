# Design

## Decisions

### The patch

`RB_GC_GUARD(x)` (defined in `ruby.h` of this release) is added after each call in
`marshal.c` that passes `RSTRING(x)->ptr` to a function that can allocate while `x` is not
used afterwards. Known places: `w_object` at the `_dump` result (`v`), the two
`class2path` results (`path`), and the others the same reading finds; the string case
(`obj`) needs none if `obj` is used or rooted afterwards, which is to be checked and said.
The patch follows the procedure of `PATCHES.md`: a diff under `patches/`, an entry with the
four fields, the patched hash.

### The audit of the other files

The files the build script compiles are searched for the pattern: a pointer taken from a
value (`RSTRING(x)->ptr`, `RARRAY(x)->ptr`, `StringValuePtr`, `StringValueCStr`) that is
used after a call that can allocate, while `x` itself is not used after that call. This is
a reading task with a bound: `marshal.c` completely; in the other files the functions that
a game's save and load path runs (`pack.c`, `string.c` functions called from `marshal.c`,
`io.c` write path, `ext/zlib/zlib.c` deflate and inflate entry points). What is found is
patched in the same way, each file with its own patch entry; what was read and found clean
is listed under "Found" by file and function. A finding outside that bound is listed under
"Follow-ups" and not patched.

### Reproduction with another compiler

The failure does not show with the gcc of the development machine. It shows in a container:

    podman run --rm --security-opt label=disable -v "$PWD":/src \
        -v "$HOME/.cargo":/cargo -v "$HOME/.rustup":/rustup:ro ubuntu:24.04 \
        bash -c 'apt-get update -qq && apt-get install -y -qq --no-install-recommends \
            build-essential libcrypt-dev ca-certificates >/dev/null &&
          export PATH=/cargo/bin:$PATH CARGO_HOME=/cargo RUSTUP_HOME=/rustup \
            CARGO_TARGET_DIR=/src/target/ubuntu-gcc && cd /src &&
          cargo test -p vkxp-binding -p vkxp-ruby-sys --locked --offline'

If the sandbox of the run cannot start a container, that is said under "Found", the tests
are run with the local gcc and with `CC=clang`, and the owner runs the container after the
run. `docs/development.md` gains a short section "Testing with another compiler" with this
command, since the same question will come up again.

### A test that does not depend on the compiler

Besides the stress test, one test that fails without the patch whatever the compiler does
with `v`, if one can be written: for example a `_dump` that returns a fresh string while
the dump buffer is made to grow with stress on, in the upstream-style Ruby tests of
`vkxp-ruby-sys` or in `vkxp-binding`. If the local compiler keeps `v` alive regardless, the
test is still added, and "Found" says that only the container shows the failure.

## Found

### The container

The sandbox of the run cannot start a container: `podman run` ends with `acquiring runtime
init lock: open /run/user/1000/libpod/tmp/alive.lck: permission denied`. The scenario
"Table under stress" was therefore not run with gcc 13; the tests were run with the local
gcc 16.2.1 and with clang 22.1.8 (`CC=clang`, its own `CARGO_TARGET_DIR`), and the owner
runs the command of "Reproduction with another compiler" after the run.

### When the collection happens

`ruby_xrealloc` collects only with `GC.stress` on or when `realloc` fails; `ruby_xmalloc`
and `rb_newobj` collect in normal operation. Growing the dump buffer is a `realloc`, so a
dump to a string loses the `_dump` result only under stress. A dump to a port hands the
buffer to `write` whenever it holds `BUFSIZ` (8192) bytes, and `write` is Ruby code that
allocates: when that point falls on the length written in front of the `_dump` result, the
string can be lost without stress. That is the case a saved game can meet.

### `marshal.c`, read completely

Guarded (patch 23, `marshal-gc-guard`):

- `w_object`, `_dump` result `v`: the known place. The guard stands after `w_ivar`, because
  the instance variables written there can be those of `v`.
- `w_object`, cases `T_CLASS` and `T_MODULE`, `path`: as in "The patch".
- `w_class` and `w_uclass`, the result of `class2path`: the same form, with the value now
  held in a local. The class holds its path in an instance variable, so these two were not
  reachable; they are guarded so that the file has one form.
- `r_symreal`, the name read by `r_bytes` and passed to `rb_intern`: `rb_intern` copies a
  new name with `ruby_strdup`, which allocates before it copies, and nothing refers to the
  string. A symbol loaded for the first time while a collection runs got a name of freed
  bytes. This one fails with the local gcc and with clang as well.
- `marshal_load`, `port`: `struct load_arg` is heap memory that `mark_load_arg` does not
  mark for `src`, so when the source is the result of `to_str` only the local refers to it.

Clean:

- `w_object`, case `T_STRING` (`obj`): `obj` is not used afterwards, but it was entered in
  `arg->data` at the top of the branch, and `mark_dump_arg` marks the keys of that table
  through the wrapper that `marshal_dump` guards. The same holds for the buffers of
  `T_REGEXP`, `T_BIGNUM` and `T_ARRAY`. No guard.
- `w_object`, `marshal_dump` result and `_dump_data` result: passed as values to
  `w_object`, which enters them in `arg->data`.
- `w_object`, case `T_STRUCT` (`mem`) and `r_object0`, case `TYPE_STRUCT` (`mem`): used in
  every round of the loop, and held by the class.
- `class2path`: `path` is returned. `w_extended`, `w_symbol`, `r_unique`: names from
  `rb_class2name` and `rb_id2name`, which the class and the symbol table hold.
- `w_nbyte`, `marshal_dump`: `arg->str` is marked by `mark_dump_arg`, the port is returned.
- `r_byte`, `r_bytes0`: read `arg->src` (see `port` above) or return the string they read.
- `r_object0`: `TYPE_FLOAT` does not allocate before the last read of the string;
  `TYPE_BIGNUM`, `TYPE_REGEXP`, `TYPE_MODULE_OLD`, `TYPE_CLASS` and `TYPE_MODULE` hold the
  string in a `volatile VALUE`, the form of this release; `TYPE_USERDEF` passes the string
  to `_load` as a value; every object made is entered in `arg->data` by `r_entry`, whose
  values `mark_load_arg` marks.

### The test

`a_dumped_string_survives_a_collection_while_it_is_written` and
`a_loaded_symbol_survives_a_collection_while_it_is_interned` in
`crates/vkxp-binding/tests/stress.rs`. Without the patch the second fails with the local
gcc and with clang. The first passes without the patch with both, to a string under stress
and to a port that collects in `write`: both compilers keep `v` alive, so only the
container shows that failure.

While writing the port: `Marshal.dump` passes its buffer to `write` as a string without a
class, so a `write` in Ruby that calls a method on its argument raises "method called on
terminated object". That is the behaviour of the release and not touched.

### The other files

Patched:

- `pack.c` (patch 24, `pack-gc-guard`). `pack_pack`, directive `w`, `buf`: a local that
  nothing else refers to, read by `rb_str_buf_cat` after the result has grown. The one
  place outside `marshal.c` with the full pattern; growing is a `realloc`, so it needs
  stress or a failed allocation.
- `pack.c`, same patch: `pack_pack`, `from` of the directives `a A Z b B h H`, `u m` and
  `M` (through `qpencode`), and `pack_unpack`, `fmt`. Each matters only when the value was
  converted from another object, and each is a local whose address was passed to
  `rb_string_value`, which a compiler that compiles one file at a time has to keep in its
  slot. Guarded so that this does not rest on how the files are compiled.
- `ext/zlib/zlib.c` (patch 25, `zlib-gc-guard`): `do_deflate` and `do_inflate`, `src`
  handed to `zstream_run` as the last statement; `rb_inflate_inflate` and
  `rb_inflate_addstr`, `src` handed to `zstream_append_buffer` on a finished stream. The
  same condition and the same reservation as `from` above.

Read and found clean:

- `pack.c`: `pack_pack` otherwise (`fmt` is used at the end; elements are read again from
  the array by each `NEXTFROM`; `from` of `p P` is kept in `associates`; `res` is
  returned); `encodes` (reads its source before the one `rb_str_buf_cat`); `pack_unpack`
  otherwise (`str` is the receiver; `bitstr` and `buf` are pushed or used after the last
  read of their buffer); `infected_str_new`.
- `string.c`: `rb_str_new`, `str_new`, `str_alloc`, `rb_str_buf_new`, `rb_str_buf_cat`,
  `str_buf_cat`, `rb_str_resize`, `rb_str_modify`, `str_independent`,
  `str_make_independent`, `rb_string_value`, `rb_string_value_ptr`,
  `rb_string_value_cstr`, `rb_str_to_str`, `rb_check_string_type`, `rb_str_associate`,
  `rb_str_associated`. Each uses or returns its own string after it allocates and reads
  the buffer pointer again. `str_new` and `str_buf_cat` read the pointer they were given
  after they allocate, which is why their callers have to keep the value.
- `io.c`: `rb_io_write`, `io_write`, `rb_io_fwrite`, `io_fwrite` (reads `RSTRING(str)->ptr`
  again at every write and uses `str` for the retry), `rb_io_binmode`, `rb_io_getc`,
  `io_read`, `read_all`, `io_fread`, `read_buffered_data`, `rb_io_print`, `rb_io_puts`,
  `io_puts_ary`.
- `ext/zlib/zlib.c`: `rb_deflate_s_deflate` with `deflate_run`, `rb_inflate_s_inflate`
  with `inflate_run` (`src` is in the argument array on the stack and used after
  `rb_ensure`; the stream is on the stack), `rb_deflate_deflate`, `zstream_run` (copies the
  input into `z->input` first; `next_in` is held by its `volatile VALUE guard`),
  `zstream_append_input`, `zstream_append_buffer`, `zstream_expand_buffer`,
  `zstream_detach_buffer`, `zstream_shift_buffer`, `zstream_passthrough_input`,
  `zstream_reset`, `zstream_end`, `rb_zstream_finish`, `rb_zstream_flush_next_out`.
- Callees of `marshal.c` that were read for what they do with the pointer: `rb_intern`
  (`parse.y`; reads the name after `ruby_strdup` allocated), `rb_reg_new` (`re.c`; reads
  the source after it allocated the object), `rb_path2class` (`variable.c`; reads the path
  between allocations). Their callers in `marshal.c` are listed above.

After patches 24 and 25 the tests of both crates pass with the local gcc and with clang,
the upstream suites `test/ruby` and `test/zlib` included.

## Follow-ups

- The owner runs the container command of "Reproduction with another compiler": the
  sandbox could not start one, so "Table under stress" is unconfirmed for gcc 13.
- The proposal says that outside the stress test the collection needs the allocation
  counter to pass its limit. For a dump to a string that is not so (see "When the
  collection happens"); the case without stress is a dump to a port. The proposal was left
  as written.
- Outside the bound of the audit, not read: the other callers of `rb_intern`, `rb_reg_new`
  and `rb_path2class` with the buffer of a temporary string, in the files the build
  compiles.
- `zstream_run` relies on a `volatile VALUE` for `next_in` on its error path, and
  `str_buf_cat` would read a stale pointer if a caller appended a shared string's own
  buffer to it. No caller that was read does; neither is patched.
