# Keep dumped strings alive while Marshal writes them

## Why

The first run of the gate on a GitHub runner (Ubuntu 24.04, gcc 13.3) failed in
`every_snippet_gives_the_same_result_under_stress`:

    marshal: table bytes: expected [true, true, 1820, [3, 20, 15, 3, 900]],
    got [true, true, 1820, [980748292, 1650545674, 469919084, 7, 6107184]]

`w_object` in `marshal.c` of Ruby 1.8.7 writes the string a `_dump` method returned with
`w_bytes(RSTRING(v)->ptr, RSTRING(v)->len, arg)`. After the pointer and the length are read
nothing refers to `v`, so a compiler may drop it; `w_bytes` grows the output buffer, which
can run a collection, the string is freed, and the bytes copied afterwards are whatever the
allocator left there. With the gcc of the development machine `v` happens to stay in a
register or on the stack; with gcc 13.3 it does not. Outside the stress test the collection
needs the allocation counter to pass its limit at that moment, so the fault is rare, but it
writes wrong bytes into a saved game without any error: `Table`, `Color`, `Tone`, `Rect` and
`Font` are all dumped this way.

Confirmed in a container of Ubuntu 24.04: the test fails as on the runner, and passes with
`RB_GC_GUARD(v)` after that call. Later Ruby versions carry the same guard.

## What Changes

- A local patch to `marshal.c` keeps every string alive whose buffer is handed to a function
  that can collect.
- The same pattern is looked for in the other vendored C files that the build compiles, and
  what is found is patched or listed.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `embedded-ruby`: values whose buffer is in use survive a collection in `Marshal.dump`.

## Impact

`third_party/ruby-1.8.7/marshal.c` and possibly other files there, `patches/`, `PATCHES.md`;
tests of `vkxp-ruby-sys` or `vkxp-binding`.
