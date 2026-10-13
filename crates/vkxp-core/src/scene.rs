//! `Scene`: the arena that owns every sprite, viewport, window, plane and tilemap.

pub use vkxp_proto::MAX_LOGICAL_DIMENSION;
use vkxp_proto::{FrameRate, Size};

use crate::tilemap::SentTables;
use crate::{
    Bitmaps, ElementKind, Plane, Rect, SceneError, Sprite, Tilemap, TilemapRgss2, Viewport, Window,
};

/// Names one occupant of one slot of an arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    pub(crate) index: u32,
    generation: u32,
}

/// One slot of an arena. `generation` is that of the current occupant, or of the last one
/// when the slot is empty.
#[derive(Debug, Clone)]
struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// Elements of one kind. `generation` increases every time a slot gets a new occupant, so no
/// two elements of one arena ever share a key.
#[derive(Debug, Clone)]
pub(crate) struct Arena<T> {
    slots: Vec<Slot<T>>,
    /// Empty slots that can take a new occupant.
    free: Vec<u32>,
}

impl<T> Default for Arena<T> {
    fn default() -> Arena<T> {
        Arena {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }
}

impl<T> Arena<T> {
    /// Stores `value`, or returns `None` when every slot is taken or used up.
    fn insert(&mut self, value: T) -> Option<Key> {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            // Slots whose generations are used up never reach the free list.
            slot.generation += 1;
            slot.value = Some(value);
            return Some(Key {
                index,
                generation: slot.generation,
            });
        }
        let index = u32::try_from(self.slots.len()).ok()?;
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        Some(Key {
            index,
            generation: 0,
        })
    }

    pub(crate) fn get(&self, key: Key) -> Option<&T> {
        self.slots
            .get(key.index as usize)
            .filter(|slot| slot.generation == key.generation)
            .and_then(|slot| slot.value.as_ref())
    }

    pub(crate) fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        self.slots
            .get_mut(key.index as usize)
            .filter(|slot| slot.generation == key.generation)
            .and_then(|slot| slot.value.as_mut())
    }

    /// Takes the element out, if the key names one.
    fn remove(&mut self, key: Key) -> Option<T> {
        let slot = self
            .slots
            .get_mut(key.index as usize)
            .filter(|slot| slot.generation == key.generation)?;
        let value = slot.value.take()?;
        // A slot at the last generation is retired, so that no key is ever handed out twice.
        if slot.generation < u32::MAX {
            self.free.push(key.index);
        }
        Some(value)
    }

    /// Takes every element out. The keys handed out so far stay used.
    fn clear(&mut self) {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            if slot.value.take().is_some() && slot.generation < u32::MAX {
                self.free.push(index as u32);
            }
        }
    }

    /// Number of slots, live or empty: one more than the largest index a key can have.
    pub(crate) fn slots(&self) -> usize {
        self.slots.len()
    }

    /// Every live element with its key, in slot order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (Key, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            let key = Key {
                index: index as u32,
                generation: slot.generation,
            };
            slot.value.as_ref().map(|value| (key, value))
        })
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (Key, &mut T)> {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| {
                let key = Key {
                    index: index as u32,
                    generation: slot.generation,
                };
                slot.value.as_mut().map(|value| (key, value))
            })
    }
}

macro_rules! handle {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        ///
        /// A handle is a small copyable name. Once its element is disposed it names nothing,
        /// also after the scene reuses the slot for a later element.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub(crate) Key);
    };
}

handle!(
    /// Names a [`Sprite`] of a [`Scene`].
    SpriteId
);
handle!(
    /// Names a [`Viewport`] of a [`Scene`].
    ViewportId
);
handle!(
    /// Names a [`Window`] of a [`Scene`].
    WindowId
);
handle!(
    /// Names a [`Plane`] of a [`Scene`].
    PlaneId
);
handle!(
    /// Names a [`Tilemap`] of a [`Scene`].
    TilemapId
);

/// The keys of the elements of `arena` that `chosen` holds for.
fn keys_of<T>(arena: &Arena<T>, chosen: impl Fn(&T) -> bool) -> Vec<Key> {
    arena
        .iter()
        .filter(|(_, element)| chosen(element))
        .map(|(key, _)| key)
        .collect()
}

/// How many elements of each kind [`Scene::dispose_viewport`] disposed with the viewport.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DisposedElements {
    pub sprites: usize,
    pub windows: usize,
    pub planes: usize,
    pub tilemaps: usize,
}

/// Everything scripts put on the screen, and the bookkeeping of `Graphics`.
///
/// Elements live in arenas, one per kind, and are named by handles: [`SpriteId`],
/// [`ViewportId`], [`WindowId`], [`PlaneId`] and [`TilemapId`]. A handle whose element was
/// disposed names nothing: reading or changing it fails with [`SceneError::Disposed`], and
/// only disposing it again and asking whether it is disposed still work.
///
/// An element is created with or without a viewport and belongs to it until it is moved to
/// another (`set_sprite_viewport` and the like, which RGSS2 has): disposing the viewport
/// disposes it, and an element created in a disposed viewport is alive and never drawn.
/// Elements are drawn
/// in ascending order of z; among equal z, the element created more recently is in front, and
/// assigning a z never changes the order among equals. A viewport takes one place in that
/// order for all its elements, by its own z and its own creation; its elements are ordered
/// among themselves in the same way.
///
/// Provenance: oracle measurement, RGSS103J, z order probe (`ph-scene-zorder-probe.rb`: 109
/// observations of sprites, planes, windows, viewports and a tilemap, none mismatching).
///
/// [`Scene::extract`] turns the scene into the frame packet a compositor draws.
#[derive(Debug, Clone)]
pub struct Scene {
    pub(crate) sprites: Arena<Sprite>,
    pub(crate) viewports: Arena<Viewport>,
    pub(crate) windows: Arena<Window>,
    pub(crate) planes: Arena<Plane>,
    pub(crate) tilemaps: Arena<Tilemap>,
    /// The tables of the tilemaps that the compositor has.
    pub(crate) tables: SentTables,
    /// Bitmaps of the scene's own whose sprite was disposed; the next extraction disposes
    /// them.
    pub(crate) released: Vec<vkxp_proto::BitmapId>,
    /// The stamp given out last.
    stamp: u64,
    pub(crate) frame_count: i64,
    /// Number of frames extracted so far.
    pub(crate) frame_index: u64,
    pub(crate) frame_rate: FrameRate,
    /// The brightness of the whole screen, from 0 to 255.
    pub(crate) brightness: u8,
    pub(crate) logical_size: Size,
}

/// `size` with each side brought into 1 to [`MAX_LOGICAL_DIMENSION`].
fn limited(size: Size) -> Size {
    Size::new(
        size.width.clamp(1, MAX_LOGICAL_DIMENSION),
        size.height.clamp(1, MAX_LOGICAL_DIMENSION),
    )
}

impl Default for Scene {
    fn default() -> Scene {
        Scene::new()
    }
}

impl Scene {
    /// The logical size of a scene created with [`Scene::new`].
    pub const DEFAULT_SIZE: Size = Size::new(640, 480);

    /// An empty scene of 640 by 480 pixels.
    pub fn new() -> Scene {
        Scene::with_logical_size(Scene::DEFAULT_SIZE)
    }

    /// An empty scene of the given size, each side brought into 1 to
    /// [`MAX_LOGICAL_DIMENSION`].
    pub fn with_logical_size(size: Size) -> Scene {
        Scene {
            sprites: Arena::default(),
            viewports: Arena::default(),
            windows: Arena::default(),
            planes: Arena::default(),
            tilemaps: Arena::default(),
            tables: SentTables::default(),
            released: Vec::new(),
            stamp: 0,
            frame_count: 0,
            frame_index: 0,
            frame_rate: FrameRate::DEFAULT,
            brightness: 255,
            logical_size: limited(size),
        }
    }

    /// The size of the logical screen in pixels.
    pub fn logical_size(&self) -> Size {
        self.logical_size
    }

    /// Sets the size of the logical screen, each side brought into 1 to
    /// [`MAX_LOGICAL_DIMENSION`]; the caller applies the limits of its runtime. Elements keep
    /// their coordinates, and the next extracted packet carries the size.
    pub fn set_logical_size(&mut self, size: Size) {
        self.logical_size = limited(size);
    }

    /// Makes the scene what [`Scene::new`] returns, except that the handles handed out so
    /// far stay used: one of an element from before names nothing, and no new element gets
    /// it. The bitmaps the scene made for its elements stay in their arena, as when a scene
    /// is dropped.
    pub fn clear(&mut self) {
        self.sprites.clear();
        self.viewports.clear();
        self.windows.clear();
        self.planes.clear();
        self.tilemaps.clear();
        *self = Scene {
            sprites: std::mem::take(&mut self.sprites),
            viewports: std::mem::take(&mut self.viewports),
            windows: std::mem::take(&mut self.windows),
            planes: std::mem::take(&mut self.planes),
            tilemaps: std::mem::take(&mut self.tilemaps),
            ..Scene::new()
        };
    }

    /// The next stamp of the draw order.
    fn next_stamp(&mut self) -> u64 {
        self.stamp += 1;
        self.stamp
    }

    /// Creates a sprite with the RGSS1 defaults, in `viewport` or directly on the screen.
    /// In a disposed viewport the sprite is alive and not drawn.
    pub fn create_sprite(&mut self, viewport: Option<ViewportId>) -> Result<SpriteId, SceneError> {
        let stamp = self.next_stamp();
        self.sprites
            .insert(Sprite::new(viewport, stamp))
            .map(SpriteId)
            .ok_or(SceneError::Full(ElementKind::Sprite))
    }

    /// Creates a viewport covering `rect` of the screen.
    pub fn create_viewport(&mut self, rect: Rect) -> Result<ViewportId, SceneError> {
        let stamp = self.next_stamp();
        self.viewports
            .insert(Viewport::new(rect, stamp))
            .map(ViewportId)
            .ok_or(SceneError::Full(ElementKind::Viewport))
    }

    /// Creates a window with the RGSS1 defaults, in `viewport` or directly on the screen.
    /// In a disposed viewport the window is alive and not drawn.
    pub fn create_window(&mut self, viewport: Option<ViewportId>) -> Result<WindowId, SceneError> {
        let stamp = self.next_stamp();
        self.windows
            .insert(Window::new(viewport, stamp))
            .map(WindowId)
            .ok_or(SceneError::Full(ElementKind::Window))
    }

    /// Creates a window of RGSS2 with its defaults and without contents, in `viewport` or
    /// directly on the screen: one element at its z, drawn by the rules of RGSS2, whose
    /// properties are assigned through [`Window::call`]. In a disposed viewport the window
    /// is alive and not drawn.
    pub fn create_window_rgss2(
        &mut self,
        viewport: Option<ViewportId>,
    ) -> Result<WindowId, SceneError> {
        let stamp = self.next_stamp();
        self.windows
            .insert(Window::new_rgss2(viewport, stamp))
            .map(WindowId)
            .ok_or(SceneError::Full(ElementKind::Window))
    }

    /// Creates a plane with the RGSS1 defaults, in `viewport` or directly on the screen.
    /// In a disposed viewport the plane is alive and not drawn.
    pub fn create_plane(&mut self, viewport: Option<ViewportId>) -> Result<PlaneId, SceneError> {
        let stamp = self.next_stamp();
        self.planes
            .insert(Plane::new(viewport, stamp))
            .map(PlaneId)
            .ok_or(SceneError::Full(ElementKind::Plane))
    }

    /// Creates a tilemap with the RGSS1 defaults, in `viewport` or directly on the screen.
    /// In a disposed viewport the tilemap is alive and not drawn.
    pub fn create_tilemap(
        &mut self,
        viewport: Option<ViewportId>,
    ) -> Result<TilemapId, SceneError> {
        let stamp = self.next_stamp();
        self.tilemaps
            .insert(Tilemap::new(viewport, stamp))
            .map(TilemapId)
            .ok_or(SceneError::Full(ElementKind::Tilemap))
    }

    /// Creates a tilemap of RGSS2, in `viewport` or directly on the screen: one that draws
    /// from nine sheets and a passages table. In a disposed viewport the tilemap is alive and
    /// not drawn.
    pub fn create_tilemap_rgss2(
        &mut self,
        viewport: Option<ViewportId>,
    ) -> Result<TilemapId, SceneError> {
        let id = self.create_tilemap(viewport)?;
        let seen = self.viewport_origin(viewport);
        self.tilemap_mut(id)?.rgss2 = Some(TilemapRgss2 {
            seen,
            ..TilemapRgss2::default()
        });
        Ok(id)
    }

    /// The ox and oy of a viewport; 0 without one and for a disposed one.
    fn viewport_origin(&self, viewport: Option<ViewportId>) -> (i32, i32) {
        let viewport = viewport.and_then(|id| self.viewports.get(id.0));
        viewport.map_or((0, 0), |viewport| (viewport.ox, viewport.oy))
    }

    /// Refreshes a tilemap of RGSS2: once it has map data, the ox and oy of its viewport as
    /// they are now are the ones added to its own until the next refresh. A tilemap of RGSS1
    /// has no such state.
    ///
    /// As measured on the original runtime of VX, a tilemap is refreshed by a write of `ox`,
    /// `oy`, `map_data`, `flash_data`, `passages` or a slot of `bitmaps`, by a write of
    /// `visible` that changes it, by `update`, and when the ox or oy of its viewport changes,
    /// but not when it is given another viewport. The caller refreshes after the writes and
    /// the update; for the change of a viewport see [`Scene::notice_viewport_origins`].
    pub fn refresh_tilemap(&mut self, id: TilemapId) -> Result<(), SceneError> {
        let origin = self.viewport_origin(self.tilemap(id)?.viewport);
        let tilemap = self.tilemap_mut(id)?;
        let has_map = tilemap.map_data.is_some();
        if let Some(rgss2) = &mut tilemap.rgss2 {
            rgss2.seen = origin;
            if has_map {
                rgss2.latched = origin;
            }
        }
        Ok(())
    }

    /// Refreshes every tilemap of RGSS2 whose viewport has another ox or oy than when the
    /// scene last looked. The scene looks by itself when a tilemap is moved and when a frame
    /// is extracted; a caller that looks after every write of a viewport's ox or oy makes
    /// each of them count, also one that a later write takes back.
    pub fn notice_viewport_origins(&mut self) {
        let moved = keys_of(&self.tilemaps, |tilemap| {
            let seen = tilemap.rgss2.as_ref().map(|rgss2| rgss2.seen);
            seen.is_some_and(|seen| seen != self.viewport_origin(tilemap.viewport))
        });
        for key in moved {
            // The key was just found.
            let _ = self.refresh_tilemap(TilemapId(key));
        }
    }

    /// Disposes a sprite. Disposing a disposed sprite does nothing. The bitmap the scene
    /// rendered for a turned sprite is disposed by the next [`Scene::extract`].
    pub fn dispose_sprite(&mut self, id: SpriteId) {
        if let Some(sprite) = self.sprites.remove(id.0) {
            self.released.extend(sprite.turned.bitmap);
        }
    }

    /// Disposes a viewport and with it every element that was created in it, and tells how
    /// many of each kind those were. The bitmaps the scene made for those elements live in
    /// `bitmaps`. Disposing a disposed viewport does nothing, also to the elements that were
    /// created in it since.
    pub fn dispose_viewport(&mut self, id: ViewportId, bitmaps: &mut Bitmaps) -> DisposedElements {
        let mut disposed = DisposedElements::default();
        if self.viewports.remove(id.0).is_none() {
            return disposed;
        }
        let inside = Some(id);
        for key in keys_of(&self.sprites, |sprite| sprite.viewport == inside) {
            self.dispose_sprite(SpriteId(key));
            disposed.sprites += 1;
        }
        for key in keys_of(&self.windows, |window| window.viewport == inside) {
            self.dispose_window(WindowId(key), bitmaps);
            disposed.windows += 1;
        }
        for key in keys_of(&self.planes, |plane| plane.viewport == inside) {
            self.dispose_plane(PlaneId(key), bitmaps);
            disposed.planes += 1;
        }
        for key in keys_of(&self.tilemaps, |tilemap| tilemap.viewport == inside) {
            self.dispose_tilemap(TilemapId(key), bitmaps);
            disposed.tilemaps += 1;
        }
        disposed
    }

    /// Disposes a window, and with it the bitmaps the scene drew from its window skin, which
    /// live in `bitmaps`. Disposing a disposed window does nothing.
    pub fn dispose_window(&mut self, id: WindowId, bitmaps: &mut Bitmaps) {
        if let Some(mut window) = self.windows.remove(id.0) {
            window.caches.dispose(bitmaps);
        }
    }

    /// Disposes a plane, and with it the bitmap the scene may have tiled from the plane's,
    /// which lives in `bitmaps`. Disposing a disposed plane does nothing.
    pub fn dispose_plane(&mut self, id: PlaneId, bitmaps: &mut Bitmaps) {
        if let Some(mut plane) = self.planes.remove(id.0) {
            plane.pretiled.dispose(bitmaps);
        }
    }

    /// Disposes a tilemap, and with it the bitmap the scene filled with its flashing cells,
    /// which lives in `bitmaps`. Disposing a disposed tilemap does nothing.
    pub fn dispose_tilemap(&mut self, id: TilemapId, bitmaps: &mut Bitmaps) {
        if let Some(mut tilemap) = self.tilemaps.remove(id.0) {
            tilemap.cache.dispose(bitmaps);
        }
    }
}

/// The access every kind of element has: reading, changing and asking whether it is disposed.
macro_rules! access {
    ($kind:ident, $id:ident, $element:ident, $arena:ident, $get:ident, $get_mut:ident, $disposed:ident) => {
        impl Scene {
            #[doc = concat!("The ", stringify!($get), " `id` names, for reading its properties.")]
            pub fn $get(&self, id: $id) -> Result<&$element, SceneError> {
                self.$arena
                    .get(id.0)
                    .ok_or(SceneError::Disposed(ElementKind::$kind))
            }

            #[doc = concat!("The ", stringify!($get), " `id` names, for changing its properties.")]
            pub fn $get_mut(&mut self, id: $id) -> Result<&mut $element, SceneError> {
                self.$arena
                    .get_mut(id.0)
                    .ok_or(SceneError::Disposed(ElementKind::$kind))
            }

            #[doc = concat!("Whether `id` names no live ", stringify!($get), ".")]
            pub fn $disposed(&self, id: $id) -> bool {
                self.$arena.get(id.0).is_none()
            }
        }
    };
}

access!(
    Sprite,
    SpriteId,
    Sprite,
    sprites,
    sprite,
    sprite_mut,
    is_sprite_disposed
);
access!(
    Viewport,
    ViewportId,
    Viewport,
    viewports,
    viewport,
    viewport_mut,
    is_viewport_disposed
);
access!(
    Window,
    WindowId,
    Window,
    windows,
    window,
    window_mut,
    is_window_disposed
);
access!(
    Plane,
    PlaneId,
    Plane,
    planes,
    plane,
    plane_mut,
    is_plane_disposed
);
access!(
    Tilemap,
    TilemapId,
    Tilemap,
    tilemaps,
    tilemap,
    tilemap_mut,
    is_tilemap_disposed
);

/// Setting z. The place among the elements with the same z is that of creation, whatever
/// is assigned.
macro_rules! set_z {
    ($id:ident, $get_mut:ident, $set_z:ident) => {
        impl Scene {
            /// Sets z. Among the elements that have that z the element keeps the place its
            /// creation gave it.
            pub fn $set_z(&mut self, id: $id, z: i32) -> Result<(), SceneError> {
                self.$get_mut(id)?.order.z = z;
                Ok(())
            }
        }
    };
}

set_z!(SpriteId, sprite_mut, set_sprite_z);
set_z!(ViewportId, viewport_mut, set_viewport_z);
set_z!(WindowId, window_mut, set_window_z);
set_z!(PlaneId, plane_mut, set_plane_z);

/// Moving an element to another viewport, which RGSS2 allows.
macro_rules! set_viewport {
    ($id:ident, $get_mut:ident, $set_viewport:ident) => {
        impl Scene {
            /// Moves the element into `viewport`, or out of any for `None`. It keeps
            /// everything else, including the place its creation gave it among elements of
            /// equal z. A disposed viewport fails with [`SceneError::Disposed`] and moves
            /// nothing.
            ///
            /// Not measured: how the original runtime orders a moved element among elements
            /// of equal z.
            pub fn $set_viewport(
                &mut self,
                id: $id,
                viewport: Option<ViewportId>,
            ) -> Result<(), SceneError> {
                self.$get_mut(id)?;
                if viewport.is_some_and(|viewport| self.viewports.get(viewport.0).is_none()) {
                    return Err(SceneError::Disposed(ElementKind::Viewport));
                }
                self.$get_mut(id)?.viewport = viewport;
                Ok(())
            }
        }
    };
}

set_viewport!(SpriteId, sprite_mut, set_sprite_viewport);
set_viewport!(WindowId, window_mut, set_window_viewport);
set_viewport!(PlaneId, plane_mut, set_plane_viewport);

impl Scene {
    /// Moves the tilemap into `viewport`, or out of any for `None`, as the other elements
    /// are moved. A tilemap of RGSS2 is not refreshed by it: it goes on adding the ox and oy
    /// it took from the viewport it leaves, as measured on the original runtime of VX.
    pub fn set_tilemap_viewport(
        &mut self,
        id: TilemapId,
        viewport: Option<ViewportId>,
    ) -> Result<(), SceneError> {
        self.tilemap_mut(id)?;
        if viewport.is_some_and(|viewport| self.viewports.get(viewport.0).is_none()) {
            return Err(SceneError::Disposed(ElementKind::Viewport));
        }
        // A change of the viewport it leaves is noticed first, and none of the new one is
        // taken for one.
        self.notice_viewport_origins();
        let seen = self.viewport_origin(viewport);
        let tilemap = self.tilemap_mut(id)?;
        tilemap.viewport = viewport;
        if let Some(rgss2) = &mut tilemap.rgss2 {
            rgss2.seen = seen;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
