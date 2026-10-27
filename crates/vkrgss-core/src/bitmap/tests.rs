use proptest::prelude::*;

use super::testing::*;
use super::*;

/// A model of one axis of the block transfer: the destination and source coordinates of the
/// positions drawn. An unmirrored span draws the positions that exist on both sides. A span
/// with a negative `len` is clipped before it is mirrored, by the four clips measured on the
/// originals.
fn naive_axis(dst_pos: i32, dst_size: u32, pos: i32, len: i32, src_size: u32) -> Vec<(i64, i64)> {
    let (dst_size, src_size) = (i64::from(dst_size), i64::from(src_size));
    let (mut dl, mut dr) = (
        i64::from(dst_pos),
        i64::from(dst_pos) + i64::from(len).abs(),
    );
    if len >= 0 {
        return (0..i64::from(len))
            .map(|i| (dl + i, i64::from(pos) + i))
            .filter(|(dst, src)| (0..dst_size).contains(dst) && (0..src_size).contains(src))
            .collect();
    }
    let (sl, mut sh) = (i64::from(pos) + i64::from(len), i64::from(pos));
    if sl < 0 {
        dl += -sl;
    }
    if dl < 0 {
        sh += dl;
        dl = 0;
    }
    if sh > src_size {
        dr += src_size - sh;
        sh = src_size;
    }
    // The last clip raises the low end of the source, which the mirrored walk never reaches.
    dr = dr.min(dst_size);
    (0..dr - dl).map(|i| (dl + i, sh - 1 - i)).collect()
}

/// A model of the block transfer, one pixel at a time from the top row down and from left to
/// right, reading from `src` as given, or, without one, from `dst` as it is at that moment: a
/// transfer within one bitmap.
#[allow(clippy::too_many_arguments)]
fn naive_blt(
    dst: &mut [u8],
    (dst_width, dst_height): (u32, u32),
    x: i32,
    y: i32,
    src: Option<&[u8]>,
    (src_width, src_height): (u32, u32),
    rect: Rect,
    opacity: u8,
) {
    let columns = naive_axis(x, dst_width, rect.x, rect.width, src_width);
    for (dy, sy) in naive_axis(y, dst_height, rect.y, rect.height, src_height) {
        for &(dx, sx) in &columns {
            let source = at(src.unwrap_or(dst), src_width, sx as u32, sy as u32);
            let offset = ((dy as u32 * dst_width + dx as u32) * 4) as usize;
            let target: Rgba = dst[offset..offset + 4].try_into().unwrap();
            dst[offset..offset + 4].copy_from_slice(&blend_normal(target, source, opacity));
        }
    }
}

// Creation.

#[test]
fn new_bitmap_is_transparent() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(4, 3, &defaults()).unwrap();
    assert_eq!(bitmaps.width(id), Ok(4));
    assert_eq!(bitmaps.height(id), Ok(3));
    assert_eq!(bitmaps.rect(id), Ok(Rect::new(0, 0, 4, 3)));
    assert_eq!(bitmaps.rgba(id).unwrap(), [0; 48]);
    for y in 0..3 {
        for x in 0..4 {
            assert_eq!(bitmaps.get_pixel(id, x, y), Ok(Color::TRANSPARENT));
        }
    }
}

#[test]
fn zero_side_fails() {
    let mut bitmaps = Bitmaps::new();
    assert_eq!(
        bitmaps.create(0, 5, &defaults()),
        Err(BitmapError::CreateFailed(CreateFailure::ZeroSide {
            width: 0,
            height: 5,
        }))
    );
    assert!(matches!(
        bitmaps.create(5, 0, &defaults()),
        Err(BitmapError::CreateFailed(CreateFailure::ZeroSide { .. }))
    ));
    assert!(matches!(
        bitmaps.create_from_rgba(0, 0, Vec::new(), &defaults()),
        Err(BitmapError::CreateFailed(CreateFailure::ZeroSide { .. }))
    ));
    assert!(bitmaps.is_empty());
}

#[test]
fn too_large_fails_and_keeps_nothing() {
    let mut bitmaps = Bitmaps::new();
    // One row more than the limit of 2^28 pixels.
    let (width, height) = (1 << 14, (1 << 14) + 1);
    assert_eq!(
        bitmaps.create(width, height, &defaults()),
        Err(BitmapError::CreateFailed(CreateFailure::TooLarge {
            width,
            height,
            limit: MAX_BITMAP_PIXELS,
        }))
    );
    assert!(matches!(
        bitmaps.create(u32::MAX, u32::MAX, &defaults()),
        Err(BitmapError::CreateFailed(CreateFailure::TooLarge { .. }))
    ));
    assert!(matches!(
        bitmaps.create_from_rgba(width, height, vec![0; 4], &defaults()),
        Err(BitmapError::CreateFailed(CreateFailure::TooLarge { .. }))
    ));
    // No slot was taken, so no pixels are held for the failed bitmaps.
    assert_eq!(bitmaps.len(), 0);
    assert!(bitmaps.slots.is_empty());
}

#[test]
fn limits_agree() {
    // One definition in the protocol, which the compositors use as well; the decoders, which
    // depend on no crate of the workspace, keep a copy of their own.
    assert_eq!(MAX_BITMAP_PIXELS, vkrgss_proto::MAX_BITMAP_PIXELS);
    assert_eq!(
        vkrgss_compose_ref::MAX_BITMAP_PIXELS,
        vkrgss_proto::MAX_BITMAP_PIXELS
    );
    assert_eq!(vkrgss_image::MAX_PIXELS, vkrgss_proto::MAX_BITMAP_PIXELS);
}

#[test]
fn from_pixels_reads_back_each_position() {
    let mut bitmaps = Bitmaps::new();
    let pixels: [Rgba; 4] = [
        [1, 2, 3, 4],
        [5, 6, 7, 8],
        [9, 10, 11, 12],
        [13, 14, 15, 16],
    ];
    let id = bitmaps
        .create_from_rgba(2, 2, pixels.concat(), &defaults())
        .unwrap();
    assert_eq!(bitmaps.get_pixel(id, 0, 0), Ok(color(pixels[0])));
    assert_eq!(bitmaps.get_pixel(id, 1, 0), Ok(color(pixels[1])));
    assert_eq!(bitmaps.get_pixel(id, 0, 1), Ok(color(pixels[2])));
    assert_eq!(bitmaps.get_pixel(id, 1, 1), Ok(color(pixels[3])));
    assert_eq!(bitmaps.rgba(id).unwrap(), pixels.concat());
}

#[test]
fn pixels_of_the_wrong_length_fail() {
    let mut bitmaps = Bitmaps::new();
    for len in [0, 15, 17] {
        assert_eq!(
            bitmaps.create_from_rgba(2, 2, vec![0; len], &defaults()),
            Err(BitmapError::CreateFailed(CreateFailure::PixelLength {
                width: 2,
                height: 2,
                expected: 16,
                found: len,
            }))
        );
    }
    assert!(bitmaps.is_empty());
}

// Font.

#[test]
fn font_comes_from_the_defaults() {
    let mut defaults = defaults();
    defaults.size = 30;
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(1, 1, &defaults).unwrap();
    assert_eq!(bitmaps.font(id).unwrap().size, 30);
    assert_eq!(bitmaps.font(id).unwrap(), &Font::new(&defaults));
    let from_pixels = bitmaps
        .create_from_rgba(1, 1, vec![0; 4], &defaults)
        .unwrap();
    assert_eq!(bitmaps.font(from_pixels).unwrap().size, 30);
}

#[test]
fn fonts_are_not_shared() {
    let defaults = defaults();
    let mut bitmaps = Bitmaps::new();
    let first = bitmaps.create(1, 1, &defaults).unwrap();
    let second = bitmaps.create(1, 1, &defaults).unwrap();
    bitmaps.font_mut(first).unwrap().size = 40;
    assert_eq!(bitmaps.font(second).unwrap(), &Font::new(&defaults));

    let mut replacement = Font::new(&defaults);
    replacement.bold = true;
    replacement.names = vec!["Other".to_owned()];
    bitmaps.set_font(second, replacement.clone()).unwrap();
    assert_eq!(bitmaps.font(second).unwrap(), &replacement);
    assert_eq!(bitmaps.font(first).unwrap().size, 40);
    assert!(!bitmaps.font(first).unwrap().bold);
}

// Disposal and identity.

#[test]
fn disposing_twice_does_nothing() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(2, 2, &defaults()).unwrap();
    let other = bitmaps.create(2, 2, &defaults()).unwrap();
    assert!(!bitmaps.is_disposed(id));
    bitmaps.dispose(id);
    assert!(bitmaps.is_disposed(id));
    bitmaps.dispose(id);
    assert!(bitmaps.is_disposed(id));
    assert!(!bitmaps.is_disposed(other));
    assert_eq!(bitmaps.len(), 1);
    // An id this arena never handed out is disposed as well.
    bitmaps.dispose(BitmapId::new(99, 0));
    assert!(bitmaps.is_disposed(BitmapId::new(99, 0)));
}

#[test]
fn every_operation_on_a_disposed_bitmap_fails() {
    let mut bitmaps = Bitmaps::new();
    let live = bitmaps
        .create_from_rgba(2, 2, pattern(2, 2, 1), &defaults())
        .unwrap();
    let gone = bitmaps.create(2, 2, &defaults()).unwrap();
    bitmaps.dispose(gone);
    let rect = Rect::new(0, 0, 2, 2);
    let disposed = Err(BitmapError::Disposed);

    assert_eq!(
        bitmaps.set_pixel(gone, 0, 0, Color::rgb(1.0, 2.0, 3.0)),
        disposed
    );
    assert_eq!(bitmaps.get_pixel(gone, 0, 0), Err(BitmapError::Disposed));
    assert_eq!(bitmaps.width(gone), Err(BitmapError::Disposed));
    assert_eq!(bitmaps.height(gone), Err(BitmapError::Disposed));
    assert_eq!(bitmaps.rect(gone), Err(BitmapError::Disposed));
    assert_eq!(bitmaps.rgba(gone), Err(BitmapError::Disposed));
    assert_eq!(bitmaps.font(gone).err(), Some(BitmapError::Disposed));
    assert_eq!(bitmaps.font_mut(gone).err(), Some(BitmapError::Disposed));
    assert_eq!(bitmaps.set_font(gone, Font::new(&defaults())), disposed);
    assert_eq!(bitmaps.clear(gone), disposed);
    assert_eq!(bitmaps.fill_rect(gone, rect, Color::TRANSPARENT), disposed);
    assert_eq!(bitmaps.hue_change(gone, 90), disposed);
    assert_eq!(bitmaps.blt(gone, 0, 0, live, rect, 255), disposed);
    assert_eq!(bitmaps.blt(gone, 0, 0, gone, rect, 255), disposed);
    assert_eq!(bitmaps.stretch_blt(gone, rect, live, rect, 255), disposed);
    // Nothing happened to the bitmap that is still alive.
    assert_eq!(bitmaps.rgba(live).unwrap(), pattern(2, 2, 1));
}

/// A disposed source is no error, as measured on the originals: the destination rectangle
/// is blended onto itself.
#[test]
fn disposed_source_blends_the_destination_onto_itself() {
    let mut bitmaps = Bitmaps::new();
    let pixels: [Rgba; 4] = [
        [10, 20, 30, 0],
        [40, 50, 60, 128],
        [70, 80, 90, 255],
        [1, 2, 3, 128],
    ];
    let dst = bitmaps
        .create_from_rgba(4, 1, pixels.concat(), &defaults())
        .unwrap();
    let src = bitmaps
        .create_from_rgba(2, 2, pattern(2, 2, 9), &defaults())
        .unwrap();
    bitmaps.dispose(src);
    // The position of the source rectangle is not used, and its size is taken absolute.
    let rect = Rect::new(7, -5, -3, 1);
    assert_eq!(bitmaps.blt(dst, 0, 0, src, rect, 255), Ok(()));
    let after: [Rgba; 4] = [
        [10, 20, 30, 0],
        [40, 50, 60, 191],
        [70, 80, 90, 255],
        pixels[3],
    ];
    assert_eq!(bitmaps.rgba(dst).unwrap(), after.concat());

    // `stretch_blt` blends its destination rectangle, clipped to the bitmap.
    let all = Rect::new(-2, -2, 9, 9);
    assert_eq!(bitmaps.stretch_blt(dst, all, src, rect, 255), Ok(()));
    let twice = blend_normal(after[1], after[1], 255);
    let after: [Rgba; 4] = [after[0], twice, after[2], [1, 2, 3, 191]];
    assert_eq!(bitmaps.rgba(dst).unwrap(), after.concat());

    // An opacity that draws nothing and an empty rectangle change nothing.
    for opacity in [0, -1, 256] {
        assert_eq!(bitmaps.blt(dst, 0, 0, src, rect, opacity), Ok(()));
    }
    assert_eq!(
        bitmaps.blt(dst, 0, 0, src, Rect::new(0, 0, 0, 1), 255),
        Ok(())
    );
    let empty = Rect::new(0, 0, -4, 1);
    assert_eq!(bitmaps.stretch_blt(dst, empty, src, rect, 255), Ok(()));
    assert_eq!(bitmaps.rgba(dst).unwrap(), after.concat());

    // A new occupant of the slot does not revive the old id.
    let reused = bitmaps
        .create_from_rgba(2, 2, pattern(2, 2, 50), &defaults())
        .unwrap();
    assert_eq!(reused.index, src.index);
    assert_eq!(
        bitmaps.blt(dst, 3, 0, src, Rect::new(0, 0, 1, 1), 255),
        Ok(())
    );
    let twice = blend_normal(after[3], after[3], 255);
    assert_eq!(
        bitmaps.rgba(dst).unwrap(),
        [after[0], after[1], after[2], twice].concat()
    );

    // A disposed destination fails, whatever the source is.
    bitmaps.dispose(dst);
    assert_eq!(
        bitmaps.blt(dst, 0, 0, src, rect, 255),
        Err(BitmapError::Disposed)
    );
    assert_eq!(
        bitmaps.stretch_blt(dst, all, src, rect, 255),
        Err(BitmapError::Disposed)
    );
}

#[test]
fn slot_reuse_gives_a_new_id() {
    let mut bitmaps = Bitmaps::new();
    let mut seen = std::collections::HashSet::new();
    let keep = bitmaps.create(1, 1, &defaults()).unwrap();
    assert!(seen.insert(keep));
    let mut id = bitmaps.create(1, 1, &defaults()).unwrap();
    for _ in 0..20 {
        assert!(seen.insert(id), "{id} was handed out twice");
        bitmaps.dispose(id);
        let next = bitmaps.create(1, 1, &defaults()).unwrap();
        assert_eq!(next.index, id.index, "the slot is reused");
        assert_ne!(next, id);
        assert!(bitmaps.is_disposed(id));
        assert!(!bitmaps.is_disposed(next));
        id = next;
    }
    assert_eq!(bitmaps.len(), 2);
}

#[test]
fn a_slot_at_the_last_generation_is_retired() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(1, 1, &defaults()).unwrap();
    bitmaps.slots[id.index as usize].generation = u32::MAX;
    let last = BitmapId::new(id.index, u32::MAX);
    assert!(!bitmaps.is_disposed(last));
    bitmaps.dispose(last);
    let next = bitmaps.create(1, 1, &defaults()).unwrap();
    assert_ne!(next.index, last.index);
    assert!(bitmaps.is_disposed(last));
}

// Pixel access.

#[test]
fn write_then_read_discards_fractions() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(3, 2, &defaults()).unwrap();
    bitmaps
        .set_pixel(id, 2, 1, Color::new(127.9, 0.5, 200.0, 99.99))
        .unwrap();
    assert_eq!(
        bitmaps.get_pixel(id, 2, 1),
        Ok(Color::new(127.0, 0.0, 200.0, 99.0))
    );
    // All four channels are replaced, not blended.
    bitmaps
        .set_pixel(id, 2, 1, Color::new(1.0, 2.0, 3.0, 0.0))
        .unwrap();
    assert_eq!(
        bitmaps.get_pixel(id, 2, 1),
        Ok(Color::new(1.0, 2.0, 3.0, 0.0))
    );
    assert_eq!(at(bitmaps.rgba(id).unwrap(), 3, 2, 1), [1, 2, 3, 0]);
}

#[test]
fn negative_components_are_stored_modulo_256() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(2, 1, &defaults()).unwrap();
    let color = Color::new(-5.0, 0.0, 0.0, 255.0);
    assert_eq!(color.red(), -5.0);
    bitmaps.set_pixel(id, 0, 0, color).unwrap();
    assert_eq!(bitmaps.get_pixel(id, 0, 0).unwrap().red(), 251.0);
    bitmaps
        .fill_rect(id, Rect::new(1, 0, 1, 1), Color::new(-0.5, -1.0, 7.6, -5.0))
        .unwrap();
    assert_eq!(bitmaps.rgba(id).unwrap(), [251, 0, 0, 255, 0, 255, 7, 0]);
}

#[test]
fn reading_outside_gives_all_zero() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps
        .create_from_rgba(3, 2, vec![200; 24], &defaults())
        .unwrap();
    for (x, y) in [
        (-1, 0),
        (0, -1),
        (3, 0),
        (0, 2),
        (i32::MIN, 0),
        (i32::MAX, i32::MAX),
    ] {
        assert_eq!(
            bitmaps.get_pixel(id, x, y),
            Ok(Color::new(0.0, 0.0, 0.0, 0.0))
        );
    }
    assert_eq!(bitmaps.get_pixel(id, 2, 1), Ok(color([200; 4])));
}

#[test]
fn writing_outside_changes_nothing() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps
        .create_from_rgba(3, 2, pattern(3, 2, 4), &defaults())
        .unwrap();
    for (x, y) in [
        (-1, 0),
        (0, -1),
        (3, 0),
        (0, 2),
        (i32::MIN, 0),
        (i32::MAX, i32::MAX),
    ] {
        bitmaps
            .set_pixel(id, x, y, Color::rgb(9.0, 9.0, 9.0))
            .unwrap();
    }
    assert_eq!(bitmaps.rgba(id).unwrap(), pattern(3, 2, 4));
}

// Clear and fill.

#[test]
fn fill_replaces_alpha() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps
        .create_from_rgba(2, 2, [9, 9, 9, 255].repeat(4), &defaults())
        .unwrap();
    bitmaps
        .fill_rect(id, Rect::new(0, 0, 2, 1), Color::new(10.0, 20.0, 30.0, 0.0))
        .unwrap();
    let rgba = bitmaps.rgba(id).unwrap();
    assert_eq!(at(rgba, 2, 0, 0), [10, 20, 30, 0]);
    assert_eq!(at(rgba, 2, 1, 0), [10, 20, 30, 0]);
    assert_eq!(at(rgba, 2, 0, 1), [9, 9, 9, 255]);
    assert_eq!(at(rgba, 2, 1, 1), [9, 9, 9, 255]);
}

#[test]
fn fill_is_clipped_to_the_bitmap() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps.create(4, 3, &defaults()).unwrap();
    bitmaps
        .fill_rect(id, Rect::new(2, 1, 100, 100), Color::rgb(1.0, 2.0, 3.0))
        .unwrap();
    let rgba = bitmaps.rgba(id).unwrap();
    for y in 0..3 {
        for x in 0..4 {
            let expected = if x >= 2 && y >= 1 {
                [1, 2, 3, 255]
            } else {
                [0; 4]
            };
            assert_eq!(at(rgba, 4, x, y), expected, "({x}, {y})");
        }
    }
    // Past the left and top edges, and with extreme values.
    bitmaps
        .fill_rect(id, Rect::new(-5, -5, 6, 6), Color::rgb(7.0, 7.0, 7.0))
        .unwrap();
    bitmaps
        .fill_rect(
            id,
            Rect::new(i32::MAX, i32::MAX, i32::MAX, i32::MAX),
            Color::TRANSPARENT,
        )
        .unwrap();
    let rgba = bitmaps.rgba(id).unwrap();
    assert_eq!(at(rgba, 4, 0, 0), [7, 7, 7, 255]);
    assert_eq!(at(rgba, 4, 1, 0), [0; 4]);
    assert_eq!(at(rgba, 4, 0, 1), [0; 4]);
    assert_eq!(at(rgba, 4, 3, 2), [1, 2, 3, 255]);
}

#[test]
fn fill_of_an_empty_rectangle_fills_nothing() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps
        .create_from_rgba(3, 3, pattern(3, 3, 2), &defaults())
        .unwrap();
    for rect in [
        Rect::new(0, 0, 0, 3),
        Rect::new(0, 0, 3, 0),
        Rect::new(2, 2, -2, 2),
        Rect::new(2, 2, 2, -2),
        Rect::new(1, 1, i32::MIN, i32::MIN),
    ] {
        bitmaps
            .fill_rect(id, rect, Color::rgb(1.0, 1.0, 1.0))
            .unwrap();
    }
    assert_eq!(bitmaps.rgba(id).unwrap(), pattern(3, 3, 2));
}

#[test]
fn clear_makes_everything_transparent_black() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps
        .create_from_rgba(3, 3, pattern(3, 3, 2), &defaults())
        .unwrap();
    bitmaps.clear(id).unwrap();
    assert_eq!(bitmaps.rgba(id).unwrap(), [0; 36]);
}

// Block transfer.

#[test]
fn opaque_copy() {
    let mut bitmaps = Bitmaps::new();
    let mut source = pattern(3, 2, 5);
    for pixel in source.as_chunks_mut::<4>().0 {
        pixel[3] = 255;
    }
    let src = bitmaps
        .create_from_rgba(3, 2, source.clone(), &defaults())
        .unwrap();
    let dst = bitmaps
        .create_from_rgba(5, 4, pattern(5, 4, 77), &defaults())
        .unwrap();
    bitmaps
        .blt(dst, 1, 1, src, Rect::new(0, 0, 3, 2), 255)
        .unwrap();
    let rgba = bitmaps.rgba(dst).unwrap();
    let untouched = pattern(5, 4, 77);
    for y in 0..4 {
        for x in 0..5 {
            let expected = if (1..4).contains(&x) && (1..3).contains(&y) {
                at(&source, 3, x - 1, y - 1)
            } else {
                at(&untouched, 5, x, y)
            };
            assert_eq!(at(rgba, 5, x, y), expected, "({x}, {y})");
        }
    }
}

#[test]
fn zero_opacity_changes_nothing() {
    let mut bitmaps = Bitmaps::new();
    let src = bitmaps
        .create_from_rgba(3, 2, pattern(3, 2, 5), &defaults())
        .unwrap();
    let dst = bitmaps
        .create_from_rgba(5, 4, pattern(5, 4, 77), &defaults())
        .unwrap();
    bitmaps
        .blt(dst, 1, 1, src, Rect::new(0, 0, 3, 2), 0)
        .unwrap();
    assert_eq!(bitmaps.rgba(dst).unwrap(), pattern(5, 4, 77));
}

/// Measured on the originals: an opacity above 255 is not limited to 255.
#[test]
fn opacity_above_255_changes_nothing() {
    let mut bitmaps = Bitmaps::new();
    let src = bitmaps
        .create_from_rgba(3, 3, pattern(3, 3, 5), &defaults())
        .unwrap();
    let before = pattern(3, 3, 60);
    let dst = bitmaps
        .create_from_rgba(3, 3, before.clone(), &defaults())
        .unwrap();
    let rect = Rect::new(0, 0, 3, 3);
    for opacity in [256, 300, 65536, 1073741823, -1, i32::MIN] {
        bitmaps.blt(dst, 0, 0, src, rect, opacity).unwrap();
        bitmaps.stretch_blt(dst, rect, src, rect, opacity).unwrap();
        let half = Rect::new(0, 0, 2, 2);
        bitmaps.stretch_blt(dst, rect, src, half, opacity).unwrap();
        assert_eq!(bitmaps.rgba(dst).unwrap(), before, "opacity {opacity}");
    }
}

#[test]
fn onto_a_transparent_destination() {
    let mut bitmaps = Bitmaps::new();
    let src = bitmaps
        .create_from_rgba(1, 1, vec![200, 100, 50, 128], &defaults())
        .unwrap();
    let dst = bitmaps.create(2, 1, &defaults()).unwrap();
    bitmaps
        .blt(dst, 1, 0, src, Rect::new(0, 0, 1, 1), 255)
        .unwrap();
    assert_eq!(bitmaps.rgba(dst).unwrap(), [0, 0, 0, 0, 200, 100, 50, 128]);
}

#[test]
fn source_rectangle_partly_outside_the_source() {
    let mut bitmaps = Bitmaps::new();
    let source = [[1, 1, 1, 255], [2, 2, 2, 255], [3, 3, 3, 255]].concat();
    let src = bitmaps.create_from_rgba(3, 1, source, &defaults()).unwrap();
    let dst = bitmaps.create(8, 1, &defaults()).unwrap();
    // Columns -2 and -1 of the rectangle do not exist; column 0 is its third.
    bitmaps
        .blt(dst, 1, 0, src, Rect::new(-2, 0, 4, 1), 255)
        .unwrap();
    let rgba = bitmaps.rgba(dst).unwrap();
    let alphas: Vec<u8> = (0..8).map(|x| at(rgba, 8, x, 0)[3]).collect();
    assert_eq!(alphas, [0, 0, 0, 255, 255, 0, 0, 0]);
    assert_eq!(at(rgba, 8, 3, 0), [1, 1, 1, 255]);
    assert_eq!(at(rgba, 8, 4, 0), [2, 2, 2, 255]);

    // Past the right and bottom of the source only the existing part is drawn.
    bitmaps.clear(dst).unwrap();
    bitmaps
        .blt(dst, 0, 0, src, Rect::new(2, 0, 5, 5), 255)
        .unwrap();
    let rgba = bitmaps.rgba(dst).unwrap();
    assert_eq!(at(rgba, 8, 0, 0), [3, 3, 3, 255]);
    assert_eq!(at(rgba, 8, 1, 0), [0; 4]);
}

#[test]
fn transfer_is_clipped_to_the_destination() {
    let mut bitmaps = Bitmaps::new();
    let source = pattern(3, 3, 5);
    let src = bitmaps
        .create_from_rgba(3, 3, source.clone(), &defaults())
        .unwrap();
    let dst = bitmaps.create(2, 2, &defaults()).unwrap();
    bitmaps
        .blt(dst, -2, -1, src, Rect::new(0, 0, 3, 3), 255)
        .unwrap();
    let mut expected = vec![0; 16];
    naive_blt(
        &mut expected,
        (2, 2),
        -2,
        -1,
        Some(&source),
        (3, 3),
        Rect::new(0, 0, 3, 3),
        255,
    );
    assert_eq!(bitmaps.rgba(dst).unwrap(), expected);
    // Only source column 2 lands inside, on destination column 0.
    assert_eq!(at(&expected, 2, 1, 0), [0; 4]);
    assert_ne!(at(&expected, 2, 0, 0), [0; 4]);
}

#[test]
fn transfer_of_an_empty_rectangle_draws_nothing() {
    let mut bitmaps = Bitmaps::new();
    let src = bitmaps
        .create_from_rgba(3, 3, pattern(3, 3, 5), &defaults())
        .unwrap();
    let dst = bitmaps.create(3, 3, &defaults()).unwrap();
    for rect in [
        Rect::new(0, 0, 0, 3),
        Rect::new(0, 0, 3, 0),
        Rect::new(0, 0, -2, 0),
        // Mirrored from the left edge of the source: every column lies outside.
        Rect::new(0, 0, i32::MIN, 3),
    ] {
        bitmaps.blt(dst, 0, 0, src, rect, 255).unwrap();
    }
    assert_eq!(bitmaps.rgba(dst).unwrap(), [0; 36]);
}

#[test]
fn transfer_with_a_negative_size_mirrors() {
    let mut bitmaps = Bitmaps::new();
    let source = pattern(4, 2, 5);
    let src = bitmaps
        .create_from_rgba(4, 2, source.clone(), &defaults())
        .unwrap();
    let dst = bitmaps.create(4, 2, &defaults()).unwrap();
    // Columns 2, 1, 0 of the source, in that order.
    bitmaps
        .blt(dst, 0, 0, src, Rect::new(3, 0, -3, 2), 255)
        .unwrap();
    let rgba = bitmaps.rgba(dst).unwrap().to_vec();
    for y in 0..2 {
        for x in 0..3 {
            assert_eq!(at(&rgba, 4, x, y), at(&source, 4, 2 - x, y), "({x}, {y})");
        }
        assert_eq!(at(&rgba, 4, 3, y), [0; 4]);
    }
    // Rows likewise.
    bitmaps.clear(dst).unwrap();
    bitmaps
        .blt(dst, 0, 0, src, Rect::new(0, 2, 4, -2), 255)
        .unwrap();
    let rgba = bitmaps.rgba(dst).unwrap();
    for y in 0..2 {
        for x in 0..4 {
            assert_eq!(at(rgba, 4, x, y), at(&source, 4, x, 1 - y), "({x}, {y})");
        }
    }
}

#[test]
fn transfer_within_one_bitmap_reads_the_live_pixels() {
    let original = pattern(4, 4, 31);
    for (x, y) in [(1, 1), (-1, -1), (1, 0), (0, 1), (2, -1), (0, 0)] {
        for opacity in [255, 128] {
            let mut bitmaps = Bitmaps::new();
            let id = bitmaps
                .create_from_rgba(4, 4, original.clone(), &defaults())
                .unwrap();
            let rect = Rect::new(0, 0, 3, 3);
            bitmaps.blt(id, x, y, id, rect, opacity).unwrap();
            let mut expected = original.clone();
            naive_blt(&mut expected, (4, 4), x, y, None, (4, 4), rect, opacity);
            assert_eq!(bitmaps.rgba(id).unwrap(), expected, "to ({x}, {y})");
        }
    }
}

// Stretched transfer.

#[test]
fn same_size_stretch_equals_a_block_transfer() {
    let mut bitmaps = Bitmaps::new();
    let src = bitmaps
        .create_from_rgba(4, 3, pattern(4, 3, 5), &defaults())
        .unwrap();
    for (position, rect) in [
        ((1, 1), Rect::new(0, 0, 4, 3)),
        ((-1, 2), Rect::new(1, 1, 3, 2)),
        ((3, 0), Rect::new(-1, -1, 4, 4)),
    ] {
        for opacity in [255, 100] {
            let blt = bitmaps
                .create_from_rgba(5, 4, pattern(5, 4, 90), &defaults())
                .unwrap();
            let stretch = bitmaps
                .create_from_rgba(5, 4, pattern(5, 4, 90), &defaults())
                .unwrap();
            let (x, y) = position;
            bitmaps.blt(blt, x, y, src, rect, opacity).unwrap();
            let dst_rect = Rect::new(x, y, rect.width, rect.height);
            bitmaps
                .stretch_blt(stretch, dst_rect, src, rect, opacity)
                .unwrap();
            assert_eq!(bitmaps.rgba(stretch).unwrap(), bitmaps.rgba(blt).unwrap());
            assert_ne!(bitmaps.rgba(blt).unwrap(), pattern(5, 4, 90));
        }
    }
}

#[test]
fn doubling_turns_each_pixel_into_a_block() {
    let mut bitmaps = Bitmaps::new();
    let (a, b) = ([10, 20, 30, 255], [40, 50, 60, 255]);
    let src = bitmaps
        .create_from_rgba(2, 1, [a, b].concat(), &defaults())
        .unwrap();
    let dst = bitmaps.create(4, 2, &defaults()).unwrap();
    bitmaps
        .stretch_blt(dst, Rect::new(0, 0, 4, 2), src, Rect::new(0, 0, 2, 1), 255)
        .unwrap();
    assert_eq!(
        bitmaps.rgba(dst).unwrap(),
        [a, a, b, b, a, a, b, b].concat()
    );
}

#[test]
fn halving_takes_columns_0_and_2() {
    let mut bitmaps = Bitmaps::new();
    let pixels: [Rgba; 4] = [
        [1, 0, 0, 255],
        [2, 0, 0, 255],
        [3, 0, 0, 255],
        [4, 0, 0, 255],
    ];
    let src = bitmaps
        .create_from_rgba(4, 1, pixels.concat(), &defaults())
        .unwrap();
    let dst = bitmaps.create(2, 1, &defaults()).unwrap();
    bitmaps
        .stretch_blt(dst, Rect::new(0, 0, 2, 1), src, Rect::new(0, 0, 4, 1), 255)
        .unwrap();
    assert_eq!(bitmaps.rgba(dst).unwrap(), [pixels[0], pixels[2]].concat());
}

#[test]
fn stretch_with_an_empty_rectangle_draws_nothing() {
    let mut bitmaps = Bitmaps::new();
    let src = bitmaps
        .create_from_rgba(3, 3, pattern(3, 3, 5), &defaults())
        .unwrap();
    let dst = bitmaps.create(3, 3, &defaults()).unwrap();
    let full = Rect::new(0, 0, 3, 3);
    for empty in [
        Rect::new(0, 0, 0, 3),
        Rect::new(0, 0, 3, 0),
        Rect::new(0, 0, -3, 3),
        Rect::new(0, 0, 3, -3),
        Rect::new(0, 0, i32::MIN, i32::MIN),
    ] {
        bitmaps.stretch_blt(dst, empty, src, full, 255).unwrap();
        bitmaps.stretch_blt(dst, full, src, empty, 255).unwrap();
        bitmaps.stretch_blt(dst, empty, src, empty, 255).unwrap();
    }
    assert_eq!(bitmaps.rgba(dst).unwrap(), [0; 36]);
}

#[test]
fn stretch_clips_like_a_block_transfer() {
    let mut bitmaps = Bitmaps::new();
    let pixels: [Rgba; 2] = [[1, 0, 0, 255], [2, 0, 0, 255]];
    let src = bitmaps
        .create_from_rgba(2, 1, pixels.concat(), &defaults())
        .unwrap();
    let dst = bitmaps.create(3, 1, &defaults()).unwrap();
    // Destination columns -2..6 take source columns 0,0,0,0,1,1,1,1; 0..3 are visible.
    bitmaps
        .stretch_blt(dst, Rect::new(-2, 0, 8, 1), src, Rect::new(0, 0, 2, 1), 255)
        .unwrap();
    assert_eq!(
        bitmaps.rgba(dst).unwrap(),
        [pixels[0], pixels[0], pixels[1]].concat()
    );

    // A source rectangle reaching past the source: its columns 2 and 3 do not exist, so
    // the right half of the destination rectangle stays empty.
    bitmaps.clear(dst).unwrap();
    bitmaps
        .stretch_blt(dst, Rect::new(0, 0, 2, 1), src, Rect::new(1, 0, 2, 1), 255)
        .unwrap();
    assert_eq!(
        bitmaps.rgba(dst).unwrap(),
        [pixels[1], [0; 4], [0; 4]].concat()
    );
}

/// The bitmap of the stretch probe: pixel (x, y) holds red `x mod 256`, green
/// `y + x / 256` and alpha 255, so that red and green name the source column and row.
fn probe_source(bitmaps: &mut Bitmaps, width: u32, height: u32) -> BitmapId {
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rgba.extend([x as u8, (y + x / 256) as u8, 0, 255]);
        }
    }
    bitmaps
        .create_from_rgba(width, height, rgba, &defaults())
        .unwrap()
}

/// The cells of a sample line: a source index, or `None` for an untouched pixel.
fn cells(line: &str) -> Vec<Option<u32>> {
    line.split_whitespace()
        .map(|cell| (cell != ".").then(|| cell.parse().unwrap()))
        .collect()
}

/// Asserts every pixel of `id` against the measured source column and row of its position;
/// a position without either is untouched.
fn assert_sampled(
    bitmaps: &Bitmaps,
    id: BitmapId,
    columns: &[Option<u32>],
    rows: &[Option<u32>],
    case: &str,
) {
    let rgba = bitmaps.rgba(id).unwrap();
    let width = bitmaps.width(id).unwrap();
    assert_eq!(columns.len(), width as usize, "{case}");
    assert_eq!(rows.len(), bitmaps.height(id).unwrap() as usize, "{case}");
    for (y, row) in rows.iter().enumerate() {
        for (x, column) in columns.iter().enumerate() {
            let expected = match (column, row) {
                (Some(column), Some(row)) => [*column as u8, (row + column / 256) as u8, 0, 255],
                _ => [0; 4],
            };
            assert_eq!(
                at(rgba, width, x as u32, y as u32),
                expected,
                "{case} ({x}, {y})"
            );
        }
    }
}

/// Every committed stretch sample measured on the original runtime: whole bitmaps, long
/// rows, source rectangles that leave the source, mirrored ones and clipped destinations.
/// The clipped cases were measured along both axes with the same result, so each is
/// evaluated along both.
#[test]
fn stretch_reproduces_the_measured_samples() {
    let samples = include_str!("../../../vkrgss-pixel/tests/data/bitmap-stretch-samples.txt");
    let mut lines = samples.lines().filter(|line| !line.starts_with('#'));
    let mut counts = [0; 4];
    let straight: Vec<Option<u32>> = (0..16).map(Some).collect();
    while let Some(case) = lines.next() {
        let n: Vec<i32> = case
            .split_whitespace()
            .filter_map(|word| word.parse().ok())
            .collect();
        let mut bitmaps = Bitmaps::new();
        match case.split_whitespace().next().unwrap() {
            "full" => {
                let (sw, sh, dw, dh) = (n[0], n[1], n[2], n[3]);
                let columns = cells(lines.next().unwrap());
                let rows = cells(lines.next().unwrap());
                let src = probe_source(&mut bitmaps, sw as u32, sh as u32);
                let dst = bitmaps.create(dw as u32, dh as u32, &defaults()).unwrap();
                let (to, from) = (Rect::new(0, 0, dw, dh), Rect::new(0, 0, sw, sh));
                bitmaps.stretch_blt(dst, to, src, from, 255).unwrap();
                assert_sampled(&bitmaps, dst, &columns, &rows, case);
                counts[0] += 1;
            }
            "big" => {
                let (sw, dw) = (n[0], n[1]);
                let columns = cells(lines.next().unwrap());
                let src = probe_source(&mut bitmaps, sw as u32, 1);
                let dst = bitmaps.create(dw as u32, 1, &defaults()).unwrap();
                let (to, from) = (Rect::new(0, 0, dw, 1), Rect::new(0, 0, sw, 1));
                bitmaps.stretch_blt(dst, to, src, from, 255).unwrap();
                assert_sampled(&bitmaps, dst, &columns, &[Some(0)], case);
                counts[1] += 1;
            }
            "clipx" => {
                let (sx, sw, dx, dw) = (n[0], n[1], n[2], n[3]);
                let measured = cells(lines.next().unwrap());
                let src = probe_source(&mut bitmaps, 16, 16);
                let wide = bitmaps.create(48, 16, &defaults()).unwrap();
                let (to, from) = (Rect::new(dx, 0, dw, 16), Rect::new(sx, 0, sw, 16));
                bitmaps.stretch_blt(wide, to, src, from, 255).unwrap();
                assert_sampled(&bitmaps, wide, &measured, &straight, case);
                let tall = bitmaps.create(16, 48, &defaults()).unwrap();
                let (to, from) = (Rect::new(0, dx, 16, dw), Rect::new(0, sx, 16, sw));
                bitmaps.stretch_blt(tall, to, src, from, 255).unwrap();
                assert_sampled(&bitmaps, tall, &straight, &measured, case);
                counts[2] += 1;
            }
            "blt" => {
                let (sx, sw, dx) = (n[0], n[1], n[2]);
                let measured = cells(lines.next().unwrap());
                let src = probe_source(&mut bitmaps, 16, 16);
                let wide = bitmaps.create(48, 16, &defaults()).unwrap();
                bitmaps
                    .blt(wide, dx, 0, src, Rect::new(sx, 0, sw, 16), 255)
                    .unwrap();
                assert_sampled(&bitmaps, wide, &measured, &straight, case);
                let tall = bitmaps.create(16, 48, &defaults()).unwrap();
                bitmaps
                    .blt(tall, 0, dx, src, Rect::new(0, sx, 16, sw), 255)
                    .unwrap();
                assert_sampled(&bitmaps, tall, &straight, &measured, case);
                counts[3] += 1;
            }
            _ => panic!("{case}"),
        }
    }
    assert!(counts[0] >= 30, "{counts:?}");
    assert_eq!(counts[1..], [3, 26, 5]);
}

#[test]
fn stretch_onto_a_negative_destination_size_draws_nothing() {
    let mut bitmaps = Bitmaps::new();
    let src = probe_source(&mut bitmaps, 16, 16);
    let dst = bitmaps.create(48, 48, &defaults()).unwrap();
    let from = Rect::new(0, 0, 16, 16);
    for to in [Rect::new(2, 2, -10, 10), Rect::new(2, 2, 10, -10)] {
        bitmaps.stretch_blt(dst, to, src, from, 255).unwrap();
    }
    assert!(bitmaps.rgba(dst).unwrap().iter().all(|&byte| byte == 0));
}

/// `blt` and `stretch_blt` blend with the formula for a destination with alpha: a selection
/// of the measured samples through the bitmap operations.
#[test]
fn transfers_blend_as_the_measured_samples() {
    let samples = include_str!("../../../vkrgss-pixel/tests/data/bitmap-blt-samples.txt");
    for line in samples.lines().filter(|line| !line.starts_with('#')) {
        let n: Vec<u8> = line
            .split_whitespace()
            .filter_map(|word| word.parse().ok())
            .collect();
        let (target, source, opacity, result) = (&n[0..4], &n[4..8], n[8], &n[9..13]);
        let mut bitmaps = Bitmaps::new();
        let src = bitmaps
            .create_from_rgba(1, 1, source.to_vec(), &defaults())
            .unwrap();
        let one = Rect::new(0, 0, 1, 1);
        let dst = bitmaps
            .create_from_rgba(2, 2, target.repeat(4), &defaults())
            .unwrap();
        bitmaps.blt(dst, 0, 0, src, one, opacity).unwrap();
        assert_eq!(at(bitmaps.rgba(dst).unwrap(), 2, 0, 0), result, "{line}");
        assert_eq!(at(bitmaps.rgba(dst).unwrap(), 2, 1, 0), target, "{line}");
        // Stretched over all four pixels.
        let all = Rect::new(0, 0, 2, 2);
        bitmaps.stretch_blt(dst, all, src, one, opacity).unwrap();
        assert_eq!(at(bitmaps.rgba(dst).unwrap(), 2, 1, 1), result, "{line}");
    }
}

/// The bitmap of the clip probe: pixel (x, y) of 16 by 16 holds (x * 10 + 5, y * 10 + 5, 77,
/// 255).
fn clip_source(bitmaps: &mut Bitmaps) -> BitmapId {
    let mut rgba = Vec::new();
    for y in 0..16u8 {
        for x in 0..16u8 {
            rgba.extend([x * 10 + 5, y * 10 + 5, 77, 255]);
        }
    }
    bitmaps.create_from_rgba(16, 16, rgba, &defaults()).unwrap()
}

/// Asserts every pixel of `id` against the measured source column and row of its position
/// in the bitmap of [`clip_source`]; a position without either is untouched.
fn assert_clipped(
    bitmaps: &Bitmaps,
    id: BitmapId,
    columns: &[Option<u32>],
    rows: &[Option<u32>],
    case: &str,
) {
    let rgba = bitmaps.rgba(id).unwrap();
    let width = bitmaps.width(id).unwrap();
    assert_eq!(columns.len(), width as usize, "{case}");
    assert_eq!(rows.len(), bitmaps.height(id).unwrap() as usize, "{case}");
    for (y, row) in rows.iter().enumerate() {
        for (x, column) in columns.iter().enumerate() {
            let expected = match (column, row) {
                (Some(column), Some(row)) => [*column as u8 * 10 + 5, *row as u8 * 10 + 5, 77, 255],
                _ => [0; 4],
            };
            assert_eq!(
                at(rgba, width, x as u32, y as u32),
                expected,
                "{case} ({x}, {y})"
            );
        }
    }
}

/// Pixels written as `rrggbbaa` words.
fn hex_pixels(text: &str) -> Vec<u8> {
    text.split_whitespace()
        .flat_map(|word| u32::from_str_radix(word, 16).unwrap().to_be_bytes())
        .collect()
}

/// Every committed sample of `bitmap-transfer-samples.txt`, measured on the originals of XP
/// and VX with the same bytes: the opacity argument, mirrored transfers of equal sizes whose
/// source rectangle leaves the source along each axis and along both, and transfers from a
/// disposed source.
#[test]
fn transfers_reproduce_the_samples_measured_on_both_originals() {
    let samples = include_str!("../../../vkrgss-pixel/tests/data/bitmap-transfer-samples.txt");
    let mut bitmaps = Bitmaps::new();
    let clip = clip_source(&mut bitmaps);
    let gone = bitmaps.create(8, 6, &defaults()).unwrap();
    bitmaps.dispose(gone);
    let mut counts = [0; 4];
    for line in samples.lines().filter(|line| !line.starts_with('#')) {
        let parts: Vec<&str> = line.split('|').map(str::trim).collect();
        let words: Vec<&str> = parts[0].split(' ').collect();
        let stretch = words[1] == "stretch";
        match words[0] {
            "opacity" => {
                // A Float is truncated before it gets here.
                let opacity = words[2].parse::<f64>().unwrap() as i32;
                let src = bitmaps
                    .create_from_rgba(1, 1, vec![200, 100, 50, 200], &defaults())
                    .unwrap();
                let before = [10, 20, 30, 255, 10, 20, 30, 90].to_vec();
                let dst = bitmaps.create_from_rgba(2, 1, before, &defaults()).unwrap();
                let one = Rect::new(0, 0, 1, 1);
                if stretch {
                    let both = Rect::new(0, 0, 2, 1);
                    bitmaps.stretch_blt(dst, both, src, one, opacity).unwrap();
                } else {
                    bitmaps.blt(dst, 0, 0, src, one, opacity).unwrap();
                    bitmaps.blt(dst, 1, 0, src, one, opacity).unwrap();
                }
                let measured: Vec<u8> = parts[1].split(' ').map(|n| n.parse().unwrap()).collect();
                assert_eq!(bitmaps.rgba(dst).unwrap(), measured, "{line}");
                bitmaps.dispose(src);
                bitmaps.dispose(dst);
                counts[0] += 1;
            }
            "mirror" => {
                let n: Vec<i32> = words[2..].iter().map(|n| n.parse().unwrap()).collect();
                let [axis, pos, src_pos, src_len] = n[..] else {
                    panic!("{line}");
                };
                // Along the axis the measured cells; across it source lines 1 to 3.
                let along = cells(parts[1]);
                let across = [Some(1), Some(2), Some(3)];
                let (dst, dst_rect, src_rect) = if axis == 0 {
                    (
                        bitmaps.create(48, 3, &defaults()).unwrap(),
                        Rect::new(pos, 0, src_len.abs(), 3),
                        Rect::new(src_pos, 1, src_len, 3),
                    )
                } else {
                    (
                        bitmaps.create(3, 48, &defaults()).unwrap(),
                        Rect::new(0, pos, 3, src_len.abs()),
                        Rect::new(1, src_pos, 3, src_len),
                    )
                };
                if stretch {
                    bitmaps
                        .stretch_blt(dst, dst_rect, clip, src_rect, 255)
                        .unwrap();
                } else {
                    bitmaps
                        .blt(dst, dst_rect.x, dst_rect.y, clip, src_rect, 255)
                        .unwrap();
                }
                if axis == 0 {
                    assert_clipped(&bitmaps, dst, &along, &across, line);
                } else {
                    assert_clipped(&bitmaps, dst, &across, &along, line);
                }
                bitmaps.dispose(dst);
                counts[1] += 1;
            }
            "both" => {
                let n: Vec<i32> = words[1..].iter().map(|n| n.parse().unwrap()).collect();
                let dst = bitmaps.create(32, 32, &defaults()).unwrap();
                let src_rect = Rect::new(n[2], n[3], n[4], n[5]);
                bitmaps.blt(dst, n[0], n[1], clip, src_rect, 255).unwrap();
                assert_clipped(&bitmaps, dst, &cells(parts[1]), &cells(parts[2]), line);
                bitmaps.dispose(dst);
                counts[2] += 1;
            }
            "disposed" => {
                let n: Vec<i32> = words[2..].iter().map(|n| n.parse().unwrap()).collect();
                let dst = bitmaps
                    .create_from_rgba(8, 6, hex_pixels(parts[1]), &defaults())
                    .unwrap();
                let transferred = if stretch {
                    let dst_rect = Rect::new(n[0], n[1], n[2], n[3]);
                    let src_rect = Rect::new(n[4], n[5], n[6], n[7]);
                    bitmaps.stretch_blt(dst, dst_rect, gone, src_rect, n[8])
                } else {
                    let src_rect = Rect::new(n[2], n[3], n[4], n[5]);
                    bitmaps.blt(dst, n[0], n[1], gone, src_rect, n[6])
                };
                assert_eq!(transferred, Ok(()), "{line}");
                assert_eq!(bitmaps.rgba(dst).unwrap(), hex_pixels(parts[2]), "{line}");
                bitmaps.dispose(dst);
                counts[3] += 1;
            }
            _ => panic!("{line}"),
        }
    }
    assert_eq!(counts, [78, 300, 8, 9]);
}

// Hue.

#[test]
fn only_an_angle_of_0_changes_no_pixel() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps
        .create_from_rgba(4, 4, pattern(4, 4, 3), &defaults())
        .unwrap();
    bitmaps.hue_change(id, 0).unwrap();
    assert_eq!(bitmaps.rgba(id).unwrap(), pattern(4, 4, 3));

    // A full turn goes through the arithmetic, as measured on the original runtime.
    let colours = [[255, 0, 0, 255], [81, 81, 0, 255], [51, 153, 204, 255]].concat();
    let id = bitmaps
        .create_from_rgba(3, 1, colours, &defaults())
        .unwrap();
    bitmaps.hue_change(id, 360).unwrap();
    assert_eq!(
        bitmaps.rgba(id).unwrap(),
        [[254, 0, 0, 255], [80, 80, 0, 255], [51, 152, 203, 255]].concat()
    );
}

#[test]
fn primary_colours_rotate_into_each_other_up_to_the_rounding() {
    let mut bitmaps = Bitmaps::new();
    let red = [255, 0, 0, 255].repeat(2);
    let first = bitmaps
        .create_from_rgba(2, 1, red.clone(), &defaults())
        .unwrap();
    let second = bitmaps.create_from_rgba(2, 1, red, &defaults()).unwrap();
    bitmaps.hue_change(first, 120).unwrap();
    bitmaps.hue_change(second, 240).unwrap();
    assert_eq!(bitmaps.rgba(first).unwrap(), [0, 254, 0, 255].repeat(2));
    assert_eq!(bitmaps.rgba(second).unwrap(), [0, 0, 254, 255].repeat(2));
}

#[test]
fn negative_angles_equal_their_positive_counterparts() {
    let mut bitmaps = Bitmaps::new();
    let negative = bitmaps
        .create_from_rgba(4, 4, pattern(4, 4, 3), &defaults())
        .unwrap();
    let positive = bitmaps
        .create_from_rgba(4, 4, pattern(4, 4, 3), &defaults())
        .unwrap();
    bitmaps.hue_change(negative, -90).unwrap();
    bitmaps.hue_change(positive, 270).unwrap();
    assert_eq!(
        bitmaps.rgba(negative).unwrap(),
        bitmaps.rgba(positive).unwrap()
    );
    assert_ne!(bitmaps.rgba(positive).unwrap(), pattern(4, 4, 3));
}

#[test]
fn grey_is_unaffected() {
    let mut bitmaps = Bitmaps::new();
    let greys = [[0, 0, 0, 255], [128, 128, 128, 7], [255, 255, 255, 255]].concat();
    let id = bitmaps
        .create_from_rgba(3, 1, greys.clone(), &defaults())
        .unwrap();
    for degrees in [1, 59, 90, 180, 271, -45, 360, 720] {
        bitmaps.hue_change(id, degrees).unwrap();
        assert_eq!(bitmaps.rgba(id).unwrap(), greys, "{degrees}");
    }
}

#[test]
fn alpha_is_kept() {
    let mut bitmaps = Bitmaps::new();
    let id = bitmaps
        .create_from_rgba(2, 1, vec![255, 0, 0, 128, 10, 200, 30, 0], &defaults())
        .unwrap();
    bitmaps.hue_change(id, 120).unwrap();
    let rotated = rotate_hue([10, 200, 30, 0], 120);
    assert_eq!(rotated[3], 0);
    assert_ne!(rotated[..3], [10, 200, 30]);
    assert_eq!(
        bitmaps.rgba(id).unwrap(),
        [[0, 254, 0, 128], rotated].concat()
    );
}

proptest! {
    #[test]
    fn blt_matches_the_naive_model(
        (dst_width, dst_height) in (1..7u32, 1..7u32),
        (src_width, src_height) in (1..7u32, 1..7u32),
        (x, y) in (-8..8i32, -8..8i32),
        rect in (-8..8i32, -8..8i32, -3..12i32, -3..12i32),
        opacity in prop_oneof![Just(0u8), Just(255u8), any::<u8>()],
        (dst_seed, src_seed) in any::<(u8, u8)>(),
        same in any::<bool>(),
    ) {
        let rect = Rect::new(rect.0, rect.1, rect.2, rect.3);
        let dst_pixels = pattern(dst_width, dst_height, dst_seed);
        let mut bitmaps = Bitmaps::new();
        let dst = bitmaps
            .create_from_rgba(dst_width, dst_height, dst_pixels.clone(), &defaults())
            .unwrap();
        let mut expected = dst_pixels.clone();
        let size = (dst_width, dst_height);
        if same {
            bitmaps.blt(dst, x, y, dst, rect, opacity).unwrap();
            naive_blt(&mut expected, size, x, y, None, size, rect, opacity);
        } else {
            let src_pixels = pattern(src_width, src_height, src_seed);
            let src = bitmaps
                .create_from_rgba(src_width, src_height, src_pixels.clone(), &defaults())
                .unwrap();
            bitmaps.blt(dst, x, y, src, rect, opacity).unwrap();
            let src_size = (src_width, src_height);
            let read = Some(&src_pixels[..]);
            naive_blt(&mut expected, size, x, y, read, src_size, rect, opacity);
            prop_assert_eq!(bitmaps.rgba(src).unwrap(), &src_pixels[..]);
        }
        prop_assert_eq!(bitmaps.rgba(dst).unwrap(), &expected[..]);
    }
}

/// A bitmap one pixel high with the pixels `a` to `f`, and those pixels.
fn row_a_to_f(bitmaps: &mut Bitmaps) -> (BitmapId, [Rgba; 6]) {
    let pixels = [10, 20, 30, 40, 50, 60].map(|v| [v, v + 1, v + 2, 255]);
    let rgba = pixels.as_flattened().to_vec();
    let id = bitmaps.create_from_rgba(6, 1, rgba, &defaults()).unwrap();
    (id, pixels)
}

#[test]
fn a_transfer_within_one_bitmap_reads_what_it_has_written() {
    let mut bitmaps = Bitmaps::new();
    let (id, [a, b, c, d, e, f]) = row_a_to_f(&mut bitmaps);
    bitmaps
        .blt(id, 1, 0, id, Rect::new(0, 0, 4, 1), 255)
        .unwrap();
    let expected = [a, a, a, a, a, f];
    assert_eq!(bitmaps.rgba(id).unwrap(), expected.as_flattened());

    // To the left every source pixel is read before it is written.
    let (id, _) = row_a_to_f(&mut bitmaps);
    bitmaps
        .blt(id, 0, 0, id, Rect::new(1, 0, 4, 1), 255)
        .unwrap();
    let expected = [b, c, d, e, e, f];
    assert_eq!(bitmaps.rgba(id).unwrap(), expected.as_flattened());

    // A shift by 2 repeats the first two pixels.
    let (id, _) = row_a_to_f(&mut bitmaps);
    bitmaps
        .blt(id, 2, 0, id, Rect::new(0, 0, 4, 1), 255)
        .unwrap();
    let expected = [a, b, a, b, a, b];
    assert_eq!(bitmaps.rgba(id).unwrap(), expected.as_flattened());
}

#[test]
fn a_transfer_within_one_bitmap_downwards_repeats_the_first_row() {
    let mut bitmaps = Bitmaps::new();
    let rows = [10, 20, 30, 40].map(|v| [v, v + 1, v + 2, 255]);
    let rgba = rows.as_flattened().to_vec();
    let id = bitmaps.create_from_rgba(1, 4, rgba, &defaults()).unwrap();
    bitmaps
        .blt(id, 0, 1, id, Rect::new(0, 0, 1, 3), 255)
        .unwrap();
    assert_eq!(bitmaps.rgba(id).unwrap(), [rows[0]; 4].as_flattened());

    let rgba = rows.as_flattened().to_vec();
    let id = bitmaps.create_from_rgba(1, 4, rgba, &defaults()).unwrap();
    bitmaps
        .blt(id, 0, 0, id, Rect::new(0, 1, 1, 3), 255)
        .unwrap();
    let expected = [rows[1], rows[2], rows[3], rows[3]];
    assert_eq!(bitmaps.rgba(id).unwrap(), expected.as_flattened());
}

/// The base bitmap of the self transfer probe: 12 by 12 pixels, opaque or with the alpha
/// pattern.
fn selfblt_base(bitmaps: &mut Bitmaps, alpha: bool) -> BitmapId {
    let mut rgba = Vec::new();
    for y in 0..12u32 {
        for x in 0..12u32 {
            let a = if alpha {
                40 + (x * 7 + y * 13) % 200
            } else {
                255
            };
            rgba.extend([x * 20 + 5, y * 20 + 5, x + y * 12, a].map(|v| v as u8));
        }
    }
    bitmaps.create_from_rgba(12, 12, rgba, &defaults()).unwrap()
}

/// Carries out the call of the self transfer probe that the name line of a case states and
/// returns the bitmap; `None` for a name the probe does not write.
fn selfblt_case(bitmaps: &mut Bitmaps, name: &str) -> Option<BitmapId> {
    let (name, shift) = match name.split_once(" d=") {
        Some((name, shift)) => {
            let (dx, dy) = shift.split_once(',')?;
            (name, (dx.parse().ok()?, dy.parse().ok()?))
        }
        None => (name, (0, 0)),
    };
    let (dx, dy): (i32, i32) = shift;
    let mut words: Vec<&str> = name.split(' ').collect();
    let last = words.pop()?;
    if words == ["base"] {
        return Some(selfblt_base(bitmaps, last == "alpha"));
    }
    let opacity: u8 = last.parse().ok()?;
    let b = selfblt_base(bitmaps, words.pop()? == "alpha");
    match words.join(" ").as_str() {
        "blt" => bitmaps.blt(b, 3 + dx, 3 + dy, b, Rect::new(3, 3, 6, 6), opacity),
        "blt whole" => bitmaps.blt(b, dx, dy, b, Rect::new(0, 0, 12, 12), opacity),
        "blt mirror" => bitmaps.blt(b, 3, 3, b, Rect::new(9, 3, -6, 6), opacity),
        "stretch same" => {
            let dst = Rect::new(3 + dx, 3 + dy, 6, 6);
            bitmaps.stretch_blt(b, dst, b, Rect::new(3, 3, 6, 6), opacity)
        }
        "stretch 2x" => {
            let dst = Rect::new(2 + dx, 2 + dy, 8, 8);
            bitmaps.stretch_blt(b, dst, b, Rect::new(4, 4, 4, 4), opacity)
        }
        "stretch half" => {
            let dst = Rect::new(4 + dx, 4 + dy, 4, 4);
            bitmaps.stretch_blt(b, dst, b, Rect::new(2, 2, 8, 8), opacity)
        }
        _ => return None,
    }
    .unwrap();
    Some(b)
}

/// Provenance: oracle measurement, RGSS103J, self transfer probe: the file the probe wrote
/// in the original runtime, with the calls of the probe.
#[test]
fn transfers_within_one_bitmap_give_the_measured_pixels() {
    let samples = include_str!("../../tests/data/selfblt-samples.txt");
    let lines: Vec<&str> = samples
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect();
    let mut bitmaps = Bitmaps::new();
    let mut transfers = 0;
    for case in lines.chunks(13) {
        let name = case[0];
        let measured: Vec<u8> = case[1..]
            .iter()
            .flat_map(|row| row.split_whitespace())
            .flat_map(|pixel| u32::from_str_radix(pixel, 16).unwrap().to_be_bytes())
            .collect();
        assert_eq!(measured.len(), 12 * 12 * 4, "{name}");
        let id = selfblt_case(&mut bitmaps, name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(bitmaps.rgba(id).unwrap(), measured, "{name}");
        transfers += usize::from(!name.starts_with("base"));
    }
    assert_eq!(transfers, 197);
}
