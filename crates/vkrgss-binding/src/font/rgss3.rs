//! `Font` of RGSS3: the objects given for the name and the size, the outline and the out
//! colour, the default objects of the class, and the definition of the class.

use std::ffi::CStr;

use vkrgss_core::{Color, Font, FontDefaults};
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{HostResult, Raise, Raised, Ruby};

use crate::bitmap::{converted_arg, text_of};
use crate::color::{color_arg, color_set};
use crate::guest::{self, Guest};
use crate::rb;
use crate::wrap::{self, Owned};

use super::{
    COLOR, DEFAULT_BOLD, DEFAULT_COLOR, DEFAULT_ITALIC, DEFAULT_NAME, DEFAULT_OUT_COLOR,
    DEFAULT_OUTLINE, DEFAULT_SHADOW, DEFAULT_SIZE, FontData, NAME, OUT_COLOR, SIZE, bold, color,
    default_color, defaults, font_arg, give_color, italic, set_bold, set_italic, set_shadow,
    shadow, this,
};

/// The number of characters of the name a font of RGSS3 is drawn with: the rest is cut, as
/// the face name of a font of Windows holds at most 31.
const NAME_LENGTH: usize = 31;

/// A font name, or an element of a list of them, as `Font.exist?` of RGSS3 asks about it: a
/// String as it is and a Symbol by its name; `None` for an object of any other class, which
/// exists in no case.
fn asked_name(ruby: &Ruby, value: VALUE) -> Result<Option<String>, Raise> {
    if rb::string_bytes(ruby, value).is_some() || ffi::symbol_p(value) {
        return Ok(Some(text_of(ruby, value)?));
    }
    Ok(None)
}

/// Which element of a list of names a font of RGSS3 is drawn with: the first that is a name
/// (see [`asked_name`]) that `exists`, else the first; `None` for an empty list.
pub(super) fn chosen_element(
    names: &[Option<String>],
    exists: impl Fn(&str) -> bool,
) -> Option<usize> {
    if names.is_empty() {
        return None;
    }
    let existing = names
        .iter()
        .position(|name| name.as_deref().is_some_and(&exists));
    Some(existing.unwrap_or(0))
}

/// The name text is drawn with from the `to_s` of the object chosen: its first
/// [`NAME_LENGTH`] characters, and none for an empty text.
pub(super) fn cut_name(text: &str) -> Option<String> {
    let cut: String = text.chars().take(NAME_LENGTH).collect();
    (!cut.is_empty()).then_some(cut)
}

/// The name a font of RGSS3 is drawn with when a script gives it `value`, settled at the
/// assignment: for an Array the element [`chosen_element`] picks, for anything else the
/// object, by its `to_s` and cut by [`cut_name`]. `None` is a font without a name, which
/// draws nothing.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1 and RGSS300 3.0.0.1,
/// `ace-v1-bitmap-REPORT.md` section 7.
pub(super) fn drawn_name(ruby: &Ruby, value: VALUE) -> Result<Option<String>, Raise> {
    let text = match rb::array_elements(ruby, value) {
        // `value` is an argument of the running method, which keeps its elements live.
        Some(elements) => {
            let names = elements
                .iter()
                .map(|&element| asked_name(ruby, element))
                .collect::<Result<Vec<_>, Raise>>()?;
            let chosen = Guest::with(ruby, |guest| {
                chosen_element(&names, |name| guest.text.font_exists_exactly(name))
            });
            match chosen {
                None => return Ok(None),
                Some(index) => match &names[index] {
                    Some(name) => name.clone(),
                    None => text_of(ruby, elements[index])?,
                },
            }
        }
        None => text_of(ruby, value)?,
    };
    Ok(cut_name(&text))
}

/// The core names of a font drawn with `name`: that one, or none.
fn names_of(name: Option<String>) -> Vec<String> {
    name.into_iter().collect()
}

/// A size of RGSS3: an Integer by conversion, a Float truncated, from 6 to 96. Anything
/// else raises `ArgumentError` with `message`.
fn size_arg(ruby: &Ruby, value: VALUE, message: &'static str) -> Result<i32, Raise> {
    let size = rb::int(ruby, value)?;
    if !(6..=96).contains(&size) {
        return Err(Raise::new(rb::arg_error(ruby), message));
    }
    Ok(size)
}

/// Gives `object` new `Color` objects of the two colours of `font`.
fn give_colors(ruby: &Ruby, object: VALUE, font: &Font) -> Result<(), Raise> {
    give_color(ruby, object, COLOR, font.color)?;
    give_color(ruby, object, OUT_COLOR, font.out_color)
}

/// Makes the font `object` read the name and size objects of the defaults.
pub(super) fn keep_default_objects(ruby: &Ruby, object: VALUE) -> Result<(), Raise> {
    let class = guest::classes(ruby).font;
    let name = rb::ivar_get(ruby, class, DEFAULT_NAME)?;
    rb::ivar_set(ruby, object, NAME, name)?;
    let size = rb::ivar_get(ruby, class, DEFAULT_SIZE)?;
    Ok(rb::ivar_set(ruby, object, SIZE, size)?)
}

/// Makes the font `object` read the name and size objects of the font `original`.
pub(super) fn keep_objects_of(ruby: &Ruby, object: VALUE, original: VALUE) -> Result<(), Raise> {
    let name = rb::ivar_get(ruby, original, NAME)?;
    rb::ivar_set(ruby, object, NAME, name)?;
    let size = rb::ivar_get(ruby, original, SIZE)?;
    Ok(rb::ivar_set(ruby, object, SIZE, size)?)
}

/// A `Color` argument of RGSS3. Anything else raises `TypeError` with the message of the
/// original.
fn color_arg_rgss3(ruby: &Ruby, value: VALUE) -> Result<Color, Raise> {
    converted_arg::<Owned<Color>>(ruby, value)?;
    color_arg(ruby, value)
}

/// `Bitmap#font=` of RGSS3: copies the name and size objects, bold, italic, shadow, outline
/// and the values of both colours of the font `given` into the font `own`, whose colour
/// objects stay its own. Anything but a `Font` raises `TypeError` with the message of the
/// original.
pub(crate) fn font_assign_rgss3(ruby: &Ruby, own: VALUE, given: VALUE) -> Result<(), Raise> {
    let data = converted_arg::<FontData>(ruby, given)?.get();
    let font = font_arg(ruby, given)?;
    this(ruby, own)?.with(|own| *own = data);
    assign_color_value(ruby, own, COLOR, font.color)?;
    assign_color_value(ruby, own, OUT_COLOR, font.out_color)?;
    keep_objects_of(ruby, own, given)
}

/// Copies `color` into the `Color` object in instance variable `name` of `object`, or gives
/// it one.
fn assign_color_value(ruby: &Ruby, object: VALUE, name: &CStr, color: Color) -> Result<(), Raise> {
    let own = rb::ivar_get(ruby, object, name)?;
    if ffi::nil_p(own) {
        give_color(ruby, object, name, color)
    } else {
        color_set(ruby, own, color)
    }
}

/// `initialize(name = Font.default_name, size = Font.default_size)` of RGSS3: at most two
/// arguments, the name not checked, the size by [`size_arg`]. The objects given are kept,
/// and without them those of the defaults.
fn initialize_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    wrap::intern_owner_names(ruby)?;
    rb::check_count_range(ruby, args, 0, 2)?;
    this(ruby, recv)?;
    let names = args
        .first()
        .map(|&name| drawn_name(ruby, name).map(names_of))
        .transpose()?;
    let size = args
        .get(1)
        .map(|&size| size_arg(ruby, size, "bad value for size"))
        .transpose()?;
    let (defaults, _) = defaults(ruby)?;
    let font = Font::with(&defaults, names, size);
    this(ruby, recv)?.with(|data| {
        *data = FontData {
            font: font.clone(),
            names_listed: false,
        }
    });
    give_colors(ruby, recv, &font)?;
    keep_default_objects(ruby, recv)?;
    if let Some(&name) = args.first() {
        rb::ivar_set(ruby, recv, NAME, name)?;
    }
    if let Some(&size) = args.get(1) {
        rb::ivar_set(ruby, recv, SIZE, size)?;
    }
    Ok(recv)
}

/// The copy of RGSS3 takes every attribute of the original and gets colour objects of its
/// own.
fn initialize_copy_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let data = this(ruby, args[0])?.get();
    let font = font_arg(ruby, args[0])?;
    this(ruby, recv)?.with(|copy| *copy = data);
    give_colors(ruby, recv, &font)?;
    keep_objects_of(ruby, recv, args[0])?;
    Ok(recv)
}

/// The object given for the name.
fn name_rgss3(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    Ok(rb::ivar_get(ruby, recv, NAME)?)
}

fn set_name_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    let names = names_of(drawn_name(ruby, args[0])?);
    this(ruby, recv)?.with(|data| data.font.names = names);
    rb::ivar_set(ruby, recv, NAME, args[0])?;
    Ok(args[0])
}

/// The number given for the size.
fn size_rgss3(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    Ok(rb::ivar_get(ruby, recv, SIZE)?)
}

fn set_size_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    let size = size_arg(ruby, args[0], "bad value for size")?;
    this(ruby, recv)?.with(|data| data.font.size = size);
    rb::ivar_set(ruby, recv, SIZE, args[0])?;
    Ok(args[0])
}

fn set_color_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    let color = color_arg_rgss3(ruby, args[0])?;
    assign_color_value(ruby, recv, COLOR, color)?;
    Ok(args[0])
}

fn outline(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::boolean(
        this(ruby, recv)?.with(|data| data.font.outline),
    ))
}

/// `outline=`: `false` and `nil` turn the outline off, everything else turns it on.
fn set_outline(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let outline = rb::truthy(args[0]);
    this(ruby, recv)?.with(|data| data.font.outline = outline);
    Ok(args[0])
}

/// The font's own out colour object.
fn out_color(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    Ok(rb::ivar_get(ruby, recv, OUT_COLOR)?)
}

fn set_out_color(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    let color = color_arg_rgss3(ruby, args[0])?;
    assign_color_value(ruby, recv, OUT_COLOR, color)?;
    Ok(args[0])
}

/// A reader of the class that returns the object of its instance variable `name`.
fn class_object(ruby: &Ruby, name: &CStr) -> HostResult {
    Ok(rb::ivar_get(ruby, guest::classes(ruby).font, name)?)
}

/// Keeps `value` in the instance variable `name` of the class.
fn keep_class_object(ruby: &Ruby, name: &CStr, value: VALUE) -> HostResult {
    rb::ivar_set(ruby, guest::classes(ruby).font, name, value)?;
    Ok(value)
}

fn set_default_name_rgss3(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let names = names_of(drawn_name(ruby, args[0])?);
    Guest::with(ruby, |guest| guest.font_defaults.names = names);
    keep_class_object(ruby, DEFAULT_NAME, args[0])
}

fn set_default_size_rgss3(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let size = size_arg(ruby, args[0], "bad value for default_size")?;
    Guest::with(ruby, |guest| guest.font_defaults.size = size);
    keep_class_object(ruby, DEFAULT_SIZE, args[0])
}

/// A writer of a flag of the defaults of RGSS3: the object is kept for the reader, and new
/// fonts take its truth.
fn set_default_flag(
    ruby: &Ruby,
    name: &CStr,
    value: VALUE,
    flag: fn(&mut FontDefaults) -> &mut bool,
) -> HostResult {
    let truth = rb::truthy(value);
    Guest::with(ruby, |guest| *flag(&mut guest.font_defaults) = truth);
    keep_class_object(ruby, name, value)
}

/// A writer of a default colour of RGSS3: the `Color` given is kept, not copied, so that a
/// later change of it changes the default. Anything else raises `TypeError`.
fn set_default_color_object(ruby: &Ruby, name: &CStr, value: VALUE) -> HostResult {
    color_arg_rgss3(ruby, value)?;
    keep_class_object(ruby, name, value)
}

/// `Font.exist?(name)` of RGSS3: whether the text backend has a family of exactly this
/// name, letter case included. A Symbol is asked by its name; any other object that is no
/// String answers false.
fn exist_rgss3(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let exists = match asked_name(ruby, args[0])? {
        Some(name) => Guest::with(ruby, |guest| guest.text.font_exists_exactly(&name)),
        None => false,
    };
    Ok(rb::boolean(exists))
}

/// Defines `Font` of RGSS3: the methods of RGSS2, with functions of their own for those
/// that keep the objects they are given and check them as measured, and the outline and
/// the out colour.
pub(super) fn define_rgss3(ruby: &Ruby, class: VALUE) -> Result<(), Raised> {
    rb::method(ruby, class, c"initialize", -1, initialize_rgss3)?;
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy_rgss3)?;
    rb::method(ruby, class, c"name", 0, name_rgss3)?;
    rb::method(ruby, class, c"name=", 1, set_name_rgss3)?;
    rb::method(ruby, class, c"size", 0, size_rgss3)?;
    rb::method(ruby, class, c"size=", 1, set_size_rgss3)?;
    rb::method(ruby, class, c"bold", 0, bold)?;
    rb::method(ruby, class, c"bold=", 1, set_bold)?;
    rb::method(ruby, class, c"italic", 0, italic)?;
    rb::method(ruby, class, c"italic=", 1, set_italic)?;
    rb::method(ruby, class, c"shadow", 0, shadow)?;
    rb::method(ruby, class, c"shadow=", 1, set_shadow)?;
    rb::method(ruby, class, c"color", 0, color)?;
    rb::method(ruby, class, c"color=", 1, set_color_rgss3)?;
    rb::method(ruby, class, c"outline", 0, outline)?;
    rb::method(ruby, class, c"outline=", 1, set_outline)?;
    rb::method(ruby, class, c"out_color", 0, out_color)?;
    rb::method(ruby, class, c"out_color=", 1, set_out_color)?;
    rb::class_method(ruby, class, c"default_name", 0, |ruby, _, _| {
        class_object(ruby, DEFAULT_NAME)
    })?;
    rb::class_method(ruby, class, c"default_name=", 1, set_default_name_rgss3)?;
    rb::class_method(ruby, class, c"default_size", 0, |ruby, _, _| {
        class_object(ruby, DEFAULT_SIZE)
    })?;
    rb::class_method(ruby, class, c"default_size=", 1, set_default_size_rgss3)?;
    rb::class_method(ruby, class, c"default_bold", 0, |ruby, _, _| {
        class_object(ruby, DEFAULT_BOLD)
    })?;
    rb::class_method(ruby, class, c"default_bold=", 1, |ruby, _, args| {
        set_default_flag(ruby, DEFAULT_BOLD, args[0], |defaults| &mut defaults.bold)
    })?;
    rb::class_method(ruby, class, c"default_italic", 0, |ruby, _, _| {
        class_object(ruby, DEFAULT_ITALIC)
    })?;
    rb::class_method(ruby, class, c"default_italic=", 1, |ruby, _, args| {
        set_default_flag(ruby, DEFAULT_ITALIC, args[0], |defaults| {
            &mut defaults.italic
        })
    })?;
    rb::class_method(ruby, class, c"default_shadow", 0, |ruby, _, _| {
        class_object(ruby, DEFAULT_SHADOW)
    })?;
    rb::class_method(ruby, class, c"default_shadow=", 1, |ruby, _, args| {
        set_default_flag(ruby, DEFAULT_SHADOW, args[0], |defaults| {
            &mut defaults.shadow
        })
    })?;
    rb::class_method(ruby, class, c"default_outline", 0, |ruby, _, _| {
        class_object(ruby, DEFAULT_OUTLINE)
    })?;
    rb::class_method(ruby, class, c"default_outline=", 1, |ruby, _, args| {
        set_default_flag(ruby, DEFAULT_OUTLINE, args[0], |defaults| {
            &mut defaults.outline
        })
    })?;
    rb::class_method(ruby, class, c"default_color", 0, default_color)?;
    rb::class_method(ruby, class, c"default_color=", 1, |ruby, _, args| {
        set_default_color_object(ruby, DEFAULT_COLOR, args[0])
    })?;
    rb::class_method(ruby, class, c"default_out_color", 0, |ruby, _, _| {
        class_object(ruby, DEFAULT_OUT_COLOR)
    })?;
    rb::class_method(ruby, class, c"default_out_color=", 1, |ruby, _, args| {
        set_default_color_object(ruby, DEFAULT_OUT_COLOR, args[0])
    })?;
    rb::class_method(ruby, class, c"exist?", 1, exist_rgss3)
}
