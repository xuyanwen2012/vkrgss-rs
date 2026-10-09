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
//!     run: vkxp_binding::RunConfig::default(),
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
//! # Running a game
//!
//! [`run_game`] is the one call that plays a game, made on the interpreter thread with a
//! [`Config`] and the guest end of a link (`vkxp_proto::link`). The caller has read
//! `Game.ini` and resolved the run-time packages, whose errors it reports before a window
//! exists, and has made the game directory the working directory, because scripts write
//! their saves through Ruby's own file API. The [`RunConfig`] inside the `Config` carries
//! the configuration file, the path of the built-in scripts file, the debug and battle-test
//! flags and the switch for the process functions.
//!
//! ```no_run
//! use vkxp_binding::{Config, RunConfig, run_game};
//! use vkxp_ruby_sys::{Interpreter, Options};
//! # fn text_backend() -> Box<dyn vkxp_binding::TextBackend + Send> { unimplemented!() }
//!
//! let game = std::path::Path::new("path/to/game");
//! let config = Config {
//!     files: vkxp_vfs::GameFiles::new(game, []),
//!     font_names: vec!["Arial".to_owned()],
//!     text: text_backend(),
//!     run: RunConfig {
//!         ini: vkxp_vfs::GameIni::read(game, None)?,
//!         builtin_scripts: "path/to/rpg.rb".into(),
//!         ..RunConfig::default()
//!     },
//! };
//! let (guest_end, host_end) = vkxp_proto::link::link(|| { /* wake the host */ });
//! let interpreter = Interpreter::start(Options::default())?;
//! // The host serves `host_end` meanwhile: a tick for every frame, input, reset and quit.
//! # drop(host_end);
//! interpreter.run(move |ruby| run_game(ruby, config, guest_end))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! In order, `run_game` installs the guest and sends the window title; evaluates the
//! prelude; evaluates the built-in scripts file under the name `(builtin)`; reads the script
//! archive into `$RGSS_SCRIPTS`, where each entry gets its inflated source as fourth
//! element; and evaluates the scripts as `Section000`, `Section001` and so on. Then:
//!
//! | A script ends with | What happens |
//! |---|---|
//! | its last statement | the next script runs; after the last one the game is over |
//! | `SystemExit` | the game is over |
//! | `Reset` | audio stops, `Graphics._reset` disposes every scene element and bitmap, the input state is cleared, and the scripts run again from the first in the same interpreter |
//! | any other exception | the report `Script '<name>' line <n>: <class> occurred.` with the message goes to the error stream and to the host as a modal message; once that is dismissed the game is over, and the run returns [`RunError::Script`] |
//!
//! The script of a report is found through the `Section` index of the backtrace and named
//! as the archive names it. However the game ends, and also when the built-in scripts file
//! or the archive cannot be read, the audio channels are stopped, the host gets its shutdown
//! message and the link is dropped.
//!
//! The built-in scripts are Enterbrain's and are not part of this repository: the file is
//! extracted from the user's RGSS DLL. `Graphics._reset` finds what to dispose through the
//! Ruby objects of the RGSS classes, with each class's own `dispose`.
//!
//! # The prelude
//!
//! What a game meets outside the RGSS classes is prepared by a prelude written in Ruby
//! (`src/prelude.rb`), on top of a few host functions whose names start with `vkxp_`:
//!
//! - `$KCODE` is `"UTF8"`, the load path is empty, `$DEBUG` and `$BTEST` follow the
//!   [`RunConfig`].
//! - `Reset` and `Hangup` are subclasses of `Exception`, so a bare `rescue` lets them pass.
//! - `save_data` writes a `Marshal` dump through Ruby's `File`. `load_data` is a host
//!   function that finds the file through the game's files, without regard to letter case.
//! - `print` and `p` write their text to `$stderr` and show it as a modal message, and return
//!   when the host has dismissed it.
//! - `Win32API` is a placeholder: `call` returns 0 and names each function once on `$stderr`.
//! - `require` and `load` evaluate only `.rb` files inside the game directory and raise
//!   `LoadError` for anything else.
//! - Unless the `RunConfig` enables them, `system`, backticks, `exec`, `fork`, `syscall` and
//!   `IO.popen` start nothing, return `nil` and say so on `$stderr`
//!   (`src/prelude_process_off.rb`).
//!
//! # The pump
//!
//! The modules `Graphics`, `Input` and `Audio` and the functions above talk to the host
//! through the guest: `Guest::send` for what goes out, and one place that consumes what
//! comes in, `Guest::pump`. It is called by `Graphics.update` and by every frame of
//! `Graphics.transition` (waiting for the tick of the frame just sent), by `Input.update`
//! (taking only what has arrived) and by `print`, `p` and the error report (waiting for the
//! dismissal). Whatever it waits for, it keeps the latest input snapshot for the next
//! `Input.update` and records a reset or quit request. The request is raised, as `Reset` or
//! `SystemExit`, by `Graphics.update` or `Graphics.transition` after the pump has returned
//! and the guest is released, which is the raise rule of the interpreter crate. A host that
//! has gone counts as a quit request, so a guest never waits for a host that is not there.
//!
//! Music and background sound remember the description they were started with (file name,
//! volume, pitch): `Audio.bgm_play` and `Audio.bgs_play` with the same description send
//! nothing, and a stop or a fade forgets it.
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
//! Its `host` module is a scripted host on the other end of a link: it ticks every frame,
//! dismisses every modal message, records what the guest sent, and plays whole games made
//! for a test.

mod audio;
mod bitmap;
mod color;
mod error;
mod font;
mod graphics;
mod guest;
mod input;
mod kernel;
mod rb;
mod rect;
mod runtime;
mod table;
mod text;
mod wrap;

#[cfg(test)]
mod foundation_tests;

#[cfg(feature = "testing")]
pub mod testing;

#[cfg(feature = "testing")]
pub mod scene_testing;

pub use error::Error;
pub use guest::{Config, Guest};
pub use runtime::{RunConfig, RunError, run_game};
pub use text::TextBackend;
