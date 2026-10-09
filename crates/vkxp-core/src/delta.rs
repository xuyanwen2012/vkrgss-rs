//! Bitmap deltas: how the pixels of the arena's bitmaps reach the host.
//!
//! Every drawing operation records the bounding rectangle of the pixels it may have changed.
//! [`Bitmaps::deltas`] turns that, for the bitmaps a frame refers to, into the smallest list of
//! [`BitmapDelta`]s that brings a compositor's bitmap store up to date.

use vkxp_proto::{BitmapDelta, BitmapId};

use crate::bitmap::{Area, Bitmap};
use crate::{BitmapError, Bitmaps};

impl Area {
    /// The area as a protocol rectangle.
    fn to_proto(self) -> vkxp_proto::Rect {
        // Bitmap sides are at most `MAX_BITMAP_PIXELS`, which fits both types.
        vkxp_proto::Rect::new(
            self.left as i32,
            self.top as i32,
            (self.right - self.left) as u32,
            (self.bottom - self.top) as u32,
        )
    }
}

/// An update carrying the pixels of `area` as they are now.
fn update(id: BitmapId, bitmap: &Bitmap, area: Area) -> BitmapDelta {
    let rgba = if area == bitmap.bounds() {
        bitmap.rgba.clone()
    } else {
        let row_len = (area.right - area.left) * 4;
        let mut rgba = Vec::with_capacity(row_len * (area.bottom - area.top));
        for y in area.top..area.bottom {
            let start = (y * bitmap.width + area.left) * 4;
            rgba.extend_from_slice(&bitmap.rgba[start..start + row_len]);
        }
        rgba
    };
    BitmapDelta::Update {
        id,
        rect: area.to_proto(),
        rgba,
    }
}

impl Bitmaps {
    /// The bounding rectangle of the pixels that may have changed since the bitmap's last
    /// update was emitted, or `None` when nothing has. A bitmap starts without one: its first
    /// reference uploads it whole.
    pub fn changed_rect(&self, id: BitmapId) -> Result<Option<vkxp_proto::Rect>, BitmapError> {
        Ok(self.get(id)?.changed.map(Area::to_proto))
    }

    /// The bitmap changes the host needs before it draws a frame that refers to `referenced`,
    /// in an order a compositor's bitmap store accepts:
    ///
    /// 1. `Destroy` for every disposed bitmap the host still knows.
    /// 2. For each referenced bitmap the host does not know: `Create`, then `Update` of the
    ///    whole bitmap.
    /// 3. For each referenced bitmap the host knows and that has a changed rectangle: `Update`
    ///    of that rectangle.
    ///
    /// Destroys come first so that a create into a reused slot is never rejected. A bitmap
    /// that is not referenced is not uploaded and keeps its changed rectangle for the request
    /// that does refer to it. Naming a bitmap more than once is the same as naming it once.
    ///
    /// The caller must deliver the result: afterwards the arena takes the host to be up to
    /// date. If `referenced` names a disposed bitmap the call fails with
    /// [`BitmapError::Disposed`], emits nothing and changes nothing.
    pub fn deltas(&mut self, referenced: &[BitmapId]) -> Result<Vec<BitmapDelta>, BitmapError> {
        for &id in referenced {
            self.get(id)?;
        }
        let mut deltas: Vec<BitmapDelta> = self
            .destroyed
            .drain(..)
            .map(|id| BitmapDelta::Destroy { id })
            .collect();

        for &id in referenced {
            let bitmap = self.get_mut(id)?;
            if bitmap.known {
                continue;
            }
            bitmap.known = true;
            bitmap.changed = None;
            deltas.push(BitmapDelta::Create {
                id,
                width: bitmap.width as u32,
                height: bitmap.height as u32,
            });
            deltas.push(update(id, bitmap, bitmap.bounds()));
        }

        for &id in referenced {
            let bitmap = self.get_mut(id)?;
            if let Some(area) = bitmap.changed.take() {
                deltas.push(update(id, bitmap, area));
            }
        }
        Ok(deltas)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vkxp_compose_ref::BitmapStore;
    use vkxp_pixel::Rgba;
    use vkxp_proto::Rect as ProtoRect;

    use super::*;
    use crate::bitmap::testing::*;
    use crate::test_support::CellRasterizer;
    use crate::{Align, Color, Font, Rect};

    /// A bitmap of `width` by `height` with a pattern, already known to the host.
    fn known(bitmaps: &mut Bitmaps, width: u32, height: u32) -> BitmapId {
        let id = bitmaps
            .create_from_rgba(width, height, pattern(width, height, 3), &defaults())
            .unwrap();
        bitmaps.deltas(&[id]).unwrap();
        assert_eq!(bitmaps.changed_rect(id), Ok(None));
        id
    }

    fn changed(bitmaps: &Bitmaps, id: BitmapId) -> Option<ProtoRect> {
        bitmaps.changed_rect(id).unwrap()
    }

    /// Checks that `deltas` are exactly a create of `id` and an update of all its pixels.
    fn assert_full_upload(deltas: &[BitmapDelta], bitmaps: &Bitmaps, id: BitmapId) {
        let (width, height) = (bitmaps.width(id).unwrap(), bitmaps.height(id).unwrap());
        assert_eq!(deltas.len(), 2);
        assert_eq!(deltas[0], BitmapDelta::Create { id, width, height });
        assert!(
            deltas[1]
                == BitmapDelta::Update {
                    id,
                    rect: ProtoRect::new(0, 0, width, height),
                    rgba: bitmaps.rgba(id).unwrap().to_vec(),
                }
        );
    }

    // What each operation marks.

    #[test]
    fn a_new_bitmap_has_no_changed_rectangle() {
        let mut bitmaps = Bitmaps::new();
        let blank = bitmaps.create(4, 4, &defaults()).unwrap();
        let from_pixels = bitmaps
            .create_from_rgba(2, 2, pattern(2, 2, 1), &defaults())
            .unwrap();
        assert_eq!(changed(&bitmaps, blank), None);
        assert_eq!(changed(&bitmaps, from_pixels), None);
        bitmaps.dispose(blank);
        assert_eq!(bitmaps.changed_rect(blank), Err(BitmapError::Disposed));
    }

    #[test]
    fn set_pixel_marks_one_pixel() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 10, 8);
        bitmaps
            .set_pixel(id, 3, 5, Color::rgb(1.0, 2.0, 3.0))
            .unwrap();
        assert_eq!(changed(&bitmaps, id), Some(ProtoRect::new(3, 5, 1, 1)));
    }

    #[test]
    fn set_pixel_outside_marks_nothing() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 10, 8);
        for (x, y) in [(-1, 0), (10, 0), (0, 8), (0, -1)] {
            bitmaps
                .set_pixel(id, x, y, Color::rgb(1.0, 2.0, 3.0))
                .unwrap();
        }
        assert_eq!(changed(&bitmaps, id), None);
    }

    #[test]
    fn changes_accumulate_into_one_bounding_rectangle() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 10, 8);
        bitmaps.set_pixel(id, 7, 1, Color::TRANSPARENT).unwrap();
        bitmaps.set_pixel(id, 2, 6, Color::TRANSPARENT).unwrap();
        assert_eq!(changed(&bitmaps, id), Some(ProtoRect::new(2, 1, 6, 6)));
    }

    #[test]
    fn fill_rect_marks_only_its_visible_part() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 10, 8);
        bitmaps
            .fill_rect(id, Rect::new(6, -2, 100, 5), Color::TRANSPARENT)
            .unwrap();
        assert_eq!(changed(&bitmaps, id), Some(ProtoRect::new(6, 0, 4, 3)));

        let other = known(&mut bitmaps, 10, 8);
        for rect in [
            Rect::new(0, 0, 0, 5),
            Rect::new(0, 0, -5, 5),
            Rect::new(10, 0, 5, 5),
            Rect::new(0, -5, 5, 5),
        ] {
            bitmaps.fill_rect(other, rect, Color::TRANSPARENT).unwrap();
        }
        assert_eq!(changed(&bitmaps, other), None);
    }

    #[test]
    fn clear_marks_the_whole_bitmap() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 10, 8);
        bitmaps.clear(id).unwrap();
        assert_eq!(changed(&bitmaps, id), Some(ProtoRect::new(0, 0, 10, 8)));
    }

    #[test]
    fn blt_marks_where_it_lands_in_the_destination() {
        let mut bitmaps = Bitmaps::new();
        let dst = known(&mut bitmaps, 10, 8);
        let src = known(&mut bitmaps, 4, 4);
        bitmaps
            .blt(dst, 2, 3, src, Rect::new(1, 1, 2, 2), 255)
            .unwrap();
        assert_eq!(changed(&bitmaps, dst), Some(ProtoRect::new(2, 3, 2, 2)));
        assert_eq!(changed(&bitmaps, src), None, "the source is only read");

        // Clipped by the destination's edge and by the source's.
        let dst = known(&mut bitmaps, 10, 8);
        bitmaps
            .blt(dst, 8, -1, src, Rect::new(0, 0, 4, 4), 255)
            .unwrap();
        assert_eq!(changed(&bitmaps, dst), Some(ProtoRect::new(8, 0, 2, 3)));
        let dst = known(&mut bitmaps, 10, 8);
        bitmaps
            .blt(dst, 1, 1, src, Rect::new(-2, 3, 5, 5), 255)
            .unwrap();
        assert_eq!(changed(&bitmaps, dst), Some(ProtoRect::new(3, 1, 3, 1)));
    }

    #[test]
    fn a_transfer_that_draws_nothing_marks_nothing() {
        let mut bitmaps = Bitmaps::new();
        let dst = known(&mut bitmaps, 10, 8);
        let src = known(&mut bitmaps, 4, 4);
        let full = Rect::new(0, 0, 4, 4);
        bitmaps.blt(dst, 2, 3, src, full, 0).unwrap();
        bitmaps.blt(dst, 10, 0, src, full, 255).unwrap();
        bitmaps
            .blt(dst, 0, 0, src, Rect::new(4, 0, 4, 4), 255)
            .unwrap();
        bitmaps
            .blt(dst, 0, 0, src, Rect::new(0, 0, 0, 4), 255)
            .unwrap();
        bitmaps
            .stretch_blt(dst, Rect::new(0, 0, 5, 0), src, full, 255)
            .unwrap();
        bitmaps
            .stretch_blt(dst, Rect::new(0, 0, 5, 5), src, full, 0)
            .unwrap();
        assert_eq!(changed(&bitmaps, dst), None);
    }

    #[test]
    fn blt_within_one_bitmap_marks_the_destination_part() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 10, 8);
        bitmaps
            .blt(id, 5, 4, id, Rect::new(0, 0, 3, 2), 255)
            .unwrap();
        assert_eq!(changed(&bitmaps, id), Some(ProtoRect::new(5, 4, 3, 2)));
    }

    #[test]
    fn stretch_blt_marks_the_visible_destination_rectangle() {
        let mut bitmaps = Bitmaps::new();
        let dst = known(&mut bitmaps, 10, 8);
        let src = known(&mut bitmaps, 4, 4);
        bitmaps
            .stretch_blt(
                dst,
                Rect::new(-3, 2, 8, 20),
                src,
                Rect::new(0, 0, 4, 4),
                255,
            )
            .unwrap();
        assert_eq!(changed(&bitmaps, dst), Some(ProtoRect::new(0, 2, 5, 6)));
        assert_eq!(changed(&bitmaps, src), None);
    }

    #[test]
    fn hue_change_marks_the_whole_bitmap_unless_the_angle_is_0() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 10, 8);
        bitmaps.hue_change(id, 0).unwrap();
        assert_eq!(changed(&bitmaps, id), None);
        bitmaps.hue_change(id, 30).unwrap();
        assert_eq!(changed(&bitmaps, id), Some(ProtoRect::new(0, 0, 10, 8)));
    }

    #[test]
    fn draw_text_marks_the_visible_part_of_the_text() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 40, 30);
        bitmaps.font_mut(id).unwrap().size = 10;
        // "AB" is 10 by 10: right aligned and centred vertically in the rectangle.
        bitmaps
            .draw_text(id, Rect::new(2, 4, 30, 20), "AB", Align::Right, &rasterizer)
            .unwrap();
        assert_eq!(changed(&bitmaps, id), Some(ProtoRect::new(22, 9, 10, 10)));

        // Clipped by the rectangle (width) and by the bitmap (bottom edge).
        let id = known(&mut bitmaps, 40, 30);
        bitmaps.font_mut(id).unwrap().size = 10;
        bitmaps
            .draw_text(id, Rect::new(5, 22, 3, 14), "AB", Align::Left, &rasterizer)
            .unwrap();
        assert_eq!(changed(&bitmaps, id), Some(ProtoRect::new(5, 24, 3, 6)));
    }

    #[test]
    fn text_that_draws_nothing_marks_nothing() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 40, 30);
        let rect = Rect::new(0, 0, 40, 30);
        bitmaps
            .draw_text(id, rect, "", Align::Left, &rasterizer)
            .unwrap();
        bitmaps
            .draw_text(id, Rect::new(0, 0, 0, 30), "AB", Align::Left, &rasterizer)
            .unwrap();
        bitmaps.font_mut(id).unwrap().color.set_alpha(0.0);
        bitmaps
            .draw_text(id, rect, "AB", Align::Left, &rasterizer)
            .unwrap();
        assert_eq!(changed(&bitmaps, id), None);
    }

    #[test]
    fn reading_and_font_changes_mark_nothing() {
        let rasterizer = CellRasterizer;
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 10, 8);
        bitmaps.get_pixel(id, 1, 1).unwrap();
        bitmaps.text_size(id, "AB", &rasterizer).unwrap();
        bitmaps.rgba(id).unwrap();
        bitmaps.set_font(id, Font::new(&defaults())).unwrap();
        bitmaps.font_mut(id).unwrap().bold = true;
        assert_eq!(changed(&bitmaps, id), None);
    }

    // Deltas.

    #[test]
    fn first_reference_creates_and_uploads_everything() {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps
            .create_from_rgba(3, 2, pattern(3, 2, 9), &defaults())
            .unwrap();
        // Drawing before the first reference does not add a separate update.
        bitmaps
            .set_pixel(id, 1, 1, Color::rgb(1.0, 2.0, 3.0))
            .unwrap();
        let deltas = bitmaps.deltas(&[id]).unwrap();
        assert_full_upload(&deltas, &bitmaps, id);
        assert_eq!(changed(&bitmaps, id), None);
    }

    #[test]
    fn unchanged_bitmap_emits_nothing() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 3, 2);
        assert_eq!(bitmaps.deltas(&[id]), Ok(Vec::new()));
        assert_eq!(bitmaps.deltas(&[id, id]), Ok(Vec::new()));
        assert_eq!(bitmaps.deltas(&[]), Ok(Vec::new()));
    }

    #[test]
    fn small_change_emits_one_small_update() {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(100, 100, &defaults()).unwrap();
        bitmaps.deltas(&[id]).unwrap();
        bitmaps
            .set_pixel(id, 40, 70, Color::new(1.0, 2.0, 3.0, 4.0))
            .unwrap();
        let deltas = bitmaps.deltas(&[id]).unwrap();
        assert_eq!(
            deltas,
            [BitmapDelta::Update {
                id,
                rect: ProtoRect::new(40, 70, 1, 1),
                rgba: vec![1, 2, 3, 4],
            }]
        );
        let BitmapDelta::Update { rect, .. } = &deltas[0] else {
            unreachable!();
        };
        assert!(rect.contains(40, 70));
        assert!(u64::from(rect.width) * u64::from(rect.height) < 100 * 100);
        // The change was sent; nothing is left.
        assert_eq!(bitmaps.deltas(&[id]), Ok(Vec::new()));
    }

    #[test]
    fn an_update_carries_the_rows_of_its_rectangle() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 5, 4);
        bitmaps
            .set_pixel(id, 1, 1, Color::new(1.0, 1.0, 1.0, 1.0))
            .unwrap();
        bitmaps
            .set_pixel(id, 3, 2, Color::new(2.0, 2.0, 2.0, 2.0))
            .unwrap();
        let deltas = bitmaps.deltas(&[id]).unwrap();
        let all = bitmaps.rgba(id).unwrap();
        let mut expected = Vec::new();
        for y in 1..3 {
            for x in 1..4 {
                expected.extend_from_slice(&at(all, 5, x, y));
            }
        }
        assert_eq!(
            deltas,
            [BitmapDelta::Update {
                id,
                rect: ProtoRect::new(1, 1, 3, 2),
                rgba: expected,
            }]
        );
    }

    #[test]
    fn change_while_unreferenced_is_kept() {
        let mut bitmaps = Bitmaps::new();
        let id = known(&mut bitmaps, 10, 8);
        let other = known(&mut bitmaps, 2, 2);
        bitmaps
            .set_pixel(id, 4, 4, Color::new(9.0, 8.0, 7.0, 6.0))
            .unwrap();
        assert_eq!(bitmaps.deltas(&[other]), Ok(Vec::new()));
        assert_eq!(bitmaps.deltas(&[]), Ok(Vec::new()));
        // More changes while unreferenced join the pending one.
        bitmaps
            .set_pixel(id, 5, 4, Color::new(1.0, 1.0, 1.0, 1.0))
            .unwrap();
        assert_eq!(
            bitmaps.deltas(&[other, id]),
            Ok(vec![BitmapDelta::Update {
                id,
                rect: ProtoRect::new(4, 4, 2, 1),
                rgba: vec![9, 8, 7, 6, 1, 1, 1, 1],
            }])
        );
    }

    #[test]
    fn an_unreferenced_new_bitmap_is_not_uploaded() {
        let mut bitmaps = Bitmaps::new();
        let id = bitmaps.create(3, 3, &defaults()).unwrap();
        bitmaps.clear(id).unwrap();
        assert_eq!(bitmaps.deltas(&[]), Ok(Vec::new()));
        let deltas = bitmaps.deltas(&[id]).unwrap();
        assert_full_upload(&deltas, &bitmaps, id);
    }

    #[test]
    fn only_disposed_bitmaps_the_host_knows_are_destroyed() {
        let mut bitmaps = Bitmaps::new();
        let seen = known(&mut bitmaps, 2, 2);
        let never_seen = bitmaps.create(2, 2, &defaults()).unwrap();
        bitmaps.dispose(seen);
        bitmaps.dispose(never_seen);
        bitmaps.dispose(seen);
        assert_eq!(
            bitmaps.deltas(&[]),
            Ok(vec![BitmapDelta::Destroy { id: seen }])
        );
        assert_eq!(bitmaps.deltas(&[]), Ok(Vec::new()));
    }

    #[test]
    fn slot_reuse_destroys_before_it_creates() {
        let mut bitmaps = Bitmaps::new();
        let mut store = BitmapStore::new();
        let old = bitmaps
            .create_from_rgba(2, 2, pattern(2, 2, 1), &defaults())
            .unwrap();
        for delta in bitmaps.deltas(&[old]).unwrap() {
            store.apply(&delta).unwrap();
        }
        bitmaps.dispose(old);
        let new = bitmaps
            .create_from_rgba(3, 1, pattern(3, 1, 2), &defaults())
            .unwrap();
        assert_eq!(new.index, old.index);
        assert_ne!(new, old);

        let deltas = bitmaps.deltas(&[new]).unwrap();
        assert_eq!(deltas[0], BitmapDelta::Destroy { id: old });
        assert_full_upload(&deltas[1..], &bitmaps, new);
        for delta in &deltas {
            store.apply(delta).unwrap();
        }
        assert!(store.get(old).is_err());
        assert_eq!(store.get(new).unwrap().rgba(), pattern(3, 1, 2));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn destroys_then_creates_then_updates() {
        let mut bitmaps = Bitmaps::new();
        let first_gone = known(&mut bitmaps, 1, 1);
        let second_gone = known(&mut bitmaps, 1, 1);
        let changed_one = known(&mut bitmaps, 4, 4);
        let unchanged = known(&mut bitmaps, 4, 4);
        bitmaps.dispose(second_gone);
        bitmaps.dispose(first_gone);
        bitmaps
            .set_pixel(changed_one, 0, 0, Color::TRANSPARENT)
            .unwrap();
        let fresh_a = bitmaps.create(2, 1, &defaults()).unwrap();
        let fresh_b = bitmaps.create(1, 2, &defaults()).unwrap();

        let deltas = bitmaps
            .deltas(&[changed_one, fresh_b, unchanged, fresh_a, fresh_b])
            .unwrap();
        assert_eq!(
            deltas,
            [
                BitmapDelta::Destroy { id: second_gone },
                BitmapDelta::Destroy { id: first_gone },
                BitmapDelta::Create {
                    id: fresh_b,
                    width: 1,
                    height: 2,
                },
                BitmapDelta::Update {
                    id: fresh_b,
                    rect: ProtoRect::new(0, 0, 1, 2),
                    rgba: vec![0; 8],
                },
                BitmapDelta::Create {
                    id: fresh_a,
                    width: 2,
                    height: 1,
                },
                BitmapDelta::Update {
                    id: fresh_a,
                    rect: ProtoRect::new(0, 0, 2, 1),
                    rgba: vec![0; 8],
                },
                BitmapDelta::Update {
                    id: changed_one,
                    rect: ProtoRect::new(0, 0, 1, 1),
                    rgba: vec![0; 4],
                },
            ]
        );
    }

    #[test]
    fn referring_to_a_disposed_bitmap_fails_and_changes_nothing() {
        let mut bitmaps = Bitmaps::new();
        let gone = known(&mut bitmaps, 1, 1);
        let live = known(&mut bitmaps, 2, 2);
        let fresh = bitmaps.create(1, 1, &defaults()).unwrap();
        bitmaps.set_pixel(live, 1, 1, Color::TRANSPARENT).unwrap();
        bitmaps.dispose(gone);
        assert_eq!(
            bitmaps.deltas(&[live, fresh, gone]),
            Err(BitmapError::Disposed)
        );
        // Everything is still pending.
        let deltas = bitmaps.deltas(&[live, fresh]).unwrap();
        assert_eq!(deltas.len(), 4);
        assert_eq!(deltas[0], BitmapDelta::Destroy { id: gone });
        assert_full_upload(&deltas[1..3], &bitmaps, fresh);
        assert_eq!(
            deltas[3],
            BitmapDelta::Update {
                id: live,
                rect: ProtoRect::new(1, 1, 1, 1),
                rgba: vec![0; 4],
            }
        );
    }

    // Deltas reproduce the bitmap.

    /// One step of a random run. Bitmaps are picked by an index into the live ones.
    #[derive(Debug, Clone)]
    enum Step {
        Create(u32, u32),
        CreateFromPixels(u32, u32, u8),
        SetPixel(usize, i32, i32, Rgba),
        FillRect(usize, Rect, Rgba),
        Clear(usize),
        Blt(usize, i32, i32, usize, Rect, u8),
        StretchBlt(usize, Rect, usize, Rect, u8),
        HueChange(usize, i32),
        DrawText(usize, Rect, usize, Align),
        Dispose(usize),
        /// Request deltas for the live bitmaps whose bit is set.
        Deltas(u32),
    }

    const TEXTS: [&str; 4] = ["", "A", "A B", "ABCDEFGH"];

    fn any_rect() -> impl Strategy<Value = Rect> {
        (-4..10i32, -4..10i32, -2..12i32, -2..12i32)
            .prop_map(|(x, y, width, height)| Rect::new(x, y, width, height))
    }

    fn any_step() -> impl Strategy<Value = Step> {
        let bitmap = 0..8usize;
        let position = -3..10i32;
        let align = prop_oneof![Just(Align::Left), Just(Align::Center), Just(Align::Right)];
        prop_oneof![
            2 => (1..9u32, 1..9u32).prop_map(|(w, h)| Step::Create(w, h)),
            2 => (1..9u32, 1..9u32, any::<u8>())
                .prop_map(|(w, h, seed)| Step::CreateFromPixels(w, h, seed)),
            3 => (bitmap.clone(), position.clone(), position.clone(), any::<Rgba>())
                .prop_map(|(b, x, y, color)| Step::SetPixel(b, x, y, color)),
            3 => (bitmap.clone(), any_rect(), any::<Rgba>())
                .prop_map(|(b, rect, color)| Step::FillRect(b, rect, color)),
            1 => bitmap.clone().prop_map(Step::Clear),
            3 => (bitmap.clone(), position.clone(), position, bitmap.clone(), any_rect(), any::<u8>())
                .prop_map(|(dst, x, y, src, rect, opacity)| Step::Blt(dst, x, y, src, rect, opacity)),
            3 => (bitmap.clone(), any_rect(), bitmap.clone(), any_rect(), any::<u8>())
                .prop_map(|(dst, dst_rect, src, src_rect, opacity)| {
                    Step::StretchBlt(dst, dst_rect, src, src_rect, opacity)
                }),
            1 => (bitmap.clone(), -400..400i32).prop_map(|(b, degrees)| Step::HueChange(b, degrees)),
            2 => (bitmap.clone(), any_rect(), 0..TEXTS.len(), align)
                .prop_map(|(b, rect, text, align)| Step::DrawText(b, rect, text, align)),
            2 => bitmap.prop_map(Step::Dispose),
            5 => any::<u32>().prop_map(Step::Deltas),
        ]
    }

    /// Requests deltas for `referenced`, applies them to `store` and compares every referenced
    /// bitmap with the arena's.
    fn sync(
        bitmaps: &mut Bitmaps,
        store: &mut BitmapStore,
        referenced: &[BitmapId],
    ) -> Result<(), TestCaseError> {
        let deltas = bitmaps.deltas(referenced).unwrap();
        for delta in &deltas {
            let applied = store.apply(delta);
            prop_assert!(applied.is_ok(), "store rejected a delta: {:?}", applied);
        }
        for &id in referenced {
            let stored = store.get(id);
            prop_assert!(stored.is_ok(), "{} is missing from the store", id);
            let stored = stored.unwrap();
            prop_assert_eq!(stored.width(), bitmaps.width(id).unwrap());
            prop_assert_eq!(stored.height(), bitmaps.height(id).unwrap());
            prop_assert!(stored.rgba() == bitmaps.rgba(id).unwrap(), "{} differs", id);
        }
        Ok(())
    }

    proptest! {
        #[test]
        fn deltas_reproduce_every_referenced_bitmap(
            steps in proptest::collection::vec(any_step(), 0..60),
        ) {
            let rasterizer = CellRasterizer;
            let mut defaults = defaults();
            defaults.size = 6;
            let mut bitmaps = Bitmaps::new();
            let mut store = BitmapStore::new();
            let mut live: Vec<BitmapId> = Vec::new();
            let mut ever: std::collections::HashSet<BitmapId> = Default::default();

            for step in steps {
                // Steps that need a bitmap do nothing while there is none.
                let pick = |index: usize| (!live.is_empty()).then(|| live[index % live.len()]);
                match step {
                    Step::Create(width, height) => {
                        let id = bitmaps.create(width, height, &defaults).unwrap();
                        prop_assert!(ever.insert(id), "{} was handed out twice", id);
                        live.push(id);
                    }
                    Step::CreateFromPixels(width, height, seed) => {
                        let pixels = pattern(width, height, seed);
                        let id = bitmaps
                            .create_from_rgba(width, height, pixels, &defaults)
                            .unwrap();
                        prop_assert!(ever.insert(id), "{} was handed out twice", id);
                        live.push(id);
                    }
                    Step::SetPixel(b, x, y, rgba) => {
                        if let Some(id) = pick(b) {
                            bitmaps.set_pixel(id, x, y, color(rgba)).unwrap();
                        }
                    }
                    Step::FillRect(b, rect, rgba) => {
                        if let Some(id) = pick(b) {
                            bitmaps.fill_rect(id, rect, color(rgba)).unwrap();
                        }
                    }
                    Step::Clear(b) => {
                        if let Some(id) = pick(b) {
                            bitmaps.clear(id).unwrap();
                        }
                    }
                    Step::Blt(dst, x, y, src, rect, opacity) => {
                        if let (Some(dst), Some(src)) = (pick(dst), pick(src)) {
                            bitmaps.blt(dst, x, y, src, rect, opacity).unwrap();
                        }
                    }
                    Step::StretchBlt(dst, dst_rect, src, src_rect, opacity) => {
                        if let (Some(dst), Some(src)) = (pick(dst), pick(src)) {
                            bitmaps
                                .stretch_blt(dst, dst_rect, src, src_rect, opacity)
                                .unwrap();
                        }
                    }
                    Step::HueChange(b, degrees) => {
                        if let Some(id) = pick(b) {
                            bitmaps.hue_change(id, degrees).unwrap();
                        }
                    }
                    Step::DrawText(b, rect, text, align) => {
                        if let Some(id) = pick(b) {
                            bitmaps
                                .draw_text(id, rect, TEXTS[text], align, &rasterizer)
                                .unwrap();
                        }
                    }
                    Step::Dispose(b) => {
                        if let Some(id) = pick(b) {
                            bitmaps.dispose(id);
                            live.retain(|&other| other != id);
                        }
                    }
                    Step::Deltas(bits) => {
                        let referenced: Vec<BitmapId> = live
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| bits >> (index % 32) & 1 == 1)
                            .map(|(_, &id)| id)
                            .collect();
                        sync(&mut bitmaps, &mut store, &referenced)?;
                    }
                }
            }

            // Referring to everything brings the store to exactly the live bitmaps.
            sync(&mut bitmaps, &mut store, &live)?;
            prop_assert_eq!(store.len(), bitmaps.len());
            prop_assert_eq!(bitmaps.deltas(&live), Ok(Vec::new()));
        }
    }
}
