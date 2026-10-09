//! `Scene`: the arena that owns every sprite, viewport, window, plane and tilemap.

use vkxp_proto::{FrameRate, Size};

use crate::element::Order;
use crate::{Bitmaps, ElementKind, Plane, Rect, SceneError, Sprite, Tilemap, Viewport, Window};

/// Largest logical width or height of a scene, the limit of the compositors.
pub const MAX_LOGICAL_DIMENSION: u32 = 16384;

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
/// An element is created with or without a viewport and keeps it for life: disposing the
/// viewport disposes it, and an element created in a disposed viewport is alive and never
/// drawn. Elements are drawn
/// in ascending order of z; among equal z, the element created, or last given a different z,
/// more recently is in front. A viewport takes one place in that order for all its elements,
/// which are ordered among themselves in the same way.
///
/// PLACEHOLDER: the order among elements with equal z, in particular after a z change, is not
/// yet checked against the original runtime.
///
/// [`Scene::extract`] turns the scene into the frame packet a compositor draws.
#[derive(Debug, Clone)]
pub struct Scene {
    pub(crate) sprites: Arena<Sprite>,
    pub(crate) viewports: Arena<Viewport>,
    pub(crate) windows: Arena<Window>,
    pub(crate) planes: Arena<Plane>,
    pub(crate) tilemaps: Arena<Tilemap>,
    /// Bitmaps of the scene's own whose sprite was disposed; the next extraction disposes
    /// them.
    pub(crate) released: Vec<vkxp_proto::BitmapId>,
    /// The stamp given out last.
    stamp: u64,
    pub(crate) frame_count: i64,
    /// Number of frames extracted so far.
    pub(crate) frame_index: u64,
    pub(crate) frame_rate: FrameRate,
    pub(crate) logical_size: Size,
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
            released: Vec::new(),
            stamp: 0,
            frame_count: 0,
            frame_index: 0,
            frame_rate: FrameRate::DEFAULT,
            logical_size: Size::new(
                size.width.clamp(1, MAX_LOGICAL_DIMENSION),
                size.height.clamp(1, MAX_LOGICAL_DIMENSION),
            ),
        }
    }

    /// The size of the logical screen in pixels.
    pub fn logical_size(&self) -> Size {
        self.logical_size
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

    /// Disposes a tilemap, and with it the bitmaps the scene composed its tiles into, which
    /// live in `bitmaps`. Disposing a disposed tilemap does nothing.
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

/// Setting z: a value different from the current one moves the element in front of every
/// other element with that z; the current value changes nothing.
macro_rules! set_z {
    ($id:ident, $get_mut:ident, $set_z:ident) => {
        impl Scene {
            /// Sets z. A value other than the current one also puts the element in front of
            /// the others that have that z; the current value changes nothing.
            pub fn $set_z(&mut self, id: $id, z: i32) -> Result<(), SceneError> {
                if self.$get_mut(id)?.order.z == z {
                    return Ok(());
                }
                let stamp = self.next_stamp();
                self.$get_mut(id)?.order = Order { z, stamp };
                Ok(())
            }
        }
    };
}

set_z!(SpriteId, sprite_mut, set_sprite_z);
set_z!(ViewportId, viewport_mut, set_viewport_z);
set_z!(WindowId, window_mut, set_window_z);
set_z!(PlaneId, plane_mut, set_plane_z);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, SharedTable, Table, Tone};

    const DISPOSED_SPRITE: SceneError = SceneError::Disposed(ElementKind::Sprite);

    #[test]
    fn a_disposed_sprite_fails_every_operation_but_disposal() {
        let mut scene = Scene::new();
        let sprite = scene.create_sprite(None).unwrap();
        assert!(!scene.is_sprite_disposed(sprite));
        scene.dispose_sprite(sprite);
        assert!(scene.is_sprite_disposed(sprite));
        assert_eq!(scene.sprite(sprite).err(), Some(DISPOSED_SPRITE));
        assert_eq!(
            scene.sprite_mut(sprite).map(|s| s.x = 5).err(),
            Some(DISPOSED_SPRITE)
        );
        assert_eq!(scene.set_sprite_z(sprite, 3), Err(DISPOSED_SPRITE));
        scene.dispose_sprite(sprite);
        assert!(scene.is_sprite_disposed(sprite));
        assert_eq!(DISPOSED_SPRITE.to_string(), "disposed sprite");
    }

    #[test]
    fn a_disposed_viewport_fails_every_operation_but_disposal() {
        let error = SceneError::Disposed(ElementKind::Viewport);
        let mut scene = Scene::new();
        let viewport = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
        scene.dispose_viewport(viewport, &mut Bitmaps::new());
        assert!(scene.is_viewport_disposed(viewport));
        assert_eq!(scene.viewport(viewport).err(), Some(error));
        assert_eq!(
            scene.viewport_mut(viewport).map(|v| v.ox = 1).err(),
            Some(error)
        );
        assert_eq!(scene.set_viewport_z(viewport, 3), Err(error));
        scene.dispose_viewport(viewport, &mut Bitmaps::new());
        assert_eq!(error.to_string(), "disposed viewport");
    }

    #[test]
    fn a_disposed_window_fails_every_operation_but_disposal() {
        let error = SceneError::Disposed(ElementKind::Window);
        let mut scene = Scene::new();
        let mut bitmaps = Bitmaps::new();
        let window = scene.create_window(None).unwrap();
        scene.dispose_window(window, &mut bitmaps);
        assert!(scene.is_window_disposed(window));
        assert_eq!(scene.window(window).err(), Some(error));
        assert_eq!(scene.window_mut(window).map(|w| w.x = 1).err(), Some(error));
        assert_eq!(scene.set_window_z(window, 3), Err(error));
        scene.dispose_window(window, &mut bitmaps);
        assert_eq!(error.to_string(), "disposed window");
    }

    #[test]
    fn a_disposed_plane_fails_every_operation_but_disposal() {
        let error = SceneError::Disposed(ElementKind::Plane);
        let mut scene = Scene::new();
        let mut bitmaps = Bitmaps::new();
        let plane = scene.create_plane(None).unwrap();
        scene.dispose_plane(plane, &mut bitmaps);
        assert!(scene.is_plane_disposed(plane));
        assert_eq!(scene.plane(plane).err(), Some(error));
        assert_eq!(scene.plane_mut(plane).map(|p| p.ox = 1).err(), Some(error));
        assert_eq!(scene.set_plane_z(plane, 3), Err(error));
        scene.dispose_plane(plane, &mut bitmaps);
        assert_eq!(error.to_string(), "disposed plane");
    }

    #[test]
    fn a_disposed_tilemap_fails_every_operation_but_disposal() {
        let error = SceneError::Disposed(ElementKind::Tilemap);
        let mut scene = Scene::new();
        let mut bitmaps = Bitmaps::new();
        let tilemap = scene.create_tilemap(None).unwrap();
        scene.dispose_tilemap(tilemap, &mut bitmaps);
        assert!(scene.is_tilemap_disposed(tilemap));
        assert_eq!(scene.tilemap(tilemap).err(), Some(error));
        assert_eq!(
            scene.tilemap_mut(tilemap).map(|t| t.ox = 1).err(),
            Some(error)
        );
        scene.dispose_tilemap(tilemap, &mut bitmaps);
        assert_eq!(error.to_string(), "disposed tilemap");
    }

    #[test]
    fn a_stale_handle_stays_disposed_when_its_slot_is_reused() {
        let mut scene = Scene::new();
        let old = scene.create_sprite(None).unwrap();
        scene.dispose_sprite(old);
        let new = scene.create_sprite(None).unwrap();
        assert_eq!(new.0.index, old.0.index, "the slot is reused");
        assert_ne!(new, old);
        assert!(scene.is_sprite_disposed(old));
        assert!(!scene.is_sprite_disposed(new));
        assert_eq!(
            scene.sprite_mut(old).map(|s| s.x = 9).err(),
            Some(DISPOSED_SPRITE)
        );
        assert_eq!(scene.sprite(new).unwrap().x, 0);
        // Disposing through the stale handle leaves the new occupant alone.
        scene.dispose_sprite(old);
        assert!(!scene.is_sprite_disposed(new));

        let old = scene.create_viewport(Rect::new(0, 0, 1, 1)).unwrap();
        scene.dispose_viewport(old, &mut Bitmaps::new());
        let new = scene.create_viewport(Rect::new(0, 0, 2, 2)).unwrap();
        assert!(scene.is_viewport_disposed(old));
        assert_eq!(scene.viewport(new).unwrap().rect, Rect::new(0, 0, 2, 2));

        let old = scene.create_window(None).unwrap();
        scene.dispose_window(old, &mut Bitmaps::new());
        let new = scene.create_window(None).unwrap();
        assert!(scene.is_window_disposed(old) && !scene.is_window_disposed(new));

        let old = scene.create_plane(None).unwrap();
        scene.dispose_plane(old, &mut Bitmaps::new());
        let new = scene.create_plane(None).unwrap();
        assert!(scene.is_plane_disposed(old) && !scene.is_plane_disposed(new));

        let old = scene.create_tilemap(None).unwrap();
        scene.dispose_tilemap(old, &mut Bitmaps::new());
        let new = scene.create_tilemap(None).unwrap();
        assert!(scene.is_tilemap_disposed(old) && !scene.is_tilemap_disposed(new));
    }

    #[test]
    fn elements_are_created_with_or_without_a_viewport() {
        let mut scene = Scene::new();
        let viewport = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
        let inside = Some(viewport);

        let sprite = scene.create_sprite(inside).unwrap();
        assert_eq!(scene.sprite(sprite).unwrap().viewport(), inside);
        let window = scene.create_window(inside).unwrap();
        assert_eq!(scene.window(window).unwrap().viewport(), inside);
        let plane = scene.create_plane(inside).unwrap();
        assert_eq!(scene.plane(plane).unwrap().viewport(), inside);
        let tilemap = scene.create_tilemap(inside).unwrap();
        assert_eq!(scene.tilemap(tilemap).unwrap().viewport(), inside);

        let sprite = scene.create_sprite(None).unwrap();
        assert_eq!(scene.sprite(sprite).unwrap().viewport(), None);
    }

    #[test]
    fn a_viewport_takes_its_elements_along() {
        let mut scene = Scene::new();
        let mut bitmaps = Bitmaps::new();
        let viewport = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
        let other = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
        let sprite = scene.create_sprite(Some(viewport)).unwrap();
        let window = scene.create_window(Some(viewport)).unwrap();
        let plane = scene.create_plane(Some(viewport)).unwrap();
        let tilemap = scene.create_tilemap(Some(viewport)).unwrap();
        let outside = (
            scene.create_sprite(None).unwrap(),
            scene.create_window(None).unwrap(),
            scene.create_plane(None).unwrap(),
            scene.create_tilemap(None).unwrap(),
        );
        let elsewhere = (
            scene.create_sprite(Some(other)).unwrap(),
            scene.create_window(Some(other)).unwrap(),
            scene.create_plane(Some(other)).unwrap(),
            scene.create_tilemap(Some(other)).unwrap(),
        );

        let disposed = scene.dispose_viewport(viewport, &mut bitmaps);
        assert_eq!(
            disposed,
            DisposedElements {
                sprites: 1,
                windows: 1,
                planes: 1,
                tilemaps: 1,
            }
        );
        assert!(scene.is_sprite_disposed(sprite));
        assert!(scene.is_window_disposed(window));
        assert!(scene.is_plane_disposed(plane));
        assert!(scene.is_tilemap_disposed(tilemap));
        assert_eq!(scene.sprite(sprite).err(), Some(DISPOSED_SPRITE));
        // Disposing them again does nothing.
        scene.dispose_sprite(sprite);
        scene.dispose_window(window, &mut bitmaps);
        scene.dispose_plane(plane, &mut bitmaps);
        scene.dispose_tilemap(tilemap, &mut bitmaps);

        assert!(!scene.is_viewport_disposed(other));
        for (sprite, window, plane, tilemap) in [outside, elsewhere] {
            assert!(!scene.is_sprite_disposed(sprite));
            assert!(!scene.is_window_disposed(window));
            assert!(!scene.is_plane_disposed(plane));
            assert!(!scene.is_tilemap_disposed(tilemap));
        }
    }

    #[test]
    fn an_element_created_in_a_disposed_viewport_is_alive() {
        let mut scene = Scene::new();
        let mut bitmaps = Bitmaps::new();
        let viewport = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
        scene.dispose_viewport(viewport, &mut bitmaps);
        let inside = Some(viewport);

        let sprite = scene.create_sprite(inside).unwrap();
        let window = scene.create_window(inside).unwrap();
        let plane = scene.create_plane(inside).unwrap();
        let tilemap = scene.create_tilemap(inside).unwrap();
        scene.sprite_mut(sprite).unwrap().x = 5;
        assert_eq!(scene.sprite(sprite).unwrap().x, 5);
        assert_eq!(scene.sprite(sprite).unwrap().viewport(), inside);
        assert_eq!(scene.window(window).unwrap().viewport(), inside);
        assert_eq!(scene.plane(plane).unwrap().viewport(), inside);
        assert_eq!(scene.tilemap(tilemap).unwrap().viewport(), inside);

        // Disposing the viewport again, or one that took its slot, leaves them alone.
        assert_eq!(
            scene.dispose_viewport(viewport, &mut bitmaps),
            DisposedElements::default()
        );
        let reused = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
        assert_eq!(reused.0.index, viewport.0.index, "the slot is reused");
        assert_eq!(
            scene.dispose_viewport(reused, &mut bitmaps),
            DisposedElements::default()
        );
        assert!(!scene.is_sprite_disposed(sprite));
        assert!(!scene.is_window_disposed(window));
        assert!(!scene.is_plane_disposed(plane));
        assert!(!scene.is_tilemap_disposed(tilemap));
    }

    #[test]
    fn a_sprite_starts_with_the_rgss1_defaults() {
        let mut scene = Scene::new();
        let id = scene.create_sprite(None).unwrap();
        let sprite = scene.sprite(id).unwrap();
        assert!(sprite.visible);
        assert_eq!((sprite.x, sprite.y, sprite.z()), (0, 0, 0));
        assert_eq!((sprite.ox, sprite.oy), (0, 0));
        assert_eq!((sprite.zoom_x, sprite.zoom_y), (1.0, 1.0));
        assert_eq!(sprite.angle, 0.0);
        assert!(!sprite.mirror);
        assert_eq!(sprite.bush_depth, 0);
        assert_eq!(sprite.opacity(), 255);
        assert_eq!(sprite.blend_type, 0);
        assert_eq!(sprite.color, Color::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(sprite.tone, Tone::NEUTRAL);
        assert_eq!(sprite.bitmap(), None);
        assert_eq!(sprite.src_rect, Rect::new(0, 0, 0, 0));
    }

    #[test]
    fn a_viewport_starts_with_the_rgss1_defaults() {
        let mut scene = Scene::new();
        let id = scene.create_viewport(Rect::new(1, 2, 3, 4)).unwrap();
        let viewport = scene.viewport(id).unwrap();
        assert_eq!(viewport.rect, Rect::new(1, 2, 3, 4));
        assert!(viewport.visible);
        assert_eq!((viewport.ox, viewport.oy, viewport.z()), (0, 0, 0));
        assert_eq!(viewport.color, Color::new(0.0, 0.0, 0.0, 0.0));
        assert_eq!(viewport.tone, Tone::NEUTRAL);
    }

    #[test]
    fn a_window_starts_with_the_rgss1_defaults() {
        let mut scene = Scene::new();
        let id = scene.create_window(None).unwrap();
        let window = scene.window(id).unwrap();
        assert_eq!(window.windowskin, None);
        assert_eq!(window.contents, None);
        assert!(window.stretch);
        assert_eq!(window.cursor_rect, Rect::new(0, 0, 0, 0));
        assert!(window.active);
        assert!(window.visible);
        assert!(!window.pause);
        assert_eq!((window.x, window.y, window.z()), (0, 0, 0));
        assert_eq!((window.width, window.height), (0, 0));
        assert_eq!((window.ox, window.oy), (0, 0));
        assert_eq!(window.opacity(), 255);
        assert_eq!(window.back_opacity(), 255);
        assert_eq!(window.contents_opacity(), 255);
    }

    #[test]
    fn opacities_are_clamped_when_set() {
        let mut scene = Scene::new();
        let id = scene.create_sprite(None).unwrap();
        let sprite = scene.sprite_mut(id).unwrap();
        sprite.set_opacity(300);
        assert_eq!(sprite.opacity(), 255);
        sprite.set_opacity(-5);
        assert_eq!(sprite.opacity(), 0);
        sprite.set_opacity(128);
        assert_eq!(sprite.opacity(), 128);

        let id = scene.create_window(None).unwrap();
        let window = scene.window_mut(id).unwrap();
        window.set_opacity(256);
        window.set_back_opacity(i32::MAX);
        window.set_contents_opacity(1000);
        assert_eq!(
            (
                window.opacity(),
                window.back_opacity(),
                window.contents_opacity()
            ),
            (255, 255, 255)
        );
        window.set_opacity(-1);
        window.set_back_opacity(i32::MIN);
        window.set_contents_opacity(-300);
        assert_eq!(
            (
                window.opacity(),
                window.back_opacity(),
                window.contents_opacity()
            ),
            (0, 0, 0)
        );

        let id = scene.create_plane(None).unwrap();
        let plane = scene.plane_mut(id).unwrap();
        plane.set_opacity(300);
        assert_eq!(plane.opacity(), 255);
        plane.set_opacity(-5);
        assert_eq!(plane.opacity(), 0);
    }

    #[test]
    fn z_reads_back_what_was_set() {
        let mut scene = Scene::new();
        let sprite = scene.create_sprite(None).unwrap();
        scene.set_sprite_z(sprite, -7).unwrap();
        assert_eq!(scene.sprite(sprite).unwrap().z(), -7);
        let viewport = scene.create_viewport(Rect::default()).unwrap();
        scene.set_viewport_z(viewport, i32::MAX).unwrap();
        assert_eq!(scene.viewport(viewport).unwrap().z(), i32::MAX);
        let window = scene.create_window(None).unwrap();
        scene.set_window_z(window, 100).unwrap();
        assert_eq!(scene.window(window).unwrap().z(), 100);
    }

    #[test]
    fn a_plane_returns_the_properties_it_was_given() {
        let mut bitmaps = Bitmaps::new();
        let bitmap = bitmaps
            .create(4, 4, &crate::bitmap::testing::defaults())
            .unwrap();
        let mut scene = Scene::new();
        let id = scene.create_plane(None).unwrap();
        {
            let plane = scene.plane(id).unwrap();
            assert_eq!(plane.bitmap, None);
            assert!(plane.visible);
            assert_eq!((plane.ox, plane.oy, plane.z()), (0, 0, 0));
            assert_eq!((plane.zoom_x, plane.zoom_y), (1.0, 1.0));
            assert_eq!((plane.opacity(), plane.blend_type), (255, 0));
            assert_eq!(plane.color, Color::TRANSPARENT);
            assert_eq!(plane.tone, Tone::NEUTRAL);
        }
        let plane = scene.plane_mut(id).unwrap();
        plane.bitmap = Some(bitmap);
        plane.visible = false;
        plane.ox = 3;
        plane.oy = -4;
        plane.zoom_x = 2.0;
        plane.zoom_y = 0.5;
        plane.set_opacity(100);
        plane.blend_type = 1;
        plane.color = Color::new(1.0, 2.0, 3.0, 4.0);
        plane.tone = Tone::new(-1.0, 2.0, -3.0, 4.0);
        scene.set_plane_z(id, 9).unwrap();

        let plane = scene.plane(id).unwrap();
        assert_eq!(plane.bitmap, Some(bitmap));
        assert!(!plane.visible);
        assert_eq!((plane.ox, plane.oy, plane.z()), (3, -4, 9));
        assert_eq!((plane.zoom_x, plane.zoom_y), (2.0, 0.5));
        assert_eq!((plane.opacity(), plane.blend_type), (100, 1));
        assert_eq!(plane.color, Color::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(plane.tone, Tone::new(-1.0, 2.0, -3.0, 4.0));
    }

    #[test]
    fn a_tilemap_returns_the_properties_it_was_given() {
        let mut bitmaps = Bitmaps::new();
        let defaults = crate::bitmap::testing::defaults();
        let tileset = bitmaps.create(8, 8, &defaults).unwrap();
        let autotile = bitmaps.create(4, 4, &defaults).unwrap();
        let mut scene = Scene::new();
        let id = scene.create_tilemap(None).unwrap();
        {
            let tilemap = scene.tilemap(id).unwrap();
            assert_eq!(tilemap.tileset, None);
            assert_eq!(tilemap.autotiles, [None; 7]);
            assert_eq!(tilemap.map_data, None);
            assert_eq!(tilemap.flash_data, None);
            assert_eq!(tilemap.priorities, None);
            assert!(tilemap.visible);
            assert_eq!((tilemap.ox, tilemap.oy), (0, 0));
        }
        let map = SharedTable::new(Table::new_3d(2, 2, 3).unwrap());
        map.set(1, 1, 2, 384);
        let flash = SharedTable::new(Table::new_2d(2, 2).unwrap());
        let priorities = SharedTable::new(Table::new(400).unwrap());
        let tilemap = scene.tilemap_mut(id).unwrap();
        tilemap.tileset = Some(tileset);
        tilemap.autotiles[6] = Some(autotile);
        tilemap.map_data = Some(map.clone());
        tilemap.flash_data = Some(flash.clone());
        tilemap.priorities = Some(priorities.clone());
        tilemap.visible = false;
        tilemap.ox = 32;
        tilemap.oy = -32;
        tilemap.update();

        let tilemap = scene.tilemap(id).unwrap();
        assert_eq!(tilemap.tileset, Some(tileset));
        assert_eq!(tilemap.autotiles[6], Some(autotile));
        assert_eq!(tilemap.autotiles[0], None);
        assert_eq!(tilemap.map_data, Some(map));
        assert_eq!(tilemap.flash_data, Some(flash));
        assert_eq!(tilemap.priorities, Some(priorities));
        assert!(!tilemap.visible);
        assert_eq!((tilemap.ox, tilemap.oy), (32, -32));
    }

    #[test]
    fn the_logical_size_is_640_by_480_unless_given() {
        assert_eq!(Scene::new().logical_size(), Size::new(640, 480));
        assert_eq!(Scene::default().logical_size(), Size::new(640, 480));
        let scene = Scene::with_logical_size(Size::new(320, 240));
        assert_eq!(scene.logical_size(), Size::new(320, 240));
        let scene = Scene::with_logical_size(Size::new(0, 100_000));
        assert_eq!(scene.logical_size(), Size::new(1, MAX_LOGICAL_DIMENSION));
        assert_eq!(
            MAX_LOGICAL_DIMENSION,
            vkxp_compose_ref::MAX_LOGICAL_DIMENSION
        );
    }
}
