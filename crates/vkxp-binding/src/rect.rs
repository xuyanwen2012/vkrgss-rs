//! The Ruby class `Rect`.

use vkxp_core::Rect;
use vkxp_ruby_sys::ffi::VALUE;
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkxp_vfs::Runtime;

use crate::error::{self, Error};
use crate::guest::Classes;
use crate::rb;
use crate::wrap::{self, Handle, Owned, Owner, Wrapped};

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

/// A new `Rect` object that is the property `owner` names.
pub(crate) fn rect_of(ruby: &Ruby, rect: Rect, owner: Owner) -> HostResult {
    wrap::new_object(ruby, Owned::of(rect, owner))
}

/// Replaces the value of a `Rect` object.
pub(crate) fn rect_set(ruby: &Ruby, object: VALUE, rect: Rect) -> Result<(), Raise> {
    this(ruby, object)?.with(|data| data.set(rect));
    Ok(())
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

/// `set(x, y, width, height)`.
fn set_again(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    wrap::intern_owner_names(ruby)?;
    set(ruby, recv, args)
}

/// The four Integers of `new(x, y, width, height)` of RGSS3.
fn rect_of_args(ruby: &Ruby, args: &[VALUE]) -> Result<Rect, Raise> {
    rb::check_count_rgss3(ruby, args, 4, 4)?;
    Ok(Rect::new(
        rb::int(ruby, args[0])?,
        rb::int(ruby, args[1])?,
        rb::int(ruby, args[2])?,
        rb::int(ruby, args[3])?,
    ))
}

/// `initialize` of RGSS3: no argument, (0, 0, 0, 0), or four Integers. One argument is never
/// an object to copy.
fn initialize_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let rect = if args.is_empty() {
        Rect::new(0, 0, 0, 0)
    } else {
        rect_of_args(ruby, args)?
    };
    this(ruby, recv)?.with(|data| data.set(rect));
    Ok(recv)
}

/// `set` of RGSS3: four Integers, or one `Rect` whose values are copied. The original copies
/// the bytes of any native object; vkxp refuses everything but a `Rect`.
fn set_rgss3(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let rect = match *args {
        [other] if !rb::is_data(other) => return Err(rb::data_expected(ruby, other)),
        [other] => rect_arg(ruby, other)?,
        _ => rect_of_args(ruby, args)?,
    };
    this(ruby, recv)?.with(|data| data.set(rect));
    Ok(recv)
}

fn initialize_copy(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult {
    let rect = rect_arg(ruby, args[0])?;
    this(ruby, recv)?.with(|data| data.set(rect));
    Ok(recv)
}

fn empty(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    wrap::intern_owner_names(ruby)?;
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
    wrap::intern_owner_names(ruby)?;
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
    Ok(rb::bytes_new(
        ruby,
        format!("({x}, {y}, {width}, {height})").as_bytes(),
    )?)
}

/// `_dump(depth)`: the serialized form `Marshal` stores.
fn dump(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Ok(rb::bytes_new(ruby, &rect_arg(ruby, recv)?.to_bytes())?)
}

/// `_load(string)`: a new object from the serialized form.
fn load(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult {
    let bytes = rb::bytes(ruby, args[0])?;
    let rect =
        Rect::from_bytes(&bytes).map_err(|error| error::to_raise(ruby, Error::Decode(error)))?;
    rect_new(ruby, rect)
}

/// Defines `Rect` as a game of `runtime` has it.
pub(crate) fn define(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<Owned<Rect>>(ruby, c"Rect")?;
    // The order of the definitions is that of the names in the symbol table of RGSS1.
    let rgss3 = runtime == Runtime::Rgss3;
    if rgss3 {
        rb::method(ruby, class, c"initialize", -1, initialize_rgss3)?;
    } else {
        rb::method(ruby, class, c"initialize", 4, set)?;
    }
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy)?;
    if rgss3 {
        rb::method(ruby, class, c"set", -1, set_rgss3)?;
    } else {
        rb::method(ruby, class, c"set", 4, set_again)?;
    }
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
