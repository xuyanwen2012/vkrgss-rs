//! The Ruby class `Rect`.

use vkxp_core::Rect;
use vkxp_ruby_sys::ffi::VALUE;
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};

use crate::error::{self, Error};
use crate::guest::Classes;
use crate::rb;
use crate::wrap::{self, Handle, Owned, Wrapped};

impl Wrapped for Owned<Rect> {
    const CLASS_NAME: &'static str = "Rect";

    fn class(classes: &Classes) -> VALUE {
        classes.rect
    }

    fn blank() -> Self {
        Owned::new(Rect::new(0, 0, 0, 0))
    }
}

/// The value of a `Rect` argument. Anything else raises `TypeError`.
pub(crate) fn rect_arg(ruby: &Ruby, value: VALUE) -> Result<Rect, Raise> {
    Ok(this(ruby, value)?.get().value())
}

/// A new `Rect` object.
pub(crate) fn rect_new(ruby: &Ruby, rect: Rect) -> HostResult {
    wrap::new_object(ruby, Owned::new(rect))
}

fn this(ruby: &Ruby, recv: VALUE) -> Result<Handle<Owned<Rect>>, Raise> {
    wrap::expect(ruby, recv)
}

/// `initialize(x, y, width, height)` and `set` with the same arguments.
fn set(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let rect = Rect::new(
        rb::int(ruby, args[0])?,
        rb::int(ruby, args[1])?,
        rb::int(ruby, args[2])?,
        rb::int(ruby, args[3])?,
    );
    this(ruby, recv)?.with(|data| data.set(rect));
    Ok(recv)
}

fn initialize_copy(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let rect = rect_arg(ruby, args[0])?;
    this(ruby, recv)?.with(|data| data.set(rect));
    Ok(recv)
}

fn empty(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    this(ruby, recv)?.with(|data| {
        let mut rect = data.value();
        rect.empty();
        data.set(rect);
    });
    Ok(recv)
}

/// The four values in the order of the constructor.
fn parts(rect: Rect) -> [i32; 4] {
    [rect.x, rect.y, rect.width, rect.height]
}

/// The reader of value `I`.
fn part<const I: usize>(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::fix(parts(rect_arg(ruby, recv)?)[I]))
}

/// The writer of value `I`.
fn set_part<const I: usize>(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let value = rb::int(ruby, args[0])?;
    this(ruby, recv)?.with(|data| {
        let mut values = parts(data.value());
        values[I] = value;
        let [x, y, width, height] = values;
        data.set(Rect::new(x, y, width, height));
    });
    Ok(args[0])
}

/// `==`, `===` and `eql?`: equal values. Anything of another class is not equal.
fn equal(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let own = rect_arg(ruby, recv)?;
    let other = wrap::find::<Owned<Rect>>(ruby, args[0]).map(|other| other.get().value());
    Ok(rb::boolean(other == Some(own)))
}

fn to_s(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let [x, y, width, height] = parts(rect_arg(ruby, recv)?);
    rb::string_new(ruby, format!("({x}, {y}, {width}, {height})").as_bytes())
}

/// `_dump(depth)`: the serialized form `Marshal` stores.
fn dump(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    rb::string_new(ruby, &rect_arg(ruby, recv)?.to_bytes())
}

/// `_load(string)`: a new object from the serialized form.
fn load(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let bytes = rb::bytes(ruby, args[0])?;
    let rect =
        Rect::from_bytes(&bytes).map_err(|error| error::to_raise(ruby, Error::Decode(error)))?;
    rect_new(ruby, rect)
}

/// Defines `Rect`.
pub(crate) fn define(ruby: &Ruby) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<Owned<Rect>>(ruby, c"Rect")?;
    rb::method(ruby, class, c"initialize", 4, set)?;
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy)?;
    rb::method(ruby, class, c"set", 4, set)?;
    rb::method(ruby, class, c"empty", 0, empty)?;
    rb::method(ruby, class, c"to_s", 0, to_s)?;
    rb::method(ruby, class, c"_dump", 1, dump)?;
    rb::class_method(ruby, class, c"_load", 1, load)?;
    rb::method(ruby, class, c"x", 0, part::<0>)?;
    rb::method(ruby, class, c"x=", 1, set_part::<0>)?;
    rb::method(ruby, class, c"y", 0, part::<1>)?;
    rb::method(ruby, class, c"y=", 1, set_part::<1>)?;
    rb::method(ruby, class, c"width", 0, part::<2>)?;
    rb::method(ruby, class, c"width=", 1, set_part::<2>)?;
    rb::method(ruby, class, c"height", 0, part::<3>)?;
    rb::method(ruby, class, c"height=", 1, set_part::<3>)?;
    rb::method(ruby, class, c"==", 1, equal)?;
    rb::method(ruby, class, c"===", 1, equal)?;
    rb::method(ruby, class, c"eql?", 1, equal)?;
    Ok(class)
}
