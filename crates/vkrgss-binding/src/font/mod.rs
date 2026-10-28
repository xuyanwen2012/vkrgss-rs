//! The Ruby class `Font`.
//!
//! A `Font` object holds the core font attributes. Its colour is a `Color` object of its
//! own, kept in an instance variable of the font so that `font.color.red = 0` changes the
//! font's colour; the colour inside the core attributes is filled in from that object when
//! the font is used. The class-level defaults are the guest's [`FontDefaults`], with the
//! default colour as a `Color` object kept by the class in the same way.
//!
//! In a game of RGSS2 the defaults are those of that runtime and a font has a shadow:
//! `Font#shadow` and `Font.default_shadow` are defined there only.
//!
//! In a game of RGSS3 a font also has an outline and an out colour, a second `Color` object
//! of its own, and keeps the objects a script gave for its name and size beside the core
//! attributes, which hold what is drawn with: the name chosen from the object
//! ([`drawn_name`]) and the integer part of the size. The class keeps the objects given to
//! the `default_` writers the same way, the two default colours among them, which are kept
//! and not copied; the guest's [`FontDefaults`] hold the values derived from them.
//!
//! [`FontDefaults`]: vkrgss_core::FontDefaults
//! [`drawn_name`]: rgss3::drawn_name

use std::ffi::CStr;

use vkrgss_core::{Color, Font, FontDefaults};
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkrgss_vfs::Runtime;

use crate::color::{color_arg, color_new, color_set};
use crate::guest::{self, Classes, Guest};
use crate::rb;
use crate::wrap::{self, Handle, Owned, Wrapped};

mod rgss3;

#[cfg(test)]
mod tests;

pub(crate) use rgss3::font_assign_rgss3;
use rgss3::{define_rgss3, keep_default_objects, keep_objects_of};

/// The instance variable of a font that holds its `Color` object. Without `@`, so scripts
/// do not see it.
const COLOR: &CStr = c"color";

/// The instance variable of the class that holds the `Color` object of
/// `Font.default_color`.
const DEFAULT_COLOR: &CStr = c"default_color";

/// The instance variables of a font of RGSS3 that hold its out colour, a `Color` object of
/// its own, and the objects given for its name and its size.
const OUT_COLOR: &CStr = c"out_color";
const NAME: &CStr = c"name";
const SIZE: &CStr = c"size";

/// The instance variables of the class in RGSS3 that hold the objects of the readers
/// `Font.default_out_color`, `default_name`, `default_size` and the four flags.
const DEFAULT_OUT_COLOR: &CStr = c"default_out_color";
const DEFAULT_NAME: &CStr = c"default_name";
const DEFAULT_SIZE: &CStr = c"default_size";
const DEFAULT_BOLD: &CStr = c"default_bold";
const DEFAULT_ITALIC: &CStr = c"default_italic";
const DEFAULT_SHADOW: &CStr = c"default_shadow";
const DEFAULT_OUTLINE: &CStr = c"default_outline";

/// The data of a `Font` object.
#[derive(Debug, Clone)]
pub(crate) struct FontData {
    /// The attributes. `color` is not kept up to date here: see [`font_arg`].
    font: Font,
    /// Whether `name` is an Array (or a single String).
    names_listed: bool,
}

impl Wrapped for FontData {
    const CLASS_NAME: &'static str = "Font";

    fn class(classes: &Classes) -> VALUE {
        classes.font
    }

    fn blank() -> Self {
        FontData {
            font: Font {
                names: Vec::new(),
                size: FontDefaults::INITIAL_SIZE,
                bold: false,
                italic: false,
                color: Color::rgb(255.0, 255.0, 255.0),
                shadow: false,
                outline: false,
                out_color: Color::new(0.0, 0.0, 0.0, 128.0),
            },
            names_listed: true,
        }
    }
}

fn this(ruby: &Ruby, recv: VALUE) -> Result<Handle<FontData>, Raise> {
    wrap::expect(ruby, recv)
}

/// Family names given as a String or as an Array of Strings, and whether it was an Array.
/// Anything else raises `TypeError`.
fn names_arg(ruby: &Ruby, value: VALUE) -> Result<(Vec<String>, bool), Raise> {
    match rb::array_elements(ruby, value) {
        // `value` stays in the caller's frame while the copies of its elements are read.
        Some(elements) => {
            let names: Result<Vec<String>, Raise> = elements
                .iter()
                .map(|&element| rb::text(ruby, element))
                .collect();
            Ok((names?, true))
        }
        None => Ok((vec![rb::text(ruby, value)?], false)),
    }
}

/// Family names as a script reads them: an Array of Strings, or the one String.
fn names_value(ruby: &Ruby, names: &[String], listed: bool) -> HostResult {
    match names {
        [name] if !listed => Ok(rb::bytes_new(ruby, name.as_bytes())?),
        _ => rb::strings_new(ruby, names),
    }
}

/// The value of the `Color` object in instance variable `name` of `object`, if it is set.
fn color_at(ruby: &Ruby, object: VALUE, name: &CStr) -> Result<Option<Color>, Raise> {
    let color = rb::ivar_get(ruby, object, name)?;
    if ffi::nil_p(color) {
        return Ok(None);
    }
    Ok(Some(color_arg(ruby, color)?))
}

/// Gives `object` a new `Color` object of this value in instance variable `name`.
fn give_color(ruby: &Ruby, object: VALUE, name: &CStr, color: Color) -> Result<(), Raise> {
    let color = color_new(ruby, color)?;
    Ok(rb::ivar_set(ruby, object, name, color)?)
}

/// `color=` for the `Color` object in instance variable `name`: the value is copied into
/// the object the font (or the class) already has.
fn assign_color(ruby: &Ruby, object: VALUE, name: &CStr, value: VALUE) -> Result<(), Raise> {
    let assigned = color_arg(ruby, value)?;
    let own = rb::ivar_get(ruby, object, name)?;
    if ffi::nil_p(own) {
        give_color(ruby, object, name, assigned)
    } else {
        color_set(ruby, own, assigned)
    }
}

/// The current defaults: the guest's, with the colour of the `Font.default_color` object
/// and, in RGSS3, that of the `Font.default_out_color` object.
fn defaults(ruby: &Ruby) -> Result<(FontDefaults, bool), Raise> {
    let class = guest::classes(ruby).font;
    let color = color_at(ruby, class, DEFAULT_COLOR)?;
    // Asked in RGSS3 only: the name of the variable is interned at its first use, which
    // would move the sequence of symbols of the other runtimes.
    let out_color = if is_rgss3(ruby) {
        color_at(ruby, class, DEFAULT_OUT_COLOR)?
    } else {
        None
    };
    let (mut defaults, listed) = Guest::with(ruby, |guest| {
        (guest.font_defaults.clone(), guest.font_names_listed)
    });
    if let Some(color) = color {
        defaults.color = color;
    }
    if let Some(out_color) = out_color {
        defaults.out_color = out_color;
    }
    Ok((defaults, listed))
}

/// The attributes of a `Font` argument, colour included, and in RGSS3 the out colour too.
/// Anything else raises `TypeError`.
pub(crate) fn font_arg(ruby: &Ruby, value: VALUE) -> Result<Font, Raise> {
    let mut font = this(ruby, value)?.get().font;
    if let Some(color) = color_at(ruby, value, COLOR)? {
        font.color = color;
    }
    // In RGSS3 only, as in `defaults`.
    if is_rgss3(ruby)
        && let Some(out_color) = color_at(ruby, value, OUT_COLOR)?
    {
        font.out_color = out_color;
    }
    Ok(font)
}

/// Whether the game is one of RGSS3.
fn is_rgss3(ruby: &Ruby) -> bool {
    Guest::with(ruby, |guest| guest.runtime.run.runtime == Runtime::Rgss3)
}

/// The font defaults a game of `runtime` starts with. RGSS1 takes its family names from
/// the configuration, by the language of the game; RGSS2 has names of its own.
pub(crate) fn initial_defaults(runtime: Runtime, names: Vec<String>) -> FontDefaults {
    match runtime {
        Runtime::Rgss1 => FontDefaults::new(names),
        Runtime::Rgss2 => FontDefaults::rgss2(),
        Runtime::Rgss3 => FontDefaults::rgss3(),
    }
}

/// `Bitmap#font=` of RGSS2: copies the name, the size, bold, italic and the colour of the
/// font `given` into the font `own`, which keeps its shadow, as measured on the original.
/// Anything but a `Font` raises `TypeError`.
pub(crate) fn font_assign(ruby: &Ruby, own: VALUE, given: VALUE) -> Result<(), Raise> {
    let data = this(ruby, given)?.get();
    let color = font_arg(ruby, given)?.color;
    this(ruby, own)?.with(|own| {
        let shadow = own.font.shadow;
        *own = data;
        own.font.shadow = shadow;
    });
    let assigned = color_new(ruby, color)?;
    assign_color(ruby, own, COLOR, assigned)
}

/// A new `Font` object with the current defaults. In RGSS3 it has an out colour of its own
/// and reads the name and size objects of the defaults.
pub(crate) fn font_new_default(ruby: &Ruby) -> HostResult {
    let (defaults, names_listed) = defaults(ruby)?;
    let font = Font::new(&defaults);
    let (color, out_color) = (font.color, font.out_color);
    let object = wrap::new_object(ruby, FontData { font, names_listed })?;
    give_color(ruby, object, COLOR, color)?;
    if is_rgss3(ruby) {
        give_color(ruby, object, OUT_COLOR, out_color)?;
        keep_default_objects(ruby, object)?;
    }
    Ok(object)
}

/// A new `Font` object with the attributes of another, and a `Color` object of its own; in
/// RGSS3 also an out colour of its own and the original's name and size objects.
pub(crate) fn font_copy(ruby: &Ruby, original: VALUE) -> HostResult {
    let data = this(ruby, original)?.get();
    let font = font_arg(ruby, original)?;
    let object = wrap::new_object(ruby, data)?;
    give_color(ruby, object, COLOR, font.color)?;
    if is_rgss3(ruby) {
        give_color(ruby, object, OUT_COLOR, font.out_color)?;
        keep_objects_of(ruby, object, original)?;
    }
    Ok(object)
}

/// `initialize(name = Font.default_name, size = Font.default_size)`.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    wrap::intern_owner_names(ruby)?;
    rb::check_count(ruby, args, 0, 2)?;
    let names = args.first().map(|&arg| names_arg(ruby, arg)).transpose()?;
    let size = args.get(1).map(|&arg| rb::int(ruby, arg)).transpose()?;
    let (defaults, default_listed) = defaults(ruby)?;
    let (names, names_listed) = match names {
        Some((names, listed)) => (Some(names), listed),
        None => (None, default_listed),
    };
    let font = Font::with(&defaults, names, size);
    let color = font.color;
    this(ruby, recv)?.with(|data| *data = FontData { font, names_listed });
    give_color(ruby, recv, COLOR, color)?;
    Ok(recv)
}

/// The copy takes the attributes of the original and gets a `Color` object of its own; the
/// instance variable Ruby copied still names the original's.
fn initialize_copy(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let data = this(ruby, args[0])?.get();
    let color = font_arg(ruby, args[0])?.color;
    this(ruby, recv)?.with(|copy| *copy = data);
    give_color(ruby, recv, COLOR, color)?;
    Ok(recv)
}

fn name(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let data = this(ruby, recv)?.get();
    names_value(ruby, &data.font.names, data.names_listed)
}

fn set_name(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let (names, listed) = names_arg(ruby, args[0])?;
    this(ruby, recv)?.with(|data| {
        data.font.names = names;
        data.names_listed = listed;
    });
    Ok(args[0])
}

fn size(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(this(ruby, recv)?.with(|data| data.font.size)))
}

/// `size=`: a number whose whole part is from 6 to 96, as the original accepts; the fraction
/// is dropped. Anything else raises `ArgumentError`.
fn set_size(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let size = rb::float(ruby, args[0])?;
    if !(6.0..97.0).contains(&size) {
        return Err(Raise::new(rb::arg_error(ruby), "bad value for size"));
    }
    let size = size as i32;
    this(ruby, recv)?.with(|data| data.font.size = size);
    Ok(args[0])
}

fn bold(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::boolean(this(ruby, recv)?.with(|data| data.font.bold)))
}

fn set_bold(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let bold = rb::truthy(args[0]);
    this(ruby, recv)?.with(|data| data.font.bold = bold);
    Ok(args[0])
}

fn italic(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::boolean(this(ruby, recv)?.with(|data| data.font.italic)))
}

fn set_italic(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let italic = rb::truthy(args[0]);
    this(ruby, recv)?.with(|data| data.font.italic = italic);
    Ok(args[0])
}

/// `shadow` of RGSS2: true or false.
fn shadow(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::boolean(this(ruby, recv)?.with(|data| data.font.shadow)))
}

/// `shadow=` of RGSS2 takes any object: `false` and `nil` turn the shadow off, everything
/// else, 0 included, turns it on.
fn set_shadow(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let shadow = rb::truthy(args[0]);
    this(ruby, recv)?.with(|data| data.font.shadow = shadow);
    Ok(args[0])
}

/// The font's own `Color` object.
fn color(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    Ok(rb::ivar_get(ruby, recv, COLOR)?)
}

fn set_color(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    assign_color(ruby, recv, COLOR, args[0])?;
    Ok(args[0])
}

fn default_name(ruby: &Ruby, _class: VALUE, _args: &[VALUE]) -> HostResult {
    let (names, listed) = Guest::with(ruby, |guest| {
        (guest.font_defaults.names.clone(), guest.font_names_listed)
    });
    names_value(ruby, &names, listed)
}

fn set_default_name(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let (names, listed) = names_arg(ruby, args[0])?;
    Guest::with(ruby, |guest| {
        guest.font_defaults.names = names;
        guest.font_names_listed = listed;
    });
    Ok(args[0])
}

fn default_size(ruby: &Ruby, _class: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(Guest::with(ruby, |guest| guest.font_defaults.size)))
}

fn set_default_size(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let size = rb::int(ruby, args[0])?;
    Guest::with(ruby, |guest| guest.font_defaults.size = size);
    Ok(args[0])
}

fn default_bold(ruby: &Ruby, _class: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::boolean(Guest::with(ruby, |guest| {
        guest.font_defaults.bold
    })))
}

fn set_default_bold(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let bold = rb::truthy(args[0]);
    Guest::with(ruby, |guest| guest.font_defaults.bold = bold);
    Ok(args[0])
}

fn default_italic(ruby: &Ruby, _class: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::boolean(Guest::with(ruby, |guest| {
        guest.font_defaults.italic
    })))
}

fn set_default_italic(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let italic = rb::truthy(args[0]);
    Guest::with(ruby, |guest| guest.font_defaults.italic = italic);
    Ok(args[0])
}

fn default_shadow(ruby: &Ruby, _class: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::boolean(Guest::with(ruby, |guest| {
        guest.font_defaults.shadow
    })))
}

fn set_default_shadow(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let shadow = rb::truthy(args[0]);
    Guest::with(ruby, |guest| guest.font_defaults.shadow = shadow);
    Ok(args[0])
}

/// The `Color` object new fonts copy their colour from.
fn default_color(ruby: &Ruby, _class: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::ivar_get(
        ruby,
        guest::classes(ruby).font,
        DEFAULT_COLOR,
    )?)
}

fn set_default_color(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    assign_color(ruby, guest::classes(ruby).font, DEFAULT_COLOR, args[0])?;
    Ok(args[0])
}

/// `Font.exist?(name)`: whether the guest's text backend has the family.
fn exist(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let name = rb::text(ruby, args[0])?;
    Ok(rb::boolean(Guest::with(ruby, |guest| {
        guest.text.font_exists(&name)
    })))
}

/// Gives the class the `Color` object of `Font.default_color`, with the colour the guest's
/// defaults start with; in RGSS3 also that of `Font.default_out_color` and the objects the
/// other readers of the defaults start with. Runs once the classes are installed.
pub(crate) fn install_defaults(ruby: &Ruby) -> Result<(), Raised> {
    let classes = guest::classes(ruby);
    let (defaults, runtime) = Guest::with(ruby, |guest| {
        (guest.font_defaults.clone(), guest.runtime.run.runtime)
    });
    let install_color = |name: &CStr, color: Color| {
        let object = rb::allocate(ruby, classes.color)?;
        if let Some(data) = wrap::find::<Owned<Color>>(ruby, object) {
            data.with(|data| data.set(ruby, color));
        }
        rb::ivar_set(ruby, classes.font, name, object)
    };
    install_color(DEFAULT_COLOR, defaults.color)?;
    if runtime != Runtime::Rgss3 {
        return Ok(());
    }
    install_color(DEFAULT_OUT_COLOR, defaults.out_color)?;
    let name = match defaults.names.first() {
        Some(name) => rb::bytes_new(ruby, name.as_bytes())?,
        None => ffi::Qnil,
    };
    rb::ivar_set(ruby, classes.font, DEFAULT_NAME, name)?;
    rb::ivar_set(ruby, classes.font, DEFAULT_SIZE, rb::fix(defaults.size))?;
    for (name, flag) in [
        (DEFAULT_BOLD, defaults.bold),
        (DEFAULT_ITALIC, defaults.italic),
        (DEFAULT_SHADOW, defaults.shadow),
        (DEFAULT_OUTLINE, defaults.outline),
    ] {
        rb::ivar_set(ruby, classes.font, name, rb::boolean(flag))?;
    }
    Ok(())
}

/// Defines `Font` as a game of `runtime` has it: the shadow exists from RGSS2 on, and RGSS3
/// has a definition of its own ([`define_rgss3`]).
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<FontData>(ruby, c"Font")?;
    if runtime == Runtime::Rgss3 {
        define_rgss3(ruby, class)?;
        return Ok(class);
    }
    rb::method(ruby, class, c"initialize", -1, initialize)?;
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy)?;
    rb::method(ruby, class, c"name", 0, name)?;
    rb::method(ruby, class, c"name=", 1, set_name)?;
    rb::method(ruby, class, c"size", 0, size)?;
    rb::method(ruby, class, c"size=", 1, set_size)?;
    rb::method(ruby, class, c"bold", 0, bold)?;
    rb::method(ruby, class, c"bold=", 1, set_bold)?;
    rb::method(ruby, class, c"italic", 0, italic)?;
    rb::method(ruby, class, c"italic=", 1, set_italic)?;
    rb::method(ruby, class, c"color", 0, color)?;
    rb::method(ruby, class, c"color=", 1, set_color)?;
    rb::class_method(ruby, class, c"default_name", 0, default_name)?;
    rb::class_method(ruby, class, c"default_name=", 1, set_default_name)?;
    rb::class_method(ruby, class, c"default_size", 0, default_size)?;
    rb::class_method(ruby, class, c"default_size=", 1, set_default_size)?;
    rb::class_method(ruby, class, c"default_bold", 0, default_bold)?;
    rb::class_method(ruby, class, c"default_bold=", 1, set_default_bold)?;
    rb::class_method(ruby, class, c"default_italic", 0, default_italic)?;
    rb::class_method(ruby, class, c"default_italic=", 1, set_default_italic)?;
    rb::class_method(ruby, class, c"default_color", 0, default_color)?;
    rb::class_method(ruby, class, c"default_color=", 1, set_default_color)?;
    rb::class_method(ruby, class, c"exist?", 1, exist)?;
    if runtime != Runtime::Rgss1 {
        rb::method(ruby, class, c"shadow", 0, shadow)?;
        rb::method(ruby, class, c"shadow=", 1, set_shadow)?;
        rb::class_method(ruby, class, c"default_shadow", 0, default_shadow)?;
        rb::class_method(ruby, class, c"default_shadow=", 1, set_default_shadow)?;
    }
    Ok(class)
}
