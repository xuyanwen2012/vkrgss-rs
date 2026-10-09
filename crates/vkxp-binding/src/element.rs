//! What the scene classes `Sprite`, `Viewport`, `Window`, `Plane` and `Tilemap` share.
//!
//! An object of one of these classes holds the handle of its element in the guest's scene;
//! the element itself lives there. The Ruby objects an element refers to (its viewport, its
//! bitmaps, its tables) and the objects of its object properties (`color`, `tone`,
//! `src_rect`, `rect`, `cursor_rect`) are kept in instance variables of the element object
//! whose names have no `@`. That keeps them alive as long as the element object is, and a
//! reader returns the variable, which is why reading twice gives the same object.
//!
//! An object property is created with the element as a value object with an owner (see
//! [`Owner`](crate::wrap::Owner)): changing it changes the element. A writer copies the
//! values of its argument into that object.
//!
//! `dispose` removes the element from the scene and leaves the handle in the object, so
//! that every later use fails in the scene with the class's `disposed` message; only
//! `Sprite#bitmap` and `Tilemap#autotiles` go on returning their object. Disposing a viewport
//! disposes the elements created in it in the same way. An element created in a disposed
//! viewport is alive, keeps that `Viewport` object and is never drawn. When the
//! collector frees an element object that was not disposed, its handle goes to a queue, as
//! the bitmap of a collected `Bitmap` does, and the element is disposed the next time the
//! guest is entered.

use std::cell::RefCell;

use vkxp_core::{Color, ElementKind, PlaneId, Rect, Scene, SceneError, SpriteId, TilemapId};
use vkxp_core::{ViewportId, WindowId};
use vkxp_proto::BitmapId;
use vkxp_ruby_sys::ffi::{self, VALUE};
use vkxp_ruby_sys::{HostResult, Raise, Raised, Ruby};

use crate::bitmap::bitmap_arg;
use crate::color::color_arg;
use crate::error::{self, Error};
use crate::guest::{Classes, Guest};
use crate::rb;
use crate::wrap::{self, Wrapped};
use crate::{plane, sprite, tilemap, viewport, window};

/// The instance variable of an element that holds its `Viewport` object, or `nil`.
pub(crate) const VIEWPORT: &std::ffi::CStr = c"viewport";

/// The scene classes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SceneClasses {
    pub(crate) sprite: VALUE,
    pub(crate) viewport: VALUE,
    pub(crate) window: VALUE,
    pub(crate) plane: VALUE,
    pub(crate) tilemap: VALUE,
    pub(crate) autotiles: VALUE,
}

/// Defines the scene classes.
pub(crate) fn define(ruby: &Ruby) -> Result<SceneClasses, Raised> {
    Ok(SceneClasses {
        sprite: sprite::define(ruby)?,
        viewport: viewport::define(ruby)?,
        window: window::define(ruby)?,
        plane: plane::define(ruby)?,
        tilemap: tilemap::define(ruby)?,
        autotiles: tilemap::define_autotiles(ruby)?,
    })
}

/// The handle of one kind of scene element.
pub(crate) trait Kind: Copy + 'static {
    /// The name of the Ruby class, for messages.
    const CLASS_NAME: &'static str;

    /// The kind as the scene's errors name it.
    const ELEMENT: ElementKind;

    /// The Ruby class among the installed ones.
    fn class(classes: &SceneClasses) -> VALUE;

    /// Whether the handle names a live element.
    fn exists(self, scene: &Scene) -> bool;

    /// Removes the element from the scene.
    fn remove(self, guest: &mut Guest);

    /// The handle as an entry of the free queue.
    fn freed(self) -> Freed;
}

macro_rules! kind {
    ($id:ident, $variant:ident, $name:literal, $class:ident, $disposed:ident, |$this:ident, $guest:ident| $remove:expr) => {
        impl Kind for $id {
            const CLASS_NAME: &'static str = $name;
            const ELEMENT: ElementKind = ElementKind::$variant;

            fn class(classes: &SceneClasses) -> VALUE {
                classes.$class
            }

            fn exists(self, scene: &Scene) -> bool {
                !scene.$disposed(self)
            }

            fn remove(self, $guest: &mut Guest) {
                let $this = self;
                $remove
            }

            fn freed(self) -> Freed {
                Freed::$variant(self)
            }
        }
    };
}

kind!(
    SpriteId,
    Sprite,
    "Sprite",
    sprite,
    is_sprite_disposed,
    |id, guest| guest.scene.dispose_sprite(id)
);
kind!(
    ViewportId,
    Viewport,
    "Viewport",
    viewport,
    is_viewport_disposed,
    |id, guest| dispose_viewport(id, guest)
);
kind!(
    WindowId,
    Window,
    "Window",
    window,
    is_window_disposed,
    |id, guest| guest.scene.dispose_window(id, &mut guest.bitmaps)
);
kind!(
    PlaneId,
    Plane,
    "Plane",
    plane,
    is_plane_disposed,
    |id, guest| guest.scene.dispose_plane(id, &mut guest.bitmaps)
);
kind!(
    TilemapId,
    Tilemap,
    "Tilemap",
    tilemap,
    is_tilemap_disposed,
    |id, guest| guest.scene.dispose_tilemap(id, &mut guest.bitmaps)
);

/// Disposes a viewport, and with it the elements that were created in it.
fn dispose_viewport(id: ViewportId, guest: &mut Guest) {
    let _disposed = guest.scene.dispose_viewport(id, &mut guest.bitmaps);
    #[cfg(feature = "testing")]
    for (kind, count) in [
        (ElementKind::Sprite, _disposed.sprites),
        (ElementKind::Window, _disposed.windows),
        (ElementKind::Plane, _disposed.planes),
        (ElementKind::Tilemap, _disposed.tilemaps),
    ] {
        crate::scene_testing::count(kind, -count.cast_signed());
    }
}

/// The element of a collected object, waiting to be disposed.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Freed {
    Sprite(SpriteId),
    Viewport(ViewportId),
    Window(WindowId),
    Plane(PlaneId),
    Tilemap(TilemapId),
}

thread_local! {
    /// Scene elements whose Ruby objects were collected, waiting to be disposed.
    static FREED: RefCell<Vec<Freed>> = const { RefCell::new(Vec::new()) };
}

/// Disposes `id` unless it is disposed already.
fn dispose_in<K: Kind>(id: K, guest: &mut Guest) {
    if id.exists(&guest.scene) {
        id.remove(guest);
        #[cfg(feature = "testing")]
        crate::scene_testing::count(K::ELEMENT, -1);
    }
}

/// Disposes the scene elements of collected objects. Called when the guest is entered.
pub(crate) fn dispose_freed(guest: &mut Guest) {
    let freed = FREED.with(|queue| std::mem::take(&mut *queue.borrow_mut()));
    for element in freed {
        match element {
            Freed::Sprite(id) => dispose_in(id, guest),
            Freed::Viewport(id) => dispose_in(id, guest),
            Freed::Window(id) => dispose_in(id, guest),
            Freed::Plane(id) => dispose_in(id, guest),
            Freed::Tilemap(id) => dispose_in(id, guest),
        }
    }
}

/// The data of an element object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Element<K> {
    /// The element in the scene; `None` before `initialize`. `dispose` leaves the handle,
    /// which then names nothing.
    id: Option<K>,
}

impl<K: Kind> Wrapped for Element<K> {
    const CLASS_NAME: &'static str = K::CLASS_NAME;

    fn class(classes: &Classes) -> VALUE {
        K::class(&classes.scene)
    }

    fn blank() -> Self {
        Element { id: None }
    }

    fn collected(self) {
        if let Some(id) = self.id {
            // After the thread's locals are gone there is no scene left to free from.
            let _ = FREED.try_with(|queue| queue.borrow_mut().push(id.freed()));
        }
    }
}

/// Runs an operation on the guest and turns its failure into the Ruby exception.
pub(crate) fn operate<R>(
    ruby: &Ruby,
    operation: impl FnOnce(&mut Guest) -> Result<R, SceneError>,
) -> Result<R, Raise> {
    Guest::with(operation).map_err(|failure| error::to_raise(ruby, Error::Scene(failure)))
}

/// The `RGSSError` of a disposed element of kind `K`.
fn disposed_error<K: Kind>(ruby: &Ruby) -> Raise {
    error::to_raise(ruby, Error::Scene(SceneError::Disposed(K::ELEMENT)))
}

/// The handle of an element object, which the scene may no longer know. An object that was
/// never initialized counts as disposed.
pub(crate) fn id<K: Kind>(ruby: &Ruby, recv: VALUE) -> Result<K, Raise> {
    wrap::expect::<Element<K>>(ruby, recv)?
        .get()
        .id
        .ok_or_else(|| disposed_error::<K>(ruby))
}

/// The handle of an element object whose element is alive. A disposed one raises
/// `RGSSError`.
pub(crate) fn live<K: Kind>(ruby: &Ruby, recv: VALUE) -> Result<K, Raise> {
    let id = id::<K>(ruby, recv)?;
    if Guest::with(|guest| id.exists(&guest.scene)) {
        Ok(id)
    } else {
        Err(disposed_error::<K>(ruby))
    }
}

/// The optional viewport argument of a constructor: no argument or `nil` for none, or a
/// `Viewport`, which may be disposed. Anything else raises `TypeError`.
pub(crate) fn viewport_arg(ruby: &Ruby, args: &[VALUE]) -> Result<Option<ViewportId>, Raise> {
    rb::check_count(ruby, args, 0, 1)?;
    match args.first() {
        Some(&viewport) if !ffi::nil_p(viewport) => id::<ViewportId>(ruby, viewport).map(Some),
        _ => Ok(None),
    }
}

/// Creates the element of the object `recv` with `create` and stores the `Viewport` object
/// it was created in. An element the object already had is disposed.
pub(crate) fn create<K: Kind>(
    ruby: &Ruby,
    recv: VALUE,
    viewport: Option<&VALUE>,
    create: impl FnOnce(&mut Scene) -> Result<K, SceneError>,
) -> Result<K, Raise> {
    let data = wrap::expect::<Element<K>>(ruby, recv)?;
    let id = operate(ruby, |guest| create(&mut guest.scene))?;
    #[cfg(feature = "testing")]
    crate::scene_testing::count(K::ELEMENT, 1);
    if let Some(previous) = data.with(|data| data.id.replace(id)) {
        Guest::with(|guest| dispose_in(previous, guest));
    }
    let viewport = viewport.copied().unwrap_or(ffi::Qnil);
    rb::ivar_set(ruby, recv, VIEWPORT, viewport)?;
    Ok(id)
}

/// `dispose`: removes the element from the scene. Disposing again does nothing.
pub(crate) fn dispose<K: Kind>(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    if let Some(id) = wrap::expect::<Element<K>>(ruby, recv)?.get().id {
        Guest::with(|guest| dispose_in(id, guest));
    }
    Ok(ffi::Qnil)
}

/// `disposed?`.
pub(crate) fn disposed<K: Kind>(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    let disposed = match wrap::expect::<Element<K>>(ruby, recv)?.get().id {
        Some(id) => Guest::with(|guest| !id.exists(&guest.scene)),
        None => true,
    };
    Ok(rb::boolean(disposed))
}

/// The `TypeError` of an attempt to copy the element `recv`, worded as the original runtime
/// does: `can't clone Sprite`.
fn refuse(ruby: &Ruby, recv: VALUE, copy: &str) -> Raise {
    rb::wrong_type(ruby, recv, |class| format!("can't {copy} {class}"))
}

/// `clone`, which elements refuse.
fn clone(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Err(refuse(ruby, recv, "clone"))
}

/// `dup`, which elements refuse.
fn dup(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Err(refuse(ruby, recv, "dup"))
}

/// `initialize_copy`, for a copy that did not come through `clone` or `dup`.
fn initialize_copy(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    Err(refuse(ruby, recv, "clone"))
}

/// `viewport`: the `Viewport` object the element was created in, or `nil`.
fn viewport_of<K: Kind>(ruby: &Ruby, recv: VALUE, _args: &[VALUE]) -> HostResult {
    live::<K>(ruby, recv)?;
    Ok(rb::ivar_get(ruby, recv, VIEWPORT)?)
}

/// Defines the top-level class `name` of the elements of kind `K` with the methods every
/// element class has: `dispose`, and `clone`, `dup` and `initialize_copy`, which raise.
pub(crate) fn define_class<K: Kind>(ruby: &Ruby, name: &std::ffi::CStr) -> Result<VALUE, Raised> {
    let class = wrap::define_class::<Element<K>>(ruby, name)?;
    rb::method(ruby, class, c"clone", 0, clone)?;
    rb::method(ruby, class, c"dup", 0, dup)?;
    rb::method(ruby, class, c"initialize_copy", 1, initialize_copy)?;
    rb::method(ruby, class, c"dispose", 0, dispose::<K>)?;
    Ok(class)
}

/// Defines `disposed?` and `viewport`, which every element class but `Viewport` has.
pub(crate) fn define_in_viewport<K: Kind>(ruby: &Ruby, class: VALUE) -> Result<(), Raised> {
    rb::method(ruby, class, c"disposed?", 0, disposed::<K>)?;
    rb::method(ruby, class, c"viewport", 0, viewport_of::<K>)
}

/// A bitmap an element is given to draw from: `nil` for none, or a `Bitmap`, whose arena id
/// is `None` as well when it is disposed and so draws nothing. Anything else raises
/// `TypeError`.
pub(crate) fn bitmap_ref(ruby: &Ruby, value: VALUE) -> Result<Option<BitmapId>, Raise> {
    if ffi::nil_p(value) {
        return Ok(None);
    }
    bitmap_arg(ruby, value)
}

/// The arguments of `flash(color, duration)`: `nil` as the colour is the flash without one.
pub(crate) fn flash_args(ruby: &Ruby, args: &[VALUE]) -> Result<(Option<Color>, i32), Raise> {
    let color = if ffi::nil_p(args[0]) {
        None
    } else {
        Some(color_arg(ruby, args[0])?)
    };
    Ok((color, rb::int(ruby, args[1])?))
}

/// A plain property value: what a writer takes and a reader returns.
pub(crate) trait Plain: Sized {
    fn from_arg(ruby: &Ruby, value: VALUE) -> Result<Self, Raise>;

    fn into_value(self, ruby: &Ruby) -> HostResult;
}

impl Plain for i32 {
    fn from_arg(ruby: &Ruby, value: VALUE) -> Result<i32, Raise> {
        rb::int(ruby, value)
    }

    fn into_value(self, _ruby: &Ruby) -> HostResult {
        Ok(rb::fix(self))
    }
}

impl Plain for f64 {
    fn from_arg(ruby: &Ruby, value: VALUE) -> Result<f64, Raise> {
        rb::float(ruby, value)
    }

    fn into_value(self, ruby: &Ruby) -> HostResult {
        rb::float_new(ruby, self)
    }
}

impl Plain for bool {
    fn from_arg(_ruby: &Ruby, value: VALUE) -> Result<bool, Raise> {
        Ok(rb::truthy(value))
    }

    fn into_value(self, _ruby: &Ruby) -> HostResult {
        Ok(rb::boolean(self))
    }
}

/// Four integer arguments as a rectangle.
pub(crate) fn rect_of_ints(ruby: &Ruby, args: &[VALUE]) -> Result<Rect, Raise> {
    Ok(Rect::new(
        rb::int(ruby, args[0])?,
        rb::int(ruby, args[1])?,
        rb::int(ruby, args[2])?,
        rb::int(ruby, args[3])?,
    ))
}

/// Defines the reader `$name` of a plain property of the elements with the handle `$id`:
/// `$read` is the value, computed in the guest from the handle and returned as a
/// [`Plain`]. It may use `?` on results of the scene.
macro_rules! reader {
    ($name:ident, $id:ty, |$guest:ident, $handle:ident| $read:expr) => {
        fn $name(
            ruby: &vkxp_ruby_sys::Ruby,
            recv: vkxp_ruby_sys::ffi::VALUE,
            _args: &[vkxp_ruby_sys::ffi::VALUE],
        ) -> vkxp_ruby_sys::HostResult {
            let $handle = $crate::element::id::<$id>(ruby, recv)?;
            let value = $crate::element::operate(ruby, |$guest| Ok($read))?;
            $crate::element::Plain::into_value(value, ruby)
        }
    };
}

/// Defines the writer `$name` of a plain property of the elements with the handle `$id`:
/// the argument is converted to `$type` as a [`Plain`] and `$write` stores it in the guest.
/// A disposed element raises before the argument is looked at.
macro_rules! writer {
    ($name:ident, $id:ty, |$guest:ident, $handle:ident, $value:ident: $type:ty| $write:expr) => {
        fn $name(
            ruby: &vkxp_ruby_sys::Ruby,
            recv: vkxp_ruby_sys::ffi::VALUE,
            args: &[vkxp_ruby_sys::ffi::VALUE],
        ) -> vkxp_ruby_sys::HostResult {
            let $handle = $crate::element::live::<$id>(ruby, recv)?;
            let $value = <$type as $crate::element::Plain>::from_arg(ruby, args[0])?;
            $crate::element::operate(ruby, |$guest| {
                $write;
                Ok(())
            })?;
            Ok(args[0])
        }
    };
}

/// Defines the reader and the writer of a property that is a public field of the element.
macro_rules! field {
    ($reader:ident, $writer:ident, $id:ty, $get:ident, $get_mut:ident, $field:ident: $type:ty) => {
        $crate::element::reader!($reader, $id, |guest, id| guest.scene.$get(id)?.$field);
        $crate::element::writer!($writer, $id, |guest, id, value: $type| guest
            .scene
            .$get_mut(id)?
            .$field = value);
    };
}

/// Defines the reader `$name` of a Ruby object the element keeps in the instance variable
/// `$variable`: an object property, a bitmap, a table.
macro_rules! kept {
    ($name:ident, $id:ty, $variable:expr) => {
        fn $name(
            ruby: &vkxp_ruby_sys::Ruby,
            recv: vkxp_ruby_sys::ffi::VALUE,
            _args: &[vkxp_ruby_sys::ffi::VALUE],
        ) -> vkxp_ruby_sys::HostResult {
            $crate::element::live::<$id>(ruby, recv)?;
            Ok($crate::rb::ivar_get(ruby, recv, $variable)?)
        }
    };
}

/// Defines the reader `$name` of a Ruby object the element keeps in the instance variable
/// `$variable` and goes on returning when it is disposed, as the original runtime does.
macro_rules! kept_after_dispose {
    ($name:ident, $id:ty, $variable:expr) => {
        fn $name(
            ruby: &vkxp_ruby_sys::Ruby,
            recv: vkxp_ruby_sys::ffi::VALUE,
            _args: &[vkxp_ruby_sys::ffi::VALUE],
        ) -> vkxp_ruby_sys::HostResult {
            $crate::wrap::expect::<$crate::element::Element<$id>>(ruby, recv)?;
            Ok($crate::rb::ivar_get(ruby, recv, $variable)?)
        }
    };
}

/// Defines the reader and the writer of an object property kept in the instance variable
/// `$variable`. The writer takes the value of its argument with `$arg` and copies it into
/// the element's own object with `$set`, which passes it on to the scene.
macro_rules! object {
    ($reader:ident, $writer:ident, $id:ty, $variable:expr, $arg:path, $set:path) => {
        $crate::element::kept!($reader, $id, $variable);

        fn $writer(
            ruby: &vkxp_ruby_sys::Ruby,
            recv: vkxp_ruby_sys::ffi::VALUE,
            args: &[vkxp_ruby_sys::ffi::VALUE],
        ) -> vkxp_ruby_sys::HostResult {
            $crate::element::live::<$id>(ruby, recv)?;
            let value = $arg(ruby, args[0])?;
            let own = $crate::rb::ivar_get(ruby, recv, $variable)?;
            $set(ruby, own, value)?;
            Ok(args[0])
        }
    };
}

pub(crate) use {field, kept, kept_after_dispose, object, reader, writer};
