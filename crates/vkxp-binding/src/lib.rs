//! RGSS Ruby API on top of core.
//!
//! Side: guest.
//!
//! This crate defines the classes a game's scripts see and connects them to the Rust crates
//! that do the work: the interpreter of `vkxp-ruby-sys`, the values and bitmaps of
//! `vkxp-core`, the files of `vkxp-vfs` and the decoders of `vkxp-image`. Today those classes
//! are `Color`, `Tone`, `Rect`, `Table`, `Font` and `Bitmap`, and the exception `RGSSError`.
//! Every method the original runtime defines for them exists with the original arity;
//! `tests/data/rgss1_api.txt` is the list, and `tests/arity.rs` checks it.
//!
//! ```no_run
//! use vkxp_binding::{Config, Guest};
//! use vkxp_ruby_sys::{Interpreter, Options};
//! # fn text_backend() -> Box<dyn vkxp_binding::TextBackend + Send> { unimplemented!() }
//!
//! let config = Config {
//!     files: vkxp_vfs::GameFiles::new("path/to/game", []),
//!     font_names: vec!["Arial".to_owned()],
//!     text: text_backend(),
//! };
//! let interpreter = Interpreter::start(Options::default())?;
//! interpreter.run(move |ruby| {
//!     Guest::install(ruby, config).expect("the classes are defined");
//!     ruby.eval_protected("Bitmap.new(32, 32).rect.to_s", "Main", 1).map(|_| ())
//! })?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Guest state
//!
//! A [`Guest`] holds what the classes operate on: the bitmap arena, the font defaults, the
//! game's files and the [`TextBackend`], and already the scene and the input state that the
//! scene classes and the modules `Graphics` and `Input` will use. [`Guest::install`] defines the classes and puts one
//! guest, made from a [`Config`], into a thread-local of the interpreter thread; there is one
//! per interpreter. Host functions reach it through [`Guest::with`], which panics on
//! re-entrant access. That is why an access is short and never calls into Ruby: a Ruby call
//! can run script code that calls back into a method that needs the guest. A method
//! converts its arguments first (a `to_s` runs there), then enters the guest for the
//! operation, then builds its result.
//!
//! # Entering and leaving Ruby
//!
//! The rules of `vkxp-ruby-sys` hold for every host function here. A method body returns a
//! `HostResult` and never raises on its own: it computes a `Result`, and the trampoline of
//! the interpreter crate raises after every Rust value of the call is dropped. Calls into
//! the C API that can raise (conversions, allocations, instance variables) go through the
//! private module `rb`, which runs each under `rb_protect` and returns a `Result`.
//!
//! # Wrapping
//!
//! Each class is a Ruby data class with an allocator, so every instance, also one made by
//! `clone`, `allocate` or `Marshal`, carries a boxed Rust struct (private module `wrap`):
//!
//! | Class | Data | Ruby objects it owns |
//! |---|---|---|
//! | `Color`, `Tone`, `Rect` | the core value, and an optional owner | |
//! | `Table` | the core table | |
//! | `Font` | the core font attributes | its `Color`, in an instance variable |
//! | `Bitmap` | the `BitmapId` of its bitmap in the arena | its `Font`, in an instance variable |
//!
//! The owner of a `Color`, `Tone` or `Rect` is what will make `sprite.color.red = 0` reach
//! the sprite; no object has one until the scene classes exist.
//!
//! A Ruby object that belongs to another is kept in an instance variable of its owner whose
//! name has no `@`, which scripts cannot see. That is what keeps it alive: the collector
//! scans the interpreter thread's stack but not Rust heap memory, so no Ruby value is ever
//! stored in a Rust struct. The classes themselves are kept alive by their constants.
//!
//! Because a font's colour and a bitmap's font are objects a script can change at any time,
//! their values are read when they are needed: the attributes of a bitmap's `Font` object
//! are copied into the arena before text is drawn or measured.
//!
//! # The free queue
//!
//! When the collector frees a `Bitmap` object, the bitmap in the arena must go too. But a
//! collection can start inside any call that allocates a Ruby object, so the free function
//! of a data object must not touch the guest. It pushes the `BitmapId` onto a queue, and
//! [`Guest::with`] disposes the queued bitmaps each time the guest is entered.
//!
//! # Errors
//!
//! Operations fail with an [`Error`], and one function (`error::to_raise`) turns it into
//! the exception of the original runtime:
//!
//! | Failure | Exception |
//! |---|---|
//! | a disposed bitmap | `RGSSError`, `disposed bitmap` |
//! | a bitmap that cannot be created, an image that cannot be decoded | `RGSSError`, `failed to create bitmap` |
//! | a file that is not found | `Errno::ENOENT`, `No such file or directory - <path as given>` |
//! | a file that cannot be read | `IOError` |
//! | malformed serialized data, an impossible table size | `ArgumentError` |
//!
//! Arguments of the wrong class raise `TypeError`, a wrong argument count `ArgumentError`
//! with Ruby's own message. Methods with a fixed arity are registered with it, so Ruby
//! checks the count; the others count for themselves.
//!
//! # Text
//!
//! Glyphs come from the guest's [`TextBackend`]: core's `TextRasterizer` plus the question
//! `Font.exist?` asks. Strings reach it as UTF-8, with invalid bytes replaced. The backend
//! over real fonts comes with the change that runs games; tests use the `CellBackend` of
//! the `testing` module.
//!
//! # Adding a class
//!
//! A later change adds a class in a module of its own:
//!
//! 1. A data struct that implements `wrap::Wrapped`: the class name, where its class is
//!    among `guest::Classes`, the data of a freshly allocated object, and what happens when
//!    an object is collected (through the free queue if the guest is involved).
//! 2. Host functions of the shape `fn(&Ruby, VALUE, &[VALUE]) -> HostResult`, built from the
//!    helpers of `rb` and `wrap` in the order described under *Guest state*.
//! 3. A `define` function that calls `wrap::define_class` and registers each method with
//!    the arity of the method list (`rb::method`, `rb::class_method`).
//! 4. A field in `guest::Classes`, filled by [`Guest::install`], and whatever state the
//!    class needs in [`Guest`].
//! 5. The class name in the list of `tests/arity.rs`, and its scenarios as Ruby snippets in
//!    `tests/cases`, which `tests/ruby.rs` evaluates and `tests/stress.rs` evaluates again
//!    with a collection at every allocation.
//!
//! # Testing
//!
//! The `testing` module, behind the feature of the same name, starts the one interpreter of
//! a test process with a guest over a generated game directory and evaluates Ruby source.

mod bitmap;
mod color;
mod error;
mod font;
mod guest;
mod rb;
mod rect;
mod table;
mod text;
mod wrap;

#[cfg(test)]
mod foundation_tests;

#[cfg(feature = "testing")]
pub mod testing;

pub use error::Error;
pub use guest::{Config, Guest};
pub use text::TextBackend;
