# Design

## Context

See `proposal.md` for motivation. What this change starts from:

- `vkxp-ruby-sys` provides the embedded Ruby 1.8.7: an `Interpreter` that owns the interpreter
  thread and runs closures on it (`Interpreter::run`, which hands the closure a `Ruby`),
  hand-written FFI for the C API (`ffi`), `Ruby::eval_protected`, rooting (`Root`), and
  `Ruby::define_class`, `define_method` and their siblings. A host function has the shape
  `Fn(&Ruby, VALUE, &[VALUE]) -> HostResult`; a panic or a returned `Raise` becomes a Ruby
  exception after every Rust value of the call is dropped. Today every host function is
  registered as taking any number of arguments, so `Method#arity` is -1 for all of them, and
  the body counts with `Ruby::check_arity`. Only one interpreter can be started per process.
  Ruby raises with `longjmp`; the crate documents the two rules for entering and leaving Ruby.
- `vkxp-core` provides `Color`, `Tone`, `Rect`, `Table`, `Font`, `FontDefaults`, the `Bitmaps`
  arena addressed by protocol `BitmapId`s, `draw_text` and `text_size` on top of the
  `TextRasterizer` trait, and typed errors (`BitmapError::Disposed`, `CreateFailed`).
- `vkxp-vfs` provides `GameFiles::resolve` and `read` by kind (image, audio, data) and a
  not-found error carrying the requested path. `vkxp-image` provides `decode(bytes)`.
- `vkxp_core::test_support::CellRasterizer` is a deterministic `TextRasterizer` for tests. The
  font engine (`vkxp-text`) is being built in parallel and is not used by this change.
- The original runtime's class, method and arity list was read from its DLL (286 entries for 15
  classes and modules) and is supplied to this change as a text file, one entry per line:
  `Class#method arity` or `Class.method arity`. 24 entries take any number of arguments (-1);
  the others take a fixed number from 0 to 4.
- The interpreter is conservative about the stack but cannot see Rust heap memory, and a
  garbage collection can start inside any call that allocates a Ruby object.

## Goals / Non-Goals

**Goals:**

- A foundation that later binding changes extend by adding modules, without reworking it.
- The five value classes and `Bitmap` complete enough for real game scripts and for `Marshal`
  to load original data files.
- Tests written in Ruby, evaluated in the embedded interpreter.

**Non-Goals:**

- Anything that needs the host link or the scene: `Graphics`, `Input`, `Audio`, sprites and the
  rest.
- Live binding of a `Color`, `Tone` or `Rect` to a sprite property. The wrapper type is designed
  for it; the scene classes use it later.

## Decisions

### Guest state

`Guest` holds what the classes operate on: the `Bitmaps` arena, `FontDefaults`, `GameFiles`,
the text backend and, later, the scene and the link. One `Guest` exists per interpreter and lives
in a thread-local on the interpreter thread. Host functions reach it through a scoped accessor
that panics on re-entrant mutable access; the method wrappers keep each access short and never
call into Ruby while holding it, because a Ruby call can run arbitrary script code that calls
back in.

`Guest::install(ruby, config)` defines the classes. `config` carries the game files (game
directory and RTP directories), the default font names and the text backend.

### Fixed-arity methods

`vkxp-ruby-sys` gains an `Arity` (`Any`, or `Fixed(n)` for the counts Ruby 1.8 accepts, 0 to
15) and variants of its `define_*` functions that take one. A fixed-arity method is registered
with a C function of that many arguments, generated per count by a macro, that collects them
into a slice and runs the same body through the same panic and raise handling as today. Ruby
then checks the count before the body runs, raises its own `ArgumentError`
(`wrong number of arguments (1 for 2)`) and reports the arity scripts can ask for. The existing
any-arity functions stay as they are. The binding registers each method with the arity of the
method list.

### Wrapping Rust in Ruby objects

Each class is a Ruby data class with an allocator. The object's data pointer is a boxed Rust
struct:

- `Color`, `Tone`, `Rect`: the core value plus an optional owner (an enum naming a scene
  element and a property). Owner is always absent in this change.
- `Table`: the core table. The wrapper is written so the table can later be shared with a
  tilemap.
- `Font`: the core font attributes; its `color` is a `Color` object kept in an instance
  variable so that `font.color.red = 0` works on the font's own colour. The same pattern serves
  `Bitmap#font`.
- `Bitmap`: the `BitmapId`.

The free function Ruby calls when an object is collected must not touch the `Guest` directly,
because collection can start while a method holds it. It pushes the handle onto a small queue
that is drained the next time the guest is entered, which disposes arena bitmaps of collected
`Bitmap` objects.

Objects referenced from Rust-side state are rooted by the Ruby object that owns them (instance
variables), never by raw values in Rust heap memory.

### Arguments and errors

A small set of helpers converts arguments: integers (`Fixnum`, `Bignum` in range, `Float`
truncated, as Ruby's own `NUM2INT`), floats, strings, and typed objects (`TypeError` naming the
expected class). Variable-arity methods count arguments themselves and raise `ArgumentError`
with Ruby's usual message.

Rust errors map to Ruby exceptions in one place: `BitmapError::Disposed` to
`RGSSError("disposed bitmap")`, creation failures and undecodable images to
`RGSSError("failed to create bitmap")`, a missing file to
`Errno::ENOENT` (message `No such file or directory - <path as given>`, which the default
`Main` script parses), malformed serialized data to `ArgumentError`. `RGSSError` is defined
here as a subclass of `StandardError`.

Every host function follows the interpreter crate's rule: compute a `Result` with all Rust
values dropped, then raise.

### Marshal

`_dump(depth)` returns a Ruby string of `to_bytes()`; `_load(string)` builds the object from
`from_bytes`. Ruby strings are byte strings in 1.8, so no encoding is involved.

### Bitmap from a file

`Bitmap.new(path)` with a single String argument: `GameFiles::read(Image, path)`, then
`vkxp_image::decode`, then `Bitmaps::create_from_rgba`. The path is passed as the script gave
it; resolution rules are those of `game-files`.

### Text backend

`TextBackend` is a trait of this crate: core's `TextRasterizer` plus
`font_exists(&self, name: &str) -> bool`. The guest holds one as a boxed trait object, given in
the configuration. `draw_text` and `text_size` pass it to the bitmap arena; `Font.exist?` asks
it. Tests use an implementation over core's `CellRasterizer` that knows a fixed list of family
names. The implementation over `vkxp-text` comes with the change that runs games, so this
change does not depend on that crate. Strings are passed as UTF-8; bytes that are not valid
UTF-8 are replaced, not rejected.

### Method list test

The supplied list is committed as `crates/vkxp-binding/tests/data/rgss1_api.txt`. A test
iterates the entries of the classes implemented so far and compares `arity`. Later binding
changes extend the set of classes the test covers until it is the whole file.

### Test harness

`vkxp_binding::testing` (behind a feature used by the crate's own tests and later by the
executable's tests) starts the interpreter once per test process, installs a `Guest` over a
temporary game directory with generated image files and the cell-based text backend, and
offers `eval(ruby) -> Result<String, RubyError>` returning `inspect` of the value. Tests are
Ruby snippets with expected `inspect` strings or raised classes. Because there is one
interpreter per process, tests in one test binary run on it one after another.

## Risks / Trade-offs

- [A collection during a method corrupts state] → The free queue, short guest borrows, and
  rooting through instance variables; a stress test forces collection after every allocation
  while running the whole Ruby test set.
- [Re-entrancy: a script callback during a host function] → No Ruby call is made while the
  guest is borrowed; `to_s` conversions happen before borrowing.
- [The interpreter crate's helper names differ from what this document assumes] → This document
  names responsibilities, not signatures; the crate documentation of `vkxp-ruby-sys` is the
  reference.
- [Global state survives between tests in one process] → Tests use fresh objects and reset font
  defaults they change.

## Follow-ups

Questions that came up during implementation and were not built or settled here:

- The error mapping names no exception for two failures. A table size that `vkxp-core`
  refuses (`TableError`) raises `ArgumentError`, and a file that is found but cannot be read
  (`VfsError::Io`) raises `IOError`. What the original runtime raises in either case is not
  checked.
- Comparing a `Color`, `Tone` or `Rect` with an object of another class (`color == nil`)
  returns `false`. Whether the original runtime returns `false` or raises `TypeError` is not
  checked.
- `Table#[]` and `Table#[]=` with a number of indices other than the table's dimensions
  raise `ArgumentError`. The requirement only defines the matching count; the original's
  behaviour for the others is not checked.
- Integer arguments follow `NUM2INT`, so a number outside 32 bits raises `RangeError`. The
  original runtime may truncate or clamp instead.
- `Font#color=` and `Font.default_color=` copy the value into the `Color` object the font
  already has; `Bitmap#font=` keeps the given `Font` object. Whether the original keeps or
  copies in each case is not checked.
- The classes are kept alive by their constants only. A script that removes the constant
  `Color` and forces a collection would leave the binding with a dead class. Registering the
  classes as collector roots would close that; `vkxp-ruby-sys` has `Root` for it.
- A path or text that is not valid UTF-8 has its invalid bytes replaced before it reaches
  `vkxp-vfs` or the text backend, so a file whose name is in a legacy encoding cannot be
  named by a script yet. This belongs to the encoding work of the change that runs games.
- The arity test covers the six classes of this change; the other nine of the method list
  are added by the changes that implement them.
