//! The Ruby classes `Color` and `Tone`.
//!
//! Both are four floating-point components that clamp when set, and differ in the name, the
//! range and the default of the fourth. The host functions are written once, over [`Quad`].
//!
//! A game of RGSS3 gets other functions for `initialize`, `set` and `_load`, as measured on
//! the original runtime of RGSS3: `new` and `set` without arguments give (0, 0, 0, 0), `set`
//! copies one `Color` or `Tone`, a wrong count says the expected count first, and a dump of
//! another length than 32 bytes loads as (0, 0, 0, 0). RGSS1 and RGSS2 keep theirs.

use vkrgss_core::{Color, DecodeError, Tone};
use vkrgss_ruby_sys::ffi::VALUE;
use vkrgss_ruby_sys::{HostResult, Raise, Raised, Ruby};
use vkrgss_vfs::Runtime;

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

    /// The four numbers of a colour in order, as `set(color)` of RGSS3 copies them.
    fn from_color(color: Color) -> Self;

    /// The four numbers of a tone in order, as `set(tone)` of RGSS3 copies them.
    fn from_tone(tone: Tone) -> Self;
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

    fn from_color(color: Color) -> Color {
        color
    }

    fn from_tone(tone: Tone) -> Color {
        Color::from(tone)
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

    fn from_color(color: Color) -> Tone {
        Tone::from(color)
    }

    fn from_tone(tone: Tone) -> Tone {
        tone
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

/// The components of RGSS3 given as three or four numeric arguments; no argument is
/// (0, 0, 0, 0).
fn parts_of_rgss3<Q: Quad>(ruby: &Ruby, args: &[VALUE]) -> Result<[f64; 4], Raise> {
    if args.is_empty() {
        return Ok([0.0; 4]);
    }
    rb::check_count_rgss3(ruby, args, 3, 4)?;
    let mut parts = [0.0, 0.0, 0.0, Q::FOURTH];
    for (part, &arg) in parts.iter_mut().zip(args) {
        *part = rb::float(ruby, arg)?;
    }
    Ok(parts)
}

/// `initialize` of RGSS3: no argument, or three or four numbers. One argument is never an
/// object to copy.
fn initialize_rgss3<Q: Quad>(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let value = Q::from_parts(parts_of_rgss3::<Q>(ruby, args)?);
    this::<Q>(ruby, recv)?.with(|data| data.set(value));
    Ok(recv)
}

/// `set` of RGSS3: the forms of `initialize`, or one `Color` or `Tone` whose four numbers
/// are copied in order. One argument that is neither raises `TypeError`.
fn set_rgss3<Q: Quad>(ruby: &Ruby, recv: VALUE, args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let value = match *args {
        [other] => copied::<Q>(ruby, other)?,
        _ => Q::from_parts(parts_of_rgss3::<Q>(ruby, args)?),
    };
    this::<Q>(ruby, recv)?.with(|data| data.set(value));
    Ok(recv)
}

/// The value of the one argument of `set` of RGSS3. The original copies the bytes of any
/// native object; vkrgss takes a `Color` or a `Tone` and refuses everything else.
fn copied<Q: Quad>(ruby: &Ruby, other: VALUE) -> Result<Q, Raise>
where
    Owned<Q>: Wrapped,
{
    if let Some(color) = wrap::find::<Owned<Color>>(ruby, other) {
        return Ok(Q::from_color(color.get().value()));
    }
    if let Some(tone) = wrap::find::<Owned<Tone>>(ruby, other) {
        return Ok(Q::from_tone(tone.get().value()));
    }
    if !rb::is_data(other) {
        return Err(rb::data_expected(ruby, other));
    }
    Ok(this::<Q>(ruby, other)?.get().value())
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

/// `_load(string)` of RGSS3: as [`load`], and a string of another length than the 32 bytes
/// of a serialized form gives (0, 0, 0, 0), as on the original runtime of RGSS3.
fn load_rgss3<Q: Quad>(ruby: &Ruby, _class: VALUE, args: &[VALUE]) -> HostResult
where
    Owned<Q>: Wrapped,
{
    let bytes = rb::bytes(ruby, args[0])?;
    let value = if bytes.len() == Q::BLANK.to_bytes().len() {
        Q::from_bytes(&bytes).map_err(|error| error::to_raise(ruby, Error::Decode(error)))?
    } else {
        Q::from_parts([0.0; 4])
    };
    wrap::new_object(ruby, Owned::new(value))
}

/// Defines the methods `Color` and `Tone` share; the caller adds the fourth component.
fn define_shared<Q: Quad>(ruby: &Ruby, class: VALUE, runtime: Runtime) -> Result<(), Raised>
where
    Owned<Q>: Wrapped,
{
    // The order of the definitions is that of the names in the symbol table of RGSS1.
    let rgss3 = runtime == Runtime::Rgss3;
    if rgss3 {
        rb::method(ruby, class, c"initialize", -1, initialize_rgss3::<Q>)?;
    } else {
        rb::method(ruby, class, c"initialize", -1, set::<Q>)?;
    }
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy::<Q>)?;
    if rgss3 {
        rb::method(ruby, class, c"set", -1, set_rgss3::<Q>)?;
    } else {
        rb::method(ruby, class, c"set", -1, set_again::<Q>)?;
    }
    rb::method(ruby, class, c"to_s", 0, to_s::<Q>)?;
    rb::method(ruby, class, c"_dump", 1, dump::<Q>)?;
    if rgss3 {
        rb::class_method(ruby, class, c"_load", 1, load_rgss3::<Q>)?;
    } else {
        rb::class_method(ruby, class, c"_load", 1, load::<Q>)?;
    }
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

/// Defines `Color` as a game of `runtime` has it.
pub(crate) fn define_color(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<Owned<Color>>(ruby, c"Color")?;
    define_shared::<Color>(ruby, class, runtime)?;
    rb::method(ruby, class, c"alpha", 0, component::<Color, 3>)?;
    rb::method(ruby, class, c"alpha=", 1, set_component::<Color, 3>)?;
    Ok(class)
}

/// Defines `Tone` as a game of `runtime` has it.
pub(crate) fn define_tone(ruby: &Ruby, runtime: Runtime) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<Owned<Tone>>(ruby, c"Tone")?;
    define_shared::<Tone>(ruby, class, runtime)?;
    rb::method(ruby, class, c"gray", 0, component::<Tone, 3>)?;
    rb::method(ruby, class, c"gray=", 1, set_component::<Tone, 3>)?;
    Ok(class)
}
