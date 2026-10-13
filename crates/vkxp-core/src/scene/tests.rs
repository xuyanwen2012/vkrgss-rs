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
fn a_cleared_scene_is_a_new_one_whose_handles_go_on() {
    let mut scene = Scene::with_logical_size(Size::new(320, 240));
    let disposed = scene.create_sprite(None).unwrap();
    scene.dispose_sprite(disposed);
    let before = [
        scene.create_sprite(None).unwrap(),
        scene.create_sprite(None).unwrap(),
    ];
    let viewport = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
    scene.frame_count = 7;
    scene.clear();

    assert_eq!(scene.logical_size(), Scene::DEFAULT_SIZE);
    assert_eq!(scene.frame_count, 0);
    assert_eq!(scene.sprites.iter().count(), 0);
    let after = [
        scene.create_sprite(None).unwrap(),
        scene.create_sprite(None).unwrap(),
        scene.create_sprite(None).unwrap(),
    ];
    assert_eq!(scene.sprites.iter().count(), 3);
    let again = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
    for old in before.into_iter().chain([disposed]) {
        assert!(scene.is_sprite_disposed(old));
        assert!(!after.contains(&old));
    }
    assert!(scene.is_viewport_disposed(viewport));
    assert_ne!(again, viewport);
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
}

#[test]
fn the_logical_size_can_be_set_and_the_next_packet_carries_it() {
    let mut scene = Scene::with_logical_size(Size::new(544, 416));
    let mut bitmaps = crate::Bitmaps::new();
    assert_eq!(
        scene.extract(&mut bitmaps).logical_size,
        Size::new(544, 416)
    );
    scene.set_logical_size(Size::new(640, 480));
    assert_eq!(scene.logical_size(), Size::new(640, 480));
    assert_eq!(
        scene.extract(&mut bitmaps).logical_size,
        Size::new(640, 480)
    );
    scene.set_logical_size(Size::new(0, 100_000));
    assert_eq!(scene.logical_size(), Size::new(1, MAX_LOGICAL_DIMENSION));
}

/// The measured draw orders among elements with equal z: every row of the table of the z
/// order probe of the original runtime (RGSS103J), where all elements start at z 0.
mod measured_order {
    use vkxp_pixel::Rgba;

    use super::*;
    use crate::extract::testing::*;

    /// The pixel every element of a row covers.
    const AT: (u32, u32) = (20, 20);

    /// An element of a row, which shows one colour of its own at [`AT`].
    #[derive(Clone, Copy)]
    enum Element {
        Sprite(SpriteId),
        Plane(PlaneId),
        /// A window without a skin: its contents are all it shows.
        Window(WindowId),
    }

    /// The elements of a row with their names, on a stage.
    struct Row {
        stage: Stage,
        elements: Vec<(char, Rgba, Element)>,
    }

    impl Row {
        fn new() -> Row {
            Row {
                stage: Stage::sized(64, 48),
                elements: Vec::new(),
            }
        }

        /// A bitmap of a colour no other element of the row has.
        fn colour(&mut self) -> (Rgba, vkxp_proto::BitmapId) {
            let colour = [40 * (self.elements.len() as u8 + 1), 0, 0, 255];
            (colour, self.stage.solid(32, 32, colour))
        }

        fn sprite(&mut self, name: char, viewport: Option<ViewportId>) -> SpriteId {
            let (colour, bitmap) = self.colour();
            let id = self.stage.scene.create_sprite(viewport).unwrap();
            let sprite = self.stage.scene.sprite_mut(id).unwrap();
            sprite.set_bitmap(Some(bitmap), &self.stage.bitmaps);
            self.elements.push((name, colour, Element::Sprite(id)));
            id
        }

        fn sprites<const N: usize>(&mut self, names: [char; N]) -> [SpriteId; N] {
            names.map(|name| self.sprite(name, None))
        }

        fn plane(&mut self, name: char) -> PlaneId {
            let (colour, bitmap) = self.colour();
            let id = self.stage.scene.create_plane(None).unwrap();
            self.stage.scene.plane_mut(id).unwrap().bitmap = Some(bitmap);
            self.elements.push((name, colour, Element::Plane(id)));
            id
        }

        fn window(&mut self, name: char) -> WindowId {
            let (colour, bitmap) = self.colour();
            let id = self.stage.scene.create_window(None).unwrap();
            let window = self.stage.scene.window_mut(id).unwrap();
            (window.width, window.height) = (64, 48);
            window.contents = Some(bitmap);
            self.elements.push((name, colour, Element::Window(id)));
            id
        }

        fn viewport(&mut self) -> ViewportId {
            let rect = Rect::new(0, 0, 64, 48);
            self.stage.scene.create_viewport(rect).unwrap()
        }

        fn z(&mut self, sprite: SpriteId, z: i32) {
            self.stage.scene.set_sprite_z(sprite, z).unwrap();
        }

        /// A z taken away and back.
        fn away_and_back(&mut self, sprite: SpriteId, z: i32) {
            self.z(sprite, z);
            self.z(sprite, 0);
        }

        /// The names of the elements from the bottom to the top: the one in front is read
        /// off the frame and hidden, which does not change the order of the others.
        fn order(&mut self) -> String {
            let mut names = Vec::new();
            for _ in 0..self.elements.len() {
                let shown = self.stage.frame().pixel(AT.0, AT.1);
                let &(name, _, element) = self
                    .elements
                    .iter()
                    .find(|(_, colour, _)| *colour == shown)
                    .expect("an element of the row is shown");
                let scene = &mut self.stage.scene;
                match element {
                    Element::Sprite(id) => scene.sprite_mut(id).unwrap().visible = false,
                    Element::Plane(id) => scene.plane_mut(id).unwrap().visible = false,
                    Element::Window(id) => scene.window_mut(id).unwrap().visible = false,
                }
                names.push(name);
            }
            names.iter().rev().collect()
        }
    }

    #[test]
    fn sprites_are_drawn_in_the_order_of_creation() {
        let mut row = Row::new();
        row.sprites(['A', 'B', 'C']);
        assert_eq!(row.order(), "ABC");
    }

    #[test]
    fn a_z_taken_away_and_back_keeps_the_place() {
        let mut row = Row::new();
        let [a, _, _] = row.sprites(['A', 'B', 'C']);
        row.away_and_back(a, 5);
        assert_eq!(row.order(), "ABC", "A.z = 5; A.z = 0");

        let mut row = Row::new();
        let [_, _, c] = row.sprites(['A', 'B', 'C']);
        row.away_and_back(c, -5);
        assert_eq!(row.order(), "ABC", "C.z = -5; C.z = 0");

        let mut row = Row::new();
        let [_, b, _] = row.sprites(['A', 'B', 'C']);
        row.away_and_back(b, -5);
        assert_eq!(row.order(), "ABC", "B.z = -5; B.z = 0");
    }

    #[test]
    fn the_order_of_the_z_assignments_does_not_count() {
        let mut row = Row::new();
        let [a, b, c] = row.sprites(['A', 'B', 'C']);
        for sprite in [c, b, a] {
            row.z(sprite, 10);
        }
        assert_eq!(row.order(), "ABC");

        let mut row = Row::new();
        let [a, b, c, _] = row.sprites(['A', 'B', 'C', 'D']);
        for sprite in [a, b, c] {
            row.z(sprite, 3);
        }
        assert_eq!(row.order(), "DABC");

        let mut row = Row::new();
        let [_, b, c, d] = row.sprites(['A', 'B', 'C', 'D']);
        for sprite in [d, c, b] {
            row.z(sprite, -3);
        }
        assert_eq!(row.order(), "BCDA");
    }

    #[test]
    fn a_frame_drawn_in_between_changes_nothing() {
        let mut row = Row::new();
        let [a, _, _] = row.sprites(['A', 'B', 'C']);
        row.stage.frame();
        row.away_and_back(a, 5);
        assert_eq!(row.order(), "ABC");
    }

    #[test]
    fn a_plane_keeps_its_place() {
        let mut row = Row::new();
        let a = row.plane('A');
        row.sprite('B', None);
        row.stage.scene.set_plane_z(a, 5).unwrap();
        row.stage.scene.set_plane_z(a, 0).unwrap();
        assert_eq!(row.order(), "AB");
    }

    #[test]
    fn a_window_keeps_its_place() {
        let mut row = Row::new();
        let a = row.window('A');
        row.window('B');
        row.stage.scene.set_window_z(a, 5).unwrap();
        row.stage.scene.set_window_z(a, 0).unwrap();
        assert_eq!(row.order(), "AB", "the contents of A are below those of B");
    }

    #[test]
    fn a_viewport_keeps_its_place() {
        let mut row = Row::new();
        let (v, w) = (row.viewport(), row.viewport());
        row.sprite('A', Some(v));
        row.sprite('B', Some(w));
        row.stage.scene.set_viewport_z(v, 5).unwrap();
        row.stage.scene.set_viewport_z(v, 0).unwrap();
        assert_eq!(row.order(), "AB");

        let mut row = Row::new();
        let v = row.viewport();
        row.sprite('A', Some(v));
        row.sprite('B', None);
        row.stage.scene.set_viewport_z(v, 5).unwrap();
        row.stage.scene.set_viewport_z(v, 0).unwrap();
        assert_eq!(row.order(), "AB");
    }

    #[test]
    fn a_viewport_ranks_by_its_own_creation_not_that_of_its_elements() {
        let mut row = Row::new();
        let v = row.viewport();
        row.sprite('B', None);
        row.sprite('A', Some(v));
        assert_eq!(row.order(), "AB", "the viewport is older than B");

        let mut row = Row::new();
        row.sprite('B', None);
        let v = row.viewport();
        row.sprite('A', Some(v));
        assert_eq!(row.order(), "BA");
    }

    #[test]
    fn a_sprite_older_than_a_tilemap_stays_behind_its_ground_tiles() {
        let mut row = Row::new();
        let s = row.sprite('S', None);
        // A map of one cell, which repeats over the screen, of a white tile of priority 0.
        let tileset = row.stage.solid(256, 32, WHITE);
        let map = SharedTable::new(Table::new_3d(1, 1, 3).unwrap());
        map.set(0, 0, 0, 384);
        let id = row.stage.scene.create_tilemap(None).unwrap();
        let tilemap = row.stage.scene.tilemap_mut(id).unwrap();
        tilemap.tileset = Some(tileset);
        tilemap.map_data = Some(map);
        assert_eq!(row.stage.frame().pixel(AT.0, AT.1), WHITE);

        row.z(s, 5);
        assert_ne!(
            row.stage.frame().pixel(AT.0, AT.1),
            WHITE,
            "z 5 is in front"
        );
        row.z(s, 0);
        assert_eq!(
            row.stage.frame().pixel(AT.0, AT.1),
            WHITE,
            "S.z = 5; S.z = 0"
        );
        row.away_and_back(s, -5);
        assert_eq!(row.stage.frame().pixel(AT.0, AT.1), WHITE);
    }
}

/// Moving an element to another viewport, as RGSS2 allows.
mod moving {
    use super::*;
    use crate::extract::testing::*;

    /// Scenario "Seen by the scene".
    #[test]
    fn a_moved_sprite_is_drawn_in_its_new_viewport() {
        let mut stage = Stage::sized(64, 48);
        let red = stage.solid(8, 8, RED);
        let sprite = stage.scene.create_sprite(None).unwrap();
        let bitmap = &stage.bitmaps;
        stage
            .scene
            .sprite_mut(sprite)
            .unwrap()
            .set_bitmap(Some(red), bitmap);
        assert_eq!(stage.frame().pixel(0, 0), RED);

        let viewport = stage
            .scene
            .create_viewport(Rect::new(20, 30, 16, 16))
            .unwrap();
        stage
            .scene
            .set_sprite_viewport(sprite, Some(viewport))
            .unwrap();
        assert_eq!(
            stage.scene.sprite(sprite).unwrap().viewport(),
            Some(viewport)
        );
        let frame = stage.frame();
        assert_eq!(frame.pixel(20, 30), RED);
        assert_ne!(frame.pixel(0, 0), RED);

        stage.scene.set_sprite_viewport(sprite, None).unwrap();
        let frame = stage.frame();
        assert_eq!(frame.pixel(0, 0), RED);
        assert_ne!(frame.pixel(20, 30), RED);
    }

    #[test]
    fn every_kind_moves_and_follows_its_new_viewport() {
        let mut scene = Scene::new();
        let mut bitmaps = Bitmaps::new();
        let old = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
        let new = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
        let sprite = scene.create_sprite(Some(old)).unwrap();
        let window = scene.create_window(Some(old)).unwrap();
        let plane = scene.create_plane(Some(old)).unwrap();
        let tilemap = scene.create_tilemap(Some(old)).unwrap();
        let stays = scene.create_sprite(Some(old)).unwrap();
        let free = scene.create_sprite(Some(old)).unwrap();

        scene.set_sprite_viewport(sprite, Some(new)).unwrap();
        scene.set_window_viewport(window, Some(new)).unwrap();
        scene.set_plane_viewport(plane, Some(new)).unwrap();
        scene.set_tilemap_viewport(tilemap, Some(new)).unwrap();
        scene.set_sprite_viewport(free, None).unwrap();
        assert_eq!(scene.sprite(sprite).unwrap().viewport(), Some(new));
        assert_eq!(scene.window(window).unwrap().viewport(), Some(new));
        assert_eq!(scene.plane(plane).unwrap().viewport(), Some(new));
        assert_eq!(scene.tilemap(tilemap).unwrap().viewport(), Some(new));
        assert_eq!(scene.sprite(free).unwrap().viewport(), None);

        // The old viewport takes along only what is still in it.
        let disposed = scene.dispose_viewport(old, &mut bitmaps);
        assert_eq!(disposed.sprites, 1);
        assert!(scene.is_sprite_disposed(stays));
        assert!(!scene.is_sprite_disposed(sprite) && !scene.is_sprite_disposed(free));
        let disposed = scene.dispose_viewport(new, &mut bitmaps);
        assert_eq!(
            disposed,
            DisposedElements {
                sprites: 1,
                windows: 1,
                planes: 1,
                tilemaps: 1,
            }
        );
    }

    #[test]
    fn a_disposed_viewport_or_element_moves_nothing() {
        let mut scene = Scene::new();
        let mut bitmaps = Bitmaps::new();
        let gone = scene.create_viewport(Rect::new(0, 0, 10, 10)).unwrap();
        scene.dispose_viewport(gone, &mut bitmaps);
        let disposed_viewport = Err(SceneError::Disposed(ElementKind::Viewport));

        let sprite = scene.create_sprite(None).unwrap();
        let window = scene.create_window(None).unwrap();
        let plane = scene.create_plane(None).unwrap();
        let tilemap = scene.create_tilemap(None).unwrap();
        assert_eq!(
            scene.set_sprite_viewport(sprite, Some(gone)),
            disposed_viewport
        );
        assert_eq!(
            scene.set_window_viewport(window, Some(gone)),
            disposed_viewport
        );
        assert_eq!(
            scene.set_plane_viewport(plane, Some(gone)),
            disposed_viewport
        );
        assert_eq!(
            scene.set_tilemap_viewport(tilemap, Some(gone)),
            disposed_viewport
        );
        assert_eq!(scene.sprite(sprite).unwrap().viewport(), None);

        scene.dispose_sprite(sprite);
        assert_eq!(
            scene.set_sprite_viewport(sprite, None),
            Err(DISPOSED_SPRITE)
        );
    }

    /// The place among equal z is that of creation, also after a move. Not measured on the
    /// original runtime.
    #[test]
    fn a_moved_sprite_keeps_its_place_among_equal_z() {
        let mut stage = Stage::sized(16, 16);
        let viewport = stage
            .scene
            .create_viewport(Rect::new(0, 0, 16, 16))
            .unwrap();
        let red = stage.solid(8, 8, RED);
        let green = stage.solid(8, 8, GREEN);
        let first = stage.scene.create_sprite(None).unwrap();
        let second = stage.scene.create_sprite(Some(viewport)).unwrap();
        let bitmap = &stage.bitmaps;
        stage
            .scene
            .sprite_mut(first)
            .unwrap()
            .set_bitmap(Some(red), bitmap);
        stage
            .scene
            .sprite_mut(second)
            .unwrap()
            .set_bitmap(Some(green), bitmap);
        stage
            .scene
            .set_sprite_viewport(first, Some(viewport))
            .unwrap();
        assert_eq!(stage.frame().pixel(4, 4), GREEN);
    }
}
