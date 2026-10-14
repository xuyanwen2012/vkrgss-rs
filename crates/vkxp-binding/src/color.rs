//! The Ruby classes `Color` and `Tone`.
//!
//! Both are four floating-point components that clamp when set, and differ in the name, the
//! range and the default of the fourth. The host functions are written once, over [`Quad`].

use vkxp_core::{Color, DecodeError, Tone};
use vkxp_ruby_sys::ffi::VALUE;
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};

use crate::error::{self, Error};
use crate::guest::Classes;
use crate::rb;
use crate::wrap::{self, Handle, Owned, Owner, Property, Wrapped};

/// A value of four components: `Color` or `Tone`.
pub(crate) trait Quad: Property + PartialEq + 'static {
    /// The fourth component when a script gives three.
    const FOURTH: f64;

    /// The value an allocated object has before `initialize`.
    const BLANK: Self;

    /// A value from four components, each clamped to its range.
    fn from_parts(parts: [f64; 4]) -> Self;

    fn parts(&self) -> [f64; 4];

    fn to_bytes(&self) -> [u8; 32];

    fn from_bytes(bytes: &[u8]) -> Result<Self, DecodeError>;
}

impl Quad for Color {
    const FOURTH: f64 = 255.0;
    const BLANK: Color = Color::TRANSPARENT;

    fn from_parts([red, green, blue, alpha]: [f64; 4]) -> Color {
        Color::new(red, green, blue, alpha)
    }

    fn parts(&self) -> [f64; 4] {
        [self.red(), self.green(), self.blue(), self.alpha()]
    }

    fn to_bytes(&self) -> [u8; 32] {
        Color::to_bytes(self)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Color, DecodeError> {
        Color::from_bytes(bytes)
    }
}

impl Quad for Tone {
    const FOURTH: f64 = 0.0;
    const BLANK: Tone = Tone::NEUTRAL;

    fn from_parts([red, green, blue, gray]: [f64; 4]) -> Tone {
        Tone::new(red, green, blue, gray)
    }

    fn parts(&self) -> [f64; 4] {
        [self.red(), self.green(), self.blue(), self.gray()]
    }

    fn to_bytes(&self) -> [u8; 32] {
        Tone::to_bytes(self)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Tone, DecodeError> {
        Tone::from_bytes(bytes)
    }
}

impl Wrapped for Owned<Color> {
    const CLASS_NAME: &'static str = "Color";

    fn class(classes: &Classes) -> VALUE {
        classes.color
    }

    fn blank() -> Self {
        Owned::new(Color::BLANK)
    }
}

impl Wrapped for Owned<Tone> {
    const CLASS_NAME: &'static str = "Tone";

    fn class(classes: &Classes) -> VALUE {
        classes.tone
    }

    fn blank() -> Self {
        Owned::new(Tone::BLANK)
    }
}

/// The value of a `Color` argument. Anything else raises `TypeError`.
pub(crate) fn color_arg(ruby: &Ruby, value: VALUE) -> Result<Color, Raise> {
    Ok(wrap::expect::<Owned<Color>>(ruby, value)?.get().value())
}

/// A new `Color` object.
pub(crate) fn color_new(ruby: &Ruby, color: Color) -> HostResult {
    wrap::new_object(ruby, Owned::new(color))
}

/// Replaces the value of a `Color` object.
pub(crate) fn color_set(ruby: &Ruby, object: VALUE, color: Color) -> Result<(), Raise> {
    wrap::expect::<Owned<Color>>(ruby, object)?.with(|data| data.set(color));
    Ok(())
}

/// The value of a `Tone` argument. Anything else raises `TypeError`.
pub(crate) fn tone_arg(ruby: &Ruby, value: VALUE) -> Result<Tone, Raise> {
    Ok(wrap::expect::<Owned<Tone>>(ruby, value)?.get().value())
}

/// Replaces the value of a `Tone` object.
pub(crate) fn tone_set(ruby: &Ruby, object: VALUE, tone: Tone) -> Result<(), Raise> {
    wrap::expect::<Owned<Tone>>(ruby, object)?.with(|data| data.set(tone));
    Ok(())
}

/// A new `Color` object that is the property `owner` names.
pub(crate) fn color_of(ruby: &Ruby, color: Color, owner: Owner) -> HostResult {
    wrap::new_object(ruby, Owned::of(color, owner))
}

/// A new `Tone` object that is the property `owner` names.
pub(crate) fn tone_of(ruby: &Ruby, tone: Tone, owner: Owner) -> HostResult {
    wrap::new_object(ruby, Owned::of(tone, owner))
}

fn this<Q: Quad>(ruby: &Ruby, recv: VALUE) -> Result<Handle<Owned<Q>>, Raise>
where
    Owned<Q>: Wrapped,
{
    wrap::expect(ruby, recv)
}

/// The components given as three or four numeric arguments.
fn parts_of<Q: Quad>(ruby: &Ruby, args: &[VALUE]) -> Result<[f64; 4], Raise> {
    rb::check_count(ruby, args, 3, 4)?;
    let mut parts = [0.0, 0.0, 0.0, Q::FOURTH];
    for (part, &arg) in parts.iter_mut().zip(args) {
        *part = rb::float(ruby, arg)?;
    }
    Ok(parts)
}

/// `set(red, green, blue, fourth = default)`.
fn set_again<Q: Quad>(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    wrap::intern_owner_names(ruby)?;
    set::<Q>(ruby, recv, args)
}

/// `initialize(red, green, blue, fourth = default)`, and `set` after its first step.
fn set<Q: Quad>(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let parts = parts_of::<Q>(ruby, args)?;
    this::<Q>(ruby, recv)?.with(|data| data.set(Q::from_parts(parts)));
    Ok(recv)
}

fn initialize_copy<Q: Quad>(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let value = this::<Q>(ruby, args[0])?.get().value();
    this::<Q>(ruby, recv)?.with(|data| data.set(value));
    Ok(recv)
}

/// The reader of component `I`.
fn component<Q: Quad, const I: usize>(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let parts = this::<Q>(ruby, recv)?.get().value().parts();
    rb::float_new(ruby, parts[I])
}

/// The writer of component `I`.
fn set_component<Q: Quad, const I: usize>(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    wrap::intern_owner_names(ruby)?;
    let part = rb::float(ruby, args[0])?;
    this::<Q>(ruby, recv)?.with(|data| {
        let mut parts = data.value().parts();
        parts[I] = part;
        data.set(Q::from_parts(parts));
    });
    Ok(args[0])
}

/// `==`, `===` and `eql?`: equal components. Anything of another class is not equal.
fn equal<Q: Quad>(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let own = this::<Q>(ruby, recv)?.get().value();
    let other = wrap::find::<Owned<Q>>(ruby, args[0]).map(|other| other.get().value());
    Ok(rb::boolean(other == Some(own)))
}

fn to_s<Q: Quad>(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let [first, second, third, fourth] = this::<Q>(ruby, recv)?.get().value().parts();
    let text = format!("({first:.6}, {second:.6}, {third:.6}, {fourth:.6})");
    Ok(rb::bytes_new(ruby, text.as_bytes())?)
}

/// `_dump(depth)`: the serialized form `Marshal` stores.
fn dump<Q: Quad>(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let bytes = this::<Q>(ruby, recv)?.get().value().to_bytes();
    Ok(rb::bytes_new(ruby, &bytes)?)
}

/// `_load(string)`: a new object from the serialized form.
fn load<Q: Quad>(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let bytes = rb::bytes(ruby, args[0])?;
    let value =
        Q::from_bytes(&bytes).map_err(|error| error::to_raise(ruby, Error::Decode(error)))?;
    wrap::new_object(ruby, Owned::new(value))
}

/// Defines the methods `Color` and `Tone` share; the caller adds the fourth component.
fn define_shared<Q: Quad>(ruby: &Ruby, class: VALUE) -> Result<(), Raised>
where
    Owned<Q>: Wrapped,
{
    rb::method(ruby, class, c"initialize", -1, set::<Q>)?;
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy::<Q>)?;
    rb::method(ruby, class, c"set", -1, set_again::<Q>)?;
    rb::method(ruby, class, c"to_s", 0, to_s::<Q>)?;
    rb::method(ruby, class, c"_dump", 1, dump::<Q>)?;
    rb::class_method(ruby, class, c"_load", 1, load::<Q>)?;
    rb::method(ruby, class, c"red", 0, component::<Q, 0>)?;
    rb::method(ruby, class, c"red=", 1, set_component::<Q, 0>)?;
    rb::method(ruby, class, c"green", 0, component::<Q, 1>)?;
    rb::method(ruby, class, c"green=", 1, set_component::<Q, 1>)?;
    rb::method(ruby, class, c"blue", 0, component::<Q, 2>)?;
    rb::method(ruby, class, c"blue=", 1, set_component::<Q, 2>)?;
    rb::method(ruby, class, c"==", 1, equal::<Q>)?;
    rb::method(ruby, class, c"===", 1, equal::<Q>)?;
    rb::method(ruby, class, c"eql?", 1, equal::<Q>)?;
    Ok(())
}

/// Defines `Color`.
pub(crate) fn define_color(ruby: &Ruby) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<Owned<Color>>(ruby, c"Color")?;
    define_shared::<Color>(ruby, class)?;
    rb::method(ruby, class, c"alpha", 0, component::<Color, 3>)?;
    rb::method(ruby, class, c"alpha=", 1, set_component::<Color, 3>)?;
    Ok(class)
}

/// Defines `Tone`.
pub(crate) fn define_tone(ruby: &Ruby) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<Owned<Tone>>(ruby, c"Tone")?;
    define_shared::<Tone>(ruby, class)?;
    rb::method(ruby, class, c"gray", 0, component::<Tone, 3>)?;
    rb::method(ruby, class, c"gray=", 1, set_component::<Tone, 3>)?;
    Ok(class)
}
