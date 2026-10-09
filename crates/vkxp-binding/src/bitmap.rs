//! The Ruby class `Bitmap`.
//!
//! A `Bitmap` object holds the id of a bitmap in the guest's arena; the pixels live there.
//! Its font is a `Font` object kept in an instance variable of the bitmap, so that
//! `bitmap.font.size = 30` changes how the bitmap draws: the attributes of that object are
//! copied into the arena's bitmap before text is drawn or measured.

use std::ffi::CStr;

use vkxp_core::{Align, BitmapError, Rect};
use vkxp_proto::BitmapId;
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkxp_vfs::FileKind;

use crate::color::{color_arg, color_new};
use crate::error::{self, Error};
use crate::font::{font_arg, font_copy, font_new_default};
use crate::guest::{self, Classes, Guest};
use crate::rb;
use crate::rect::{rect_arg, rect_new};
use crate::wrap::{self, Handle, Wrapped};

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
    Guest::with(operation).map_err(|error| error::to_raise(ruby, error))
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
        Guest::with(|guest| guest.bitmaps.dispose(previous));
    }
    Ok(())
}

/// An opacity argument, limited to 0 to 255.
fn opacity_arg(ruby: &Ruby, value: Option<&VALUE>) -> Result<u8, Raise> {
    match value {
        Some(&value) => Ok(rb::int(ruby, value)?.clamp(0, 255) as u8),
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

/// `initialize(width, height)` creates a transparent bitmap; `initialize(path)` loads an
/// image found through the game's files.
fn initialize(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::check_count(ruby, args, 1, 2)?;
    this(ruby, recv)?;
    let id = if let [path] = args {
        let path = rb::text(ruby, *path)?;
        operate(ruby, |guest| {
            let bytes = guest.files.read(FileKind::Image, &path)?;
            let image = vkxp_image::decode(&bytes)?;
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
        Guest::with(|guest| guest.bitmaps.dispose(id));
    }
    Ok(ffi::Qnil)
}

fn disposed(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let disposed = match this(ruby, recv)?.get().id {
        Some(id) => Guest::with(|guest| guest.bitmaps.is_disposed(id)),
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
    rb::check_count(ruby, args, 4, 5)?;
    let id = id_arg(ruby, recv)?;
    let x = rb::int(ruby, args[0])?;
    let y = rb::int(ruby, args[1])?;
    let source = id_arg(ruby, args[2])?;
    let source_rect = rect_arg(ruby, args[3])?;
    let opacity = opacity_arg(ruby, args.get(4))?;
    operate(ruby, |guest| {
        Ok(guest.bitmaps.blt(id, x, y, source, source_rect, opacity)?)
    })?;
    Ok(ffi::Qnil)
}

/// `stretch_blt(dest_rect, source, source_rect, opacity = 255)`.
fn stretch_blt(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    rb::check_count(ruby, args, 3, 4)?;
    let id = id_arg(ruby, recv)?;
    let dest_rect = rect_arg(ruby, args[0])?;
    let source = id_arg(ruby, args[1])?;
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
    let id = id_arg(ruby, recv)?;
    operate(ruby, |guest| Ok(guest.bitmaps.clear(id)?))?;
    Ok(ffi::Qnil)
}

fn get_pixel(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = id_arg(ruby, recv)?;
    let x = rb::int(ruby, args[0])?;
    let y = rb::int(ruby, args[1])?;
    let color = operate(ruby, |guest| Ok(guest.bitmaps.get_pixel(id, x, y)?))?;
    color_new(ruby, color)
}

fn set_pixel(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
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

/// Copies the attributes of the bitmap's `Font` object into the arena's bitmap, which is
/// where drawing and measuring take them from.
fn apply_font(ruby: &Ruby, recv: VALUE, id: BitmapId) -> Result<(), Raise> {
    let font = rb::ivar_get(ruby, recv, FONT)?;
    if ffi::nil_p(font) {
        return Ok(());
    }
    let font = font_arg(ruby, font)?;
    operate(ruby, |guest| Ok(guest.bitmaps.set_font(id, font)?))
}

/// `draw_text(x, y, width, height, string, align = 0)` or `draw_text(rect, string, align = 0)`.
/// Text that is not a `String` raises `TypeError`, as in the original.
fn draw_text(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = id_arg(ruby, recv)?;
    let (rect, rest) = match args.len() {
        2 | 3 => (rect_arg(ruby, args[0])?, &args[1..]),
        5 | 6 => (rect_of(ruby, &args[..4])?, &args[4..]),
        _ => return Err(rb::count_error(ruby, args, &[2, 3, 5, 6])),
    };
    let text = rb::text(ruby, rest[0])?;
    let align = match rest.get(1).map(|&align| rb::int(ruby, align)).transpose()? {
        Some(1) => Align::Center,
        Some(2) => Align::Right,
        _ => Align::Left,
    };
    apply_font(ruby, recv, id)?;
    operate(ruby, |guest| {
        Ok(guest
            .bitmaps
            .draw_text(id, rect, &text, align, &*guest.text)?)
    })?;
    Ok(ffi::Qnil)
}

/// `text_size(string)`: a `Rect` at 0, 0 with the size the text has in the bitmap's font.
/// Text that is not a `String` raises `TypeError`.
fn text_size(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let id = id_arg(ruby, recv)?;
    let text = rb::text(ruby, args[0])?;
    apply_font(ruby, recv, id)?;
    let size = operate(ruby, |guest| {
        Ok(guest.bitmaps.text_size(id, &text, &*guest.text)?)
    })?;
    rect_new(ruby, size)
}

/// Defines `Bitmap`.
pub(crate) fn define(ruby: &Ruby) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<BitmapData>(ruby, c"Bitmap")?;
    rb::method(ruby, class, c"initialize", -1, initialize)?;
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
    rb::method(ruby, class, c"font=", 1, set_font)?;
    rb::method(ruby, class, c"draw_text", -1, draw_text)?;
    rb::method(ruby, class, c"text_size", 1, text_size)?;
    Ok(class)
}
