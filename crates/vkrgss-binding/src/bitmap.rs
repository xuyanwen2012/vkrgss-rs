//! The Ruby class `Bitmap`.
//!
//! A `Bitmap` object holds the id of a bitmap in the guest's arena; the pixels live there.
//! Its font is a `Font` object kept in an instance variable of the bitmap, so that
//! `bitmap.font.size = 30` changes how the bitmap draws: the attributes of that object are
//! copied into the arena's bitmap before text is drawn or measured.
//!
//! In a game of RGSS2 the class has four more methods (`blur`, `radial_blur`, `clear_rect`,
//! `gradient_fill_rect`) and differs in `Bitmap.new`, `font=`, `draw_text` and `text_size`,
//! as measured on the original runtime of VX.
//!
//! In a game of RGSS3 the class has the methods of RGSS2. `Bitmap.new` has no limit of its
//! own, `font=` copies every attribute, text is drawn by the rules of RGSS3 of the core and
//! not at all with a font without a name, and the methods that take several argument
//! counts or a `Color`, `Rect`, `Bitmap` or `Font` raise the errors of the original runtime
//! of VX Ace.

use std::ffi::CStr;

use vkrgss_core::{Align, BitmapError, Color, CreateFailure, Rect, Squeeze, TextRules};
use vkrgss_proto::BitmapId;
use vkrgss_ruby_sys::ffi::{self, VALUE};
use vkrgss_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkrgss_vfs::{FileKind, Runtime};

use crate::color::{color_arg, color_new};
use crate::error::{self, Error};
use crate::font::{
    FontData, font_arg, font_assign, font_assign_rgss3, font_copy, font_new_default,
};
use crate::guest::{self, Classes, Guest};
use crate::rb;
use crate::rect::{rect_arg, rect_new};
use crate::stats::{Part, Span};
use crate::wrap::{self, Handle, Owned, Wrapped};

/// The instance variable of a bitmap that holds its `Font` object. Without `@`, so scripts
/// do not see it.
const FONT: &CStr = c"font";

/// The data of a `Bitmap` object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BitmapData {
    /// The bitmap in the arena; `None` before `initialize` and after `dispose`.
    id: Option<BitmapId>,
}

impl Wrapped for BitmapData {
    const CLASS_NAME: &'static str = "Bitmap";

    fn class(classes: &Classes) -> VALUE {
        classes.bitmap
    }

    fn blank() -> Self {
        BitmapData { id: None }
    }

    fn collected(self) {
        if let Some(id) = self.id {
            guest::defer_free(id);
        }
    }
}

fn this(ruby: &Ruby, recv: VALUE) -> Result<Handle<BitmapData>, Raise> {
    wrap::expect(ruby, recv)
}

/// Runs an operation on the guest and turns its failure into the Ruby exception.
fn operate<R>(
    ruby: &Ruby,
    operation: impl FnOnce(&mut Guest) -> Result<R, Error>,
) -> Result<R, Raise> {
    Guest::with(ruby, operation).map_err(|error| error::to_raise(ruby, error))
}

/// The arena id of a `Bitmap` argument. Anything else raises `TypeError`, a disposed bitmap
/// `RGSSError`.
fn id_arg(ruby: &Ruby, value: VALUE) -> Result<BitmapId, Raise> {
    this(ruby, value)?
        .get()
        .id
        .ok_or_else(|| error::to_raise(ruby, BitmapError::Disposed.into()))
}

/// The arena id of a `Bitmap` argument that an element is to draw from, `None` when the
/// bitmap is disposed. Anything else raises `TypeError`.
pub(crate) fn bitmap_arg(ruby: &Ruby, value: VALUE) -> Result<Option<BitmapId>, Raise> {
    Ok(this(ruby, value)?.get().id)
}

/// Makes `id` the bitmap of the object, disposing the one it had.
fn adopt(ruby: &Ruby, recv: VALUE, id: BitmapId) -> Result<(), Raise> {
    let previous = this(ruby, recv)?.with(|data| data.id.replace(id));
    if let Some(previous) = previous {
        Guest::with(ruby, |guest| guest.bitmaps.dispose(previous));
    }
    Ok(())
}

/// A new `Bitmap` object of the bitmap `id` of the arena, with a font of its own as
/// `Bitmap.new` gives one. The bitmap is disposed when the object cannot be made.
pub(crate) fn bitmap_new(ruby: &Ruby, id: BitmapId) -> HostResult {
    let made = wrap::new_object(ruby, BitmapData { id: Some(id) }).and_then(|object| {
        let font = font_new_default(ruby)?;
        rb::ivar_set(ruby, object, FONT, font)?;
        Ok(object)
    });
    // An object that was made owns the bitmap, and gives it back when it is collected.
    if made.is_err() {
        Guest::with(ruby, |guest| guest.bitmaps.dispose(id));
    }
    made
}

/// A new `Bitmap` object of `width` by `height` transparent pixels, as `Bitmap.new` makes
/// one.
pub(crate) fn new_blank(ruby: &Ruby, width: u32, height: u32) -> HostResult {
    let id = operate(ruby, |guest| {
        Ok(guest.bitmaps.create(width, height, &guest.font_defaults)?)
    })?;
    bitmap_new(ruby, id)
}

/// An opacity argument: the integer given, which the transfer does not limit to 255.
fn opacity_arg(ruby: &Ruby, value: Option<&VALUE>) -> Result<i32, Raise> {
    match value {
        Some(&value) => rb::int(ruby, value),
        None => Ok(255),
    }
}

/// Four integer arguments as a rectangle.
fn rect_of(ruby: &Ruby, args: &[VALUE]) -> Result<Rect, Raise> {
    Ok(Rect::new(
        rb::int(ruby, args[0])?,
        rb::int(ruby, args[1])?,
        rb::int(ruby, args[2])?,
        rb::int(ruby, args[3])?,
    ))
}

/// Largest bitmap `Bitmap.new(width, height)` of RGSS2 makes, in pixels, as measured on the
/// original: 2048 by 2048 works and 2049 by 2048 does not. A picture file may be larger.
const RGSS2_MAX_NEW_PIXELS: u64 = 4_194_304;

/// `initialize` of RGSS1.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    initialize_with(ruby, recv, args, None)
}

/// `initialize` of RGSS2: a size above [`RGSS2_MAX_NEW_PIXELS`] raises `RGSSError`.
fn initialize_rgss2(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    initialize_with(ruby, recv, args, Some(RGSS2_MAX_NEW_PIXELS))
}

/// `initialize(width, height)` creates a transparent bitmap, of at most `limit` pixels when
/// there is one; `initialize(path)` loads an image found through the game's files.
fn initialize_with(ruby: &Ruby, recv: VALUE, args: &[VALUE], limit: Option<u64>) -> HostResult {
    wrap::intern_owner_names(ruby)?;
    rb::check_count(ruby, args, 1, 2)?;
    this(ruby, recv)?;
    let id = if let [path] = args {
        let path = rb::text(ruby, *path)?;
        operate(ruby, |guest| {
            let bytes = guest.files.read(FileKind::Image, &path)?;
            let image = vkrgss_image::decode(&bytes)?;
            Ok(guest.bitmaps.create_from_rgba(
                image.width,
                image.height,
                image.rgba,
                &guest.font_defaults,
            )?)
        })?
    } else {
        // A negative size has no pixels either: the arena refuses it as it refuses 0.
        let width = rb::int(ruby, args[0])?.max(0).unsigned_abs();
        let height = rb::int(ruby, args[1])?.max(0).unsigned_abs();
        operate(ruby, |guest| {
            if let Some(limit) = limit
                && u64::from(width) * u64::from(height) > limit
            {
                let too_large = CreateFailure::TooLarge {
                    width,
                    height,
                    limit,
                };
                return Err(BitmapError::from(too_large).into());
            }
            Ok(guest.bitmaps.create(width, height, &guest.font_defaults)?)
        })?
    };
    adopt(ruby, recv, id)?;
    let font = font_new_default(ruby)?;
    rb::ivar_set(ruby, recv, FONT, font)?;
    Ok(recv)
}

/// The copy gets a bitmap of its own with the original's pixels, and a `Font` object of its
/// own with the original's attributes; the instance variable Ruby copied still names the
/// original's.
fn initialize_copy(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    this(ruby, recv)?;
    let original = id_arg(ruby, args[0])?;
    let id = operate(ruby, |guest| {
        let size = guest.bitmaps.rect(original)?;
        let rgba = guest.bitmaps.rgba(original)?.to_vec();
        Ok(guest.bitmaps.create_from_rgba(
            size.width.unsigned_abs(),
            size.height.unsigned_abs(),
            rgba,
            &guest.font_defaults,
        )?)
    })?;
    adopt(ruby, recv, id)?;
    let original_font = rb::ivar_get(ruby, args[0], FONT)?;
    let font = if ffi::nil_p(original_font) {
        font_new_default(ruby)?
    } else {
        font_copy(ruby, original_font)?
    };
    rb::ivar_set(ruby, recv, FONT, font)?;
    Ok(recv)
}

fn dispose(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let id = this(ruby, recv)?.with(|data| data.id.take());
    if let Some(id) = id {
        Guest::with(ruby, |guest| guest.bitmaps.dispose(id));
    }
    Ok(ffi::Qnil)
}

fn disposed(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let disposed = match this(ruby, recv)?.get().id {
        Some(id) => Guest::with(ruby, |guest| guest.bitmaps.is_disposed(id)),
        None => true,
    };
    Ok(rb::boolean(disposed))
}

fn width(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let id = id_arg(ruby, recv)?;
    let rect = operate(ruby, |guest| Ok(guest.bitmaps.rect(id)?))?;
    Ok(rb::fix(rect.width))
}

fn height(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let id = id_arg(ruby, recv)?;
    let rect = operate(ruby, |guest| Ok(guest.bitmaps.rect(id)?))?;
    Ok(rb::fix(rect.height))
}

fn rect(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let id = id_arg(ruby, recv)?;
    let rect = operate(ruby, |guest| Ok(guest.bitmaps.rect(id)?))?;
    rect_new(ruby, rect)
}

/// `blt(x, y, source, source_rect, opacity = 255)`.
fn blt(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    rb::check_count(ruby, args, 4, 5)?;
    let id = id_arg(ruby, recv)?;
    let x = rb::int(ruby, args[0])?;
    let y = rb::int(ruby, args[1])?;
    // A disposed source raises nothing: the transfer has a rule for it.
    let source = bitmap_arg(ruby, args[2])?;
    let source_rect = rect_arg(ruby, args[3])?;
    let opacity = opacity_arg(ruby, args.get(4))?;
    operate(ruby, |guest| {
        Ok(guest.bitmaps.blt(id, x, y, source, source_rect, opacity)?)
    })?;
    Ok(ffi::Qnil)
}

/// `stretch_blt(dest_rect, source, source_rect, opacity = 255)`.
fn stretch_blt(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    rb::check_count(ruby, args, 3, 4)?;
    let id = id_arg(ruby, recv)?;
    let dest_rect = rect_arg(ruby, args[0])?;
    let source = bitmap_arg(ruby, args[1])?;
    let source_rect = rect_arg(ruby, args[2])?;
    let opacity = opacity_arg(ruby, args.get(3))?;
    operate(ruby, |guest| {
        Ok(guest
            .bitmaps
            .stretch_blt(id, dest_rect, source, source_rect, opacity)?)
    })?;
    Ok(ffi::Qnil)
}

/// `fill_rect(rect, color)` or `fill_rect(x, y, width, height, color)`.
fn fill_rect(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let (rect, color) = match args {
        [rect, color] => (rect_arg(ruby, *rect)?, color_arg(ruby, *color)?),
        [x, y, width, height, color] => (
            rect_of(ruby, &[*x, *y, *width, *height])?,
            color_arg(ruby, *color)?,
        ),
        _ => return Err(rb::count_error(ruby, args, &[2, 5])),
    };
    operate(ruby, |guest| {
        Ok(guest.bitmaps.fill_rect(id, rect, color)?)
    })?;
    Ok(ffi::Qnil)
}

fn clear(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    operate(ruby, |guest| Ok(guest.bitmaps.clear(id)?))?;
    Ok(ffi::Qnil)
}

fn get_pixel(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let x = rb::int(ruby, args[0])?;
    let y = rb::int(ruby, args[1])?;
    let color = operate(ruby, |guest| Ok(guest.bitmaps.get_pixel(id, x, y)?))?;
    color_new(ruby, color)
}

fn set_pixel(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let x = rb::int(ruby, args[0])?;
    let y = rb::int(ruby, args[1])?;
    let color = color_arg(ruby, args[2])?;
    operate(ruby, |guest| {
        Ok(guest.bitmaps.set_pixel(id, x, y, color)?)
    })?;
    Ok(ffi::Qnil)
}

fn hue_change(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let hue = rb::int(ruby, args[0])?;
    operate(ruby, |guest| Ok(guest.bitmaps.hue_change(id, hue)?))?;
    Ok(ffi::Qnil)
}

/// The bitmap's own `Font` object.
fn font(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    id_arg(ruby, recv)?;
    Ok(rb::ivar_get(ruby, recv, FONT)?)
}

fn set_font(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    id_arg(ruby, recv)?;
    font_arg(ruby, args[0])?;
    rb::ivar_set(ruby, recv, FONT, args[0])?;
    Ok(args[0])
}

/// `font=` of RGSS2: the name, the size, bold, italic and the colour of the font given are
/// copied into the bitmap's own `Font` object, which keeps its shadow.
fn set_font_rgss2(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    id_arg(ruby, recv)?;
    font_arg(ruby, args[0])?;
    let own = rb::ivar_get(ruby, recv, FONT)?;
    if ffi::nil_p(own) {
        let copy = font_copy(ruby, args[0])?;
        rb::ivar_set(ruby, recv, FONT, copy)?;
    } else {
        font_assign(ruby, own, args[0])?;
    }
    Ok(args[0])
}

/// A text argument of RGSS2 and RGSS3: any object, as its `to_s`.
pub(crate) fn text_of(ruby: &Ruby, value: VALUE) -> Result<String, Raise> {
    if rb::string_bytes(ruby, value).is_some() {
        return rb::text(ruby, value);
    }
    // SAFETY: `value` is an argument of the running method, and so live.
    let string = unsafe { ruby.funcall(value, c"to_s", &[]) }?;
    rb::text(ruby, string)
}

/// Copies the attributes of the bitmap's `Font` object into the arena's bitmap, which is
/// where drawing and measuring take them from. Returns whether the font has a family name
/// to draw with, which only a font of RGSS3 can lack.
fn apply_font(ruby: &Ruby, recv: VALUE, id: BitmapId) -> Result<bool, Raise> {
    let font = rb::ivar_get(ruby, recv, FONT)?;
    if ffi::nil_p(font) {
        return Ok(true);
    }
    let font = font_arg(ruby, font)?;
    let named = !font.names.is_empty();
    operate(ruby, |guest| Ok(guest.bitmaps.set_font(id, font)?))?;
    Ok(named)
}

/// `draw_text` of RGSS1. Text that is not a `String` raises `TypeError`, as in the original.
fn draw_text(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    draw_text_as(ruby, recv, args, Runtime::Rgss1)
}

/// `draw_text` of RGSS2: the text is the `to_s` of any object, and it is squeezed by the
/// rule of RGSS2.
fn draw_text_rgss2(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    draw_text_as(ruby, recv, args, Runtime::Rgss2)
}

/// `draw_text(x, y, width, height, string, align = 0)` or `draw_text(rect, string, align = 0)`.
fn draw_text_as(ruby: &Ruby, recv: VALUE, args: &[VALUE], runtime: Runtime) -> HostResult {
    let _span = Span::enter(Part::Text);
    let id = id_arg(ruby, recv)?;
    let (rect, rest) = match args.len() {
        2 | 3 => (rect_arg(ruby, args[0])?, &args[1..]),
        5 | 6 => (rect_of(ruby, &args[..4])?, &args[4..]),
        _ => return Err(rb::count_error(ruby, args, &[2, 3, 5, 6])),
    };
    let (text, squeeze) = match runtime {
        Runtime::Rgss1 => (rb::text(ruby, rest[0])?, Squeeze::Rgss1),
        Runtime::Rgss2 | Runtime::Rgss3 => (text_of(ruby, rest[0])?, Squeeze::Rgss2),
    };
    let align = match rest.get(1).map(|&align| rb::int(ruby, align)).transpose()? {
        Some(1) => Align::Center,
        Some(2) => Align::Right,
        _ => Align::Left,
    };
    apply_font(ruby, recv, id)?;
    operate(ruby, |guest| {
        Ok(guest
            .bitmaps
            .draw_text_squeezed(id, rect, &text, align, squeeze, &*guest.text)?)
    })?;
    Ok(ffi::Qnil)
}

/// `text_size(string)`: a `Rect` at 0, 0 with the size the text has in the bitmap's font.
/// Text that is not a `String` raises `TypeError`.
fn text_size(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    // A disposed bitmap is reported before the text is looked at.
    id_arg(ruby, recv)?;
    let text = rb::text(ruby, args[0])?;
    text_size_of(ruby, recv, &text)
}

/// `text_size` of RGSS2: the text is the `to_s` of any object. The shadow does not count.
fn text_size_rgss2(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    // A disposed bitmap is reported before the text is converted.
    id_arg(ruby, recv)?;
    let text = text_of(ruby, args[0])?;
    text_size_of(ruby, recv, &text)
}

fn text_size_of(ruby: &Ruby, recv: VALUE, text: &str) -> HostResult {
    let _span = Span::enter(Part::Text);
    let id = id_arg(ruby, recv)?;
    apply_font(ruby, recv, id)?;
    let size = operate(ruby, |guest| {
        Ok(guest.bitmaps.text_size(id, text, &*guest.text)?)
    })?;
    rect_new(ruby, size)
}

/// The `ArgumentError` of the methods RGSS2 adds, with the message of the original, which
/// names the count it expects first and the count given second.
fn rgss2_count_error(ruby: &Ruby, expected: usize, given: usize) -> Raise {
    Raise::new(
        rb::arg_error(ruby),
        format!("wrong # of arguments ({expected} for {given})"),
    )
}

/// Whether a value is a `Rect`.
fn is_rect(ruby: &Ruby, value: VALUE) -> bool {
    wrap::find::<Owned<Rect>>(ruby, value).is_some()
}

/// The handle of an argument that must be an instance of the class of `T`, as the original
/// runtime of RGSS2 checks the colours of `gradient_fill_rect` and that of RGSS3 every
/// `Color`, `Rect`, `Bitmap` and `Font` argument: anything else raises `TypeError` with
/// `can't convert <class> into <expected class>`.
pub(crate) fn converted_arg<T: Wrapped>(ruby: &Ruby, value: VALUE) -> Result<Handle<T>, Raise> {
    wrap::find::<T>(ruby, value).ok_or_else(|| {
        rb::wrong_type(ruby, value, |class| {
            format!("can't convert {class} into {}", T::CLASS_NAME)
        })
    })
}

/// A colour argument of `gradient_fill_rect`. Anything but a `Color` raises `TypeError`
/// with the message of the original.
fn gradient_color_arg(ruby: &Ruby, value: VALUE) -> Result<Color, Raise> {
    Ok(converted_arg::<Owned<Color>>(ruby, value)?.get().value())
}

/// `blur` of RGSS2.
fn blur(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    operate(ruby, |guest| Ok(guest.bitmaps.blur(id)?))?;
    Ok(ffi::Qnil)
}

/// `radial_blur(angle, division)` of RGSS2: two integers, limited inside to 0 to 360 and 2
/// to 100.
fn radial_blur(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let angle = rb::int(ruby, args[0])?;
    let division = rb::int(ruby, args[1])?;
    operate(ruby, |guest| {
        Ok(guest.bitmaps.radial_blur(id, angle, division)?)
    })?;
    Ok(ffi::Qnil)
}

/// `clear_rect(rect)` or `clear_rect(x, y, width, height)` of RGSS2. A single argument that
/// is not a `Rect` is a wrong count, as in the original. With no argument the original ends
/// with a page fault; here it is a wrong count too.
fn clear_rect(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let rect = match args {
        [rect] if is_rect(ruby, *rect) => rect_arg(ruby, *rect)?,
        [first, ..] if is_rect(ruby, *first) => {
            return Err(rgss2_count_error(ruby, 1, args.len()));
        }
        [_, _, _, _] => rect_of(ruby, args)?,
        _ => return Err(rgss2_count_error(ruby, 4, args.len())),
    };
    operate(ruby, |guest| Ok(guest.bitmaps.clear_rect(id, rect)?))?;
    Ok(ffi::Qnil)
}

/// `gradient_fill_rect(rect, color1, color2, vertical = false)` or
/// `gradient_fill_rect(x, y, width, height, color1, color2, vertical = false)` of RGSS2.
///
/// With seven arguments the gradient is vertical whatever the seventh is, `false` and `nil`
/// included: this is the behaviour of the original, which tests another argument in its
/// place. With a `Rect` the last argument works as documented. With no argument the
/// original ends with a page fault; here it is a wrong count.
fn gradient_fill_rect(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let given = args.len();
    if given < 3 {
        return Err(rgss2_count_error(ruby, 3, given));
    }
    let (rect, colors, vertical) = if is_rect(ruby, args[0]) {
        if given > 4 {
            return Err(rgss2_count_error(ruby, 4, given));
        }
        let vertical = args.get(3).is_some_and(|&flag| rb::truthy(flag));
        (rect_arg(ruby, args[0])?, &args[1..3], vertical)
    } else {
        match given {
            6 | 7 => (rect_of(ruby, &args[..4])?, &args[4..6], given == 7),
            0..6 => return Err(rgss2_count_error(ruby, 6, given)),
            _ => return Err(rgss2_count_error(ruby, 7, given)),
        }
    };
    let from = gradient_color_arg(ruby, colors[0])?;
    let to = gradient_color_arg(ruby, colors[1])?;
    operate(ruby, |guest| {
        Ok(guest
            .bitmaps
            .gradient_fill_rect(id, rect, from, to, vertical)?)
    })?;
    Ok(ffi::Qnil)
}

// --- RGSS3 -----------------------------------------------------------------------------------

/// The `TypeError` of a path argument of RGSS3 that is no `String`, worded as Ruby 1.9.2
/// words its own conversion: `can't convert <class> into String`, with `nil`, `true` and
/// `false` named so.
fn path_arg_rgss3(ruby: &Ruby, value: VALUE) -> Result<String, Raise> {
    if rb::string_bytes(ruby, value).is_some() {
        return rb::text(ruby, value);
    }
    let message = |name: &str| format!("can't convert {name} into String");
    Err(match value {
        ffi::Qnil => Raise::new(rb::type_error(ruby), message("nil")),
        ffi::Qtrue => Raise::new(rb::type_error(ruby), message("true")),
        ffi::Qfalse => Raise::new(rb::type_error(ruby), message("false")),
        _ => rb::wrong_type(ruby, value, message),
    })
}

/// `initialize` of RGSS3: one or two arguments, `(given for 1..2)` otherwise. A size has no
/// limit of its own: the arena refuses a width of 0 or less, a height of 0 and a bitmap
/// above its largest, and a negative height is taken by its size, as measured.
/// `initialize(path)` is that of RGSS2 but for the message of a path that is no `String`.
///
/// Provenance: oracle measurement 2026-10-13, RGSS301 3.0.1.1 and RGSS300 3.0.0.1,
/// `ace-v1-bitmap-REPORT.md` section 2.
fn initialize_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    wrap::intern_owner_names(ruby)?;
    rb::check_count_range(ruby, args, 1, 2)?;
    this(ruby, recv)?;
    let id = if let [path] = args {
        let path = path_arg_rgss3(ruby, *path)?;
        operate(ruby, |guest| {
            let bytes = guest.files.read(FileKind::Image, &path)?;
            let image = vkrgss_image::decode(&bytes)?;
            Ok(guest.bitmaps.create_from_rgba(
                image.width,
                image.height,
                image.rgba,
                &guest.font_defaults,
            )?)
        })?
    } else {
        let width = rb::int(ruby, args[0])?.max(0).unsigned_abs();
        let height = rb::int(ruby, args[1])?.unsigned_abs();
        operate(ruby, |guest| {
            Ok(guest.bitmaps.create(width, height, &guest.font_defaults)?)
        })?
    };
    adopt(ruby, recv, id)?;
    let font = font_new_default(ruby)?;
    rb::ivar_set(ruby, recv, FONT, font)?;
    Ok(recv)
}

/// `font=` of RGSS3: a `Font` only, whose name and size objects, flags and colour values
/// are copied into the bitmap's own `Font` object.
fn set_font_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    id_arg(ruby, recv)?;
    converted_arg::<FontData>(ruby, args[0])?;
    let own = rb::ivar_get(ruby, recv, FONT)?;
    if ffi::nil_p(own) {
        let copy = font_copy(ruby, args[0])?;
        rb::ivar_set(ruby, recv, FONT, copy)?;
    } else {
        font_assign_rgss3(ruby, own, args[0])?;
    }
    Ok(args[0])
}

/// The rectangle and the rest of the arguments of `draw_text`, `clear_rect` and
/// `gradient_fill_rect` of RGSS3: a `Rect` first takes the short form, of `short` counts,
/// and anything else the form of four integers, of `long` counts. A wrong count names the
/// bound of its form nearer the count given, as `check_count_rgss3` does.
fn rect_form_rgss3<'a>(
    ruby: &Ruby,
    args: &'a [VALUE],
    short: (usize, usize),
    long: (usize, usize),
) -> Result<(Rect, &'a [VALUE]), Raise> {
    if args.first().is_some_and(|&first| is_rect(ruby, first)) {
        rb::check_count_rgss3(ruby, args, short.0, short.1)?;
        Ok((rect_arg(ruby, args[0])?, &args[1..]))
    } else {
        rb::check_count_rgss3(ruby, args, long.0, long.1)?;
        Ok((rect_of(ruby, &args[..4])?, &args[4..]))
    }
}

/// `draw_text` of RGSS3: drawn by [`TextRules::Rgss3`]; with a font without a name nothing
/// is drawn once the arguments are checked.
fn draw_text_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Text);
    let id = id_arg(ruby, recv)?;
    let (rect, rest) = rect_form_rgss3(ruby, args, (2, 3), (5, 6))?;
    let text = text_of(ruby, rest[0])?;
    let align = match rest.get(1).map(|&align| rb::int(ruby, align)).transpose()? {
        Some(1) => Align::Center,
        Some(2) => Align::Right,
        _ => Align::Left,
    };
    if !apply_font(ruby, recv, id)? {
        return Ok(ffi::Qnil);
    }
    operate(ruby, |guest| {
        Ok(guest
            .bitmaps
            .draw_text_as(id, rect, &text, align, TextRules::Rgss3, &*guest.text)?)
    })?;
    Ok(ffi::Qnil)
}

/// `text_size` of RGSS3: the size the core measures, without outline or shadow; (0, 0, 0,
/// 0) for a font without a name, where the original returns uninitialised numbers.
fn text_size_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Text);
    let id = id_arg(ruby, recv)?;
    let text = text_of(ruby, args[0])?;
    if !apply_font(ruby, recv, id)? {
        return rect_new(ruby, Rect::new(0, 0, 0, 0));
    }
    let size = operate(ruby, |guest| {
        Ok(guest.bitmaps.text_size(id, &text, &*guest.text)?)
    })?;
    rect_new(ruby, size)
}

/// A `Rect` argument of RGSS3, with the message of the original for anything else.
fn rect_arg_rgss3(ruby: &Ruby, value: VALUE) -> Result<Rect, Raise> {
    Ok(converted_arg::<Owned<Rect>>(ruby, value)?.get().value())
}

/// A source bitmap of `blt` and `stretch_blt` of RGSS3, `None` when it is disposed, with the
/// message of the original for anything that is no `Bitmap`.
fn source_arg_rgss3(ruby: &Ruby, value: VALUE) -> Result<Option<BitmapId>, Raise> {
    Ok(converted_arg::<BitmapData>(ruby, value)?.get().id)
}

/// `blt` of RGSS3: four or five arguments, with the messages of the original.
fn blt_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    rb::check_count_rgss3(ruby, args, 4, 5)?;
    let id = id_arg(ruby, recv)?;
    let x = rb::int(ruby, args[0])?;
    let y = rb::int(ruby, args[1])?;
    let source = source_arg_rgss3(ruby, args[2])?;
    let source_rect = rect_arg_rgss3(ruby, args[3])?;
    let opacity = opacity_arg(ruby, args.get(4))?;
    operate(ruby, |guest| {
        Ok(guest.bitmaps.blt(id, x, y, source, source_rect, opacity)?)
    })?;
    Ok(ffi::Qnil)
}

/// `stretch_blt` of RGSS3: three or four arguments, with the messages of the original.
fn stretch_blt_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    rb::check_count_rgss3(ruby, args, 3, 4)?;
    let id = id_arg(ruby, recv)?;
    let dest_rect = rect_arg_rgss3(ruby, args[0])?;
    let source = source_arg_rgss3(ruby, args[1])?;
    let source_rect = rect_arg_rgss3(ruby, args[2])?;
    let opacity = opacity_arg(ruby, args.get(3))?;
    operate(ruby, |guest| {
        Ok(guest
            .bitmaps
            .stretch_blt(id, dest_rect, source, source_rect, opacity)?)
    })?;
    Ok(ffi::Qnil)
}

/// `fill_rect` of RGSS3: a `Rect` and a `Color`, or five arguments; every other call is a
/// wrong count that expects five, as measured.
fn fill_rect_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let (rect, color) = match args {
        [rect, color]
            if is_rect(ruby, *rect) && wrap::find::<Owned<Color>>(ruby, *color).is_some() =>
        {
            (rect_arg(ruby, *rect)?, color_arg(ruby, *color)?)
        }
        [x, y, width, height, color] => (
            rect_of(ruby, &[*x, *y, *width, *height])?,
            gradient_color_arg(ruby, *color)?,
        ),
        _ => {
            return Err(Raise::new(
                rb::arg_error(ruby),
                format!("wrong number of arguments (5 for {})", args.len()),
            ));
        }
    };
    operate(ruby, |guest| {
        Ok(guest.bitmaps.fill_rect(id, rect, color)?)
    })?;
    Ok(ffi::Qnil)
}

/// `set_pixel` of RGSS3, with the message of the original for a colour of another class.
fn set_pixel_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let x = rb::int(ruby, args[0])?;
    let y = rb::int(ruby, args[1])?;
    let color = gradient_color_arg(ruby, args[2])?;
    operate(ruby, |guest| {
        Ok(guest.bitmaps.set_pixel(id, x, y, color)?)
    })?;
    Ok(ffi::Qnil)
}

/// `clear_rect` of RGSS3: a `Rect`, or four integers.
fn clear_rect_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let (rect, _) = rect_form_rgss3(ruby, args, (1, 1), (4, 4))?;
    operate(ruby, |guest| Ok(guest.bitmaps.clear_rect(id, rect)?))?;
    Ok(ffi::Qnil)
}

/// `gradient_fill_rect` of RGSS3: a `Rect` and two colours, or four integers and two
/// colours, each with the flag; with seven arguments the gradient is vertical whatever the
/// seventh is, as in RGSS2.
fn gradient_fill_rect_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let _span = Span::enter(Part::Bitmap);
    let id = id_arg(ruby, recv)?;
    let (rect, rest) = rect_form_rgss3(ruby, args, (3, 4), (6, 7))?;
    let vertical = if args.len() == 7 {
        true
    } else {
        rest.get(2).is_some_and(|&flag| rb::truthy(flag))
    };
    let from = gradient_color_arg(ruby, rest[0])?;
    let to = gradient_color_arg(ruby, rest[1])?;
    operate(ruby, |guest| {
        Ok(guest
            .bitmaps
            .gradient_fill_rect(id, rect, from, to, vertical)?)
    })?;
    Ok(ffi::Qnil)
}

/// Defines `Bitmap` as a game of `runtime` has it; RGSS3 has a definition of its own
/// ([`define_rgss3`]).
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<BitmapData>(ruby, c"Bitmap")?;
    if runtime == Runtime::Rgss3 {
        define_rgss3(ruby, class)?;
        return Ok(class);
    }
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy)?;
    rb::method(ruby, class, c"dispose", 0, dispose)?;
    rb::method(ruby, class, c"disposed?", 0, disposed)?;
    rb::method(ruby, class, c"width", 0, width)?;
    rb::method(ruby, class, c"height", 0, height)?;
    rb::method(ruby, class, c"rect", 0, rect)?;
    rb::method(ruby, class, c"blt", -1, blt)?;
    rb::method(ruby, class, c"stretch_blt", -1, stretch_blt)?;
    rb::method(ruby, class, c"fill_rect", -1, fill_rect)?;
    rb::method(ruby, class, c"clear", 0, clear)?;
    rb::method(ruby, class, c"get_pixel", 2, get_pixel)?;
    rb::method(ruby, class, c"set_pixel", 3, set_pixel)?;
    rb::method(ruby, class, c"hue_change", 1, hue_change)?;
    rb::method(ruby, class, c"font", 0, font)?;
    if runtime == Runtime::Rgss1 {
        rb::method(ruby, class, c"initialize", -1, initialize)?;
        rb::method(ruby, class, c"font=", 1, set_font)?;
        rb::method(ruby, class, c"draw_text", -1, draw_text)?;
        rb::method(ruby, class, c"text_size", 1, text_size)?;
    } else {
        rb::method(ruby, class, c"initialize", -1, initialize_rgss2)?;
        rb::method(ruby, class, c"font=", 1, set_font_rgss2)?;
        rb::method(ruby, class, c"draw_text", -1, draw_text_rgss2)?;
        rb::method(ruby, class, c"text_size", 1, text_size_rgss2)?;
        rb::method(ruby, class, c"clear_rect", -1, clear_rect)?;
        rb::method(ruby, class, c"gradient_fill_rect", -1, gradient_fill_rect)?;
        rb::method(ruby, class, c"blur", 0, blur)?;
        rb::method(ruby, class, c"radial_blur", 2, radial_blur)?;
    }
    Ok(class)
}

/// Defines `Bitmap` of RGSS3: the methods of RGSS2, with functions of their own for
/// creation, `font=`, text and the methods whose arguments the original checks otherwise.
fn define_rgss3(ruby: &Ruby, class: VALUE) -> Result<(), Raised> {
    rb::method(ruby, class, c"initialize", -1, initialize_rgss3)?;
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy)?;
    rb::method(ruby, class, c"dispose", 0, dispose)?;
    rb::method(ruby, class, c"disposed?", 0, disposed)?;
    rb::method(ruby, class, c"width", 0, width)?;
    rb::method(ruby, class, c"height", 0, height)?;
    rb::method(ruby, class, c"rect", 0, rect)?;
    rb::method(ruby, class, c"blt", -1, blt_rgss3)?;
    rb::method(ruby, class, c"stretch_blt", -1, stretch_blt_rgss3)?;
    rb::method(ruby, class, c"fill_rect", -1, fill_rect_rgss3)?;
    rb::method(ruby, class, c"clear", 0, clear)?;
    rb::method(ruby, class, c"get_pixel", 2, get_pixel)?;
    rb::method(ruby, class, c"set_pixel", 3, set_pixel_rgss3)?;
    rb::method(ruby, class, c"hue_change", 1, hue_change)?;
    rb::method(ruby, class, c"font", 0, font)?;
    rb::method(ruby, class, c"font=", 1, set_font_rgss3)?;
    rb::method(ruby, class, c"draw_text", -1, draw_text_rgss3)?;
    rb::method(ruby, class, c"text_size", 1, text_size_rgss3)?;
    rb::method(ruby, class, c"clear_rect", -1, clear_rect_rgss3)?;
    rb::method(
        ruby,
        class,
        c"gradient_fill_rect",
        -1,
        gradient_fill_rect_rgss3,
    )?;
    rb::method(ruby, class, c"blur", 0, blur)?;
    rb::method(ruby, class, c"radial_blur", 2, radial_blur)
}
