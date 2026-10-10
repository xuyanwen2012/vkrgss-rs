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
}
