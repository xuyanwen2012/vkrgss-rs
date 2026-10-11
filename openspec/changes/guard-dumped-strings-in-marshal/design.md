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

To be filled in.

## Follow-ups

None yet.
