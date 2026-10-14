//! The bitmaps the guest has announced, kept by slot and generation.

use std::collections::BTreeMap;

use vkrgss_pixel::Rgba;
use vkrgss_proto::{BitmapDelta, BitmapId, Rect};

use crate::{BitmapError, SlotState};

/// Largest bitmap the store accepts, in pixels (1 GiB of RGBA8).
pub const MAX_BITMAP_PIXELS: u64 = 1 << 28;

/// The pixels of one bitmap: RGBA8 with straight alpha, in row order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitmap {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Bitmap {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// All pixels, four bytes each, in row order.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// The pixel at (`x`, `y`).
    ///
    /// # Panics
    ///
    /// If the position is outside the bitmap.
    pub fn pixel(&self, x: u32, y: u32) -> Rgba {
        assert!(
            x < self.width && y < self.height,
            "pixel outside the bitmap"
        );
        let offset = (y as usize * self.width as usize + x as usize) * 4;
        [
            self.rgba[offset],
            self.rgba[offset + 1],
            self.rgba[offset + 2],
            self.rgba[offset + 3],
        ]
    }

    /// The rectangle covering the whole bitmap.
    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width, self.height)
    }
}

/// One occupied slot: the generation living there and its pixels.
#[derive(Debug, Clone)]
struct Slot {
    generation: u32,
    bitmap: Bitmap,
}

/// All live bitmaps, driven by [`BitmapDelta`]s.
///
/// A slot is named by the index of a [`BitmapId`]. A reference is valid only while the slot
/// holds the very generation the id names, so an id kept after `Destroy` is rejected even when
/// the slot has a new occupant.
#[derive(Debug, Clone, Default)]
pub struct BitmapStore {
    slots: BTreeMap<u32, Slot>,
}

impl BitmapStore {
    pub fn new() -> BitmapStore {
        BitmapStore::default()
    }

    /// Number of live bitmaps.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The live bitmap `id` names, or an invalid-reference error.
    pub fn get(&self, id: BitmapId) -> Result<&Bitmap, BitmapError> {
        match self.slots.get(&id.index) {
            Some(slot) if slot.generation == id.generation => Ok(&slot.bitmap),
            other => Err(invalid_reference(id, other)),
        }
    }

    /// Applies one delta. On error the store is unchanged.
    pub fn apply(&mut self, delta: &BitmapDelta) -> Result<(), BitmapError> {
        match delta {
            BitmapDelta::Create { id, width, height } => self.create(*id, *width, *height),
            BitmapDelta::Update { id, rect, rgba } => self.update(*id, *rect, rgba),
            BitmapDelta::Destroy { id } => self.destroy(*id),
        }
    }

    fn create(&mut self, id: BitmapId, width: u32, height: u32) -> Result<(), BitmapError> {
        if let Some(slot) = self.slots.get(&id.index) {
            return Err(BitmapError::SlotOccupied {
                id,
                occupant: slot.generation,
            });
        }
        let pixels = u64::from(width) * u64::from(height);
        if pixels > MAX_BITMAP_PIXELS {
            return Err(BitmapError::TooLarge {
                id,
                width,
                height,
                limit: MAX_BITMAP_PIXELS,
            });
        }
        let bitmap = Bitmap {
            width,
            height,
            rgba: vec![0; pixels as usize * 4],
        };
        self.slots.insert(
            id.index,
            Slot {
                generation: id.generation,
                bitmap,
            },
        );
        Ok(())
    }

    fn update(&mut self, id: BitmapId, rect: Rect, rgba: &[u8]) -> Result<(), BitmapError> {
        let bitmap = match self.slots.get_mut(&id.index) {
            Some(slot) if slot.generation == id.generation => &mut slot.bitmap,
            other => return Err(invalid_reference(id, other.as_deref())),
        };
        if !bitmap.bounds().contains_rect(&rect) {
            return Err(BitmapError::UpdateOutOfBounds {
                id,
                rect,
                width: bitmap.width,
                height: bitmap.height,
            });
        }
        let expected = u64::from(rect.width) * u64::from(rect.height) * 4;
        if rgba.len() as u64 != expected {
            return Err(BitmapError::UpdateLengthMismatch {
                id,
                expected,
                found: rgba.len(),
            });
        }
        if rect.is_empty() {
            return Ok(());
        }
        // In bounds, so the rectangle's origin is non-negative and everything fits in usize.
        let (left, top) = (rect.x as usize, rect.y as usize);
        let row_len = rect.width as usize * 4;
        for (row, source) in rgba.chunks_exact(row_len).enumerate() {
            let start = ((top + row) * bitmap.width as usize + left) * 4;
            bitmap.rgba[start..start + row_len].copy_from_slice(source);
        }
        Ok(())
    }

    fn destroy(&mut self, id: BitmapId) -> Result<(), BitmapError> {
        self.get(id)?;
        self.slots.remove(&id.index);
        Ok(())
    }
}

fn invalid_reference(id: BitmapId, slot: Option<&Slot>) -> BitmapError {
    BitmapError::InvalidReference {
        id,
        slot: match slot {
            Some(slot) => SlotState::Occupied {
                generation: slot.generation,
            },
            None => SlotState::Empty,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgba = [255, 0, 0, 255];
    const BLUE: Rgba = [0, 0, 255, 128];

    fn create(id: BitmapId, width: u32, height: u32) -> BitmapDelta {
        BitmapDelta::Create { id, width, height }
    }

    fn fill(id: BitmapId, rect: Rect, color: Rgba) -> BitmapDelta {
        BitmapDelta::Update {
            id,
            rect,
            rgba: color.repeat((rect.width * rect.height) as usize),
        }
    }

    #[test]
    fn created_bitmap_is_transparent_black() {
        let mut store = BitmapStore::new();
        let id = BitmapId::new(0, 0);
        assert!(store.is_empty());
        store.apply(&create(id, 3, 2)).unwrap();

        let bitmap = store.get(id).unwrap();
        assert_eq!((bitmap.width(), bitmap.height()), (3, 2));
        assert_eq!(bitmap.rgba(), &[0; 24]);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn update_replaces_only_its_rectangle() {
        let mut store = BitmapStore::new();
        let id = BitmapId::new(4, 1);
        store.apply(&create(id, 4, 3)).unwrap();
        store.apply(&fill(id, Rect::new(0, 0, 4, 3), RED)).unwrap();
        store.apply(&fill(id, Rect::new(1, 1, 2, 2), BLUE)).unwrap();

        let bitmap = store.get(id).unwrap();
        for y in 0..3 {
            for x in 0..4 {
                let inside = (1..3).contains(&x) && (1..3).contains(&y);
                let expected = if inside { BLUE } else { RED };
                assert_eq!(bitmap.pixel(x, y), expected, "({x}, {y})");
            }
        }
    }

    #[test]
    fn update_keeps_row_order() {
        let mut store = BitmapStore::new();
        let id = BitmapId::new(0, 0);
        store.apply(&create(id, 2, 2)).unwrap();
        let rgba: Vec<u8> = (0..16).collect();
        store
            .apply(&BitmapDelta::Update {
                id,
                rect: Rect::new(0, 0, 2, 2),
                rgba,
            })
            .unwrap();
        let bitmap = store.get(id).unwrap();
        assert_eq!(bitmap.pixel(0, 0), [0, 1, 2, 3]);
        assert_eq!(bitmap.pixel(1, 0), [4, 5, 6, 7]);
        assert_eq!(bitmap.pixel(0, 1), [8, 9, 10, 11]);
        assert_eq!(bitmap.pixel(1, 1), [12, 13, 14, 15]);
    }

    #[test]
    fn destroy_removes_the_bitmap() {
        let mut store = BitmapStore::new();
        let id = BitmapId::new(2, 0);
        store.apply(&create(id, 1, 1)).unwrap();
        store.apply(&BitmapDelta::Destroy { id }).unwrap();

        assert!(store.is_empty());
        assert_eq!(
            store.get(id),
            Err(BitmapError::InvalidReference {
                id,
                slot: SlotState::Empty,
            })
        );
    }

    #[test]
    fn reused_slot_rejects_the_old_generation() {
        let mut store = BitmapStore::new();
        let old = BitmapId::new(5, 0);
        let new = BitmapId::new(5, 1);
        store.apply(&create(old, 1, 1)).unwrap();
        store.apply(&fill(old, Rect::new(0, 0, 1, 1), RED)).unwrap();
        store.apply(&BitmapDelta::Destroy { id: old }).unwrap();
        store.apply(&create(new, 1, 1)).unwrap();
        store
            .apply(&fill(new, Rect::new(0, 0, 1, 1), BLUE))
            .unwrap();

        let stale = BitmapError::InvalidReference {
            id: old,
            slot: SlotState::Occupied { generation: 1 },
        };
        assert_eq!(store.get(old), Err(stale.clone()));
        assert_eq!(
            stale.to_string(),
            "invalid bitmap reference 5v0: its slot now holds generation 1"
        );
        // Neither an update nor a destroy through the old id touches the new occupant.
        assert_eq!(
            store.apply(&fill(old, Rect::new(0, 0, 1, 1), RED)),
            Err(stale.clone())
        );
        assert_eq!(store.apply(&BitmapDelta::Destroy { id: old }), Err(stale));
        assert_eq!(store.get(new).unwrap().pixel(0, 0), BLUE);
    }

    #[test]
    fn never_created_bitmap_is_an_invalid_reference() {
        let mut store = BitmapStore::new();
        let id = BitmapId::new(9, 9);
        let error = BitmapError::InvalidReference {
            id,
            slot: SlotState::Empty,
        };
        assert_eq!(store.get(id), Err(error.clone()));
        assert_eq!(
            store.apply(&fill(id, Rect::new(0, 0, 1, 1), RED)),
            Err(error.clone())
        );
        assert_eq!(store.apply(&BitmapDelta::Destroy { id }), Err(error));
    }

    #[test]
    fn creating_into_an_occupied_slot_is_rejected() {
        let mut store = BitmapStore::new();
        store.apply(&create(BitmapId::new(1, 0), 2, 2)).unwrap();
        for generation in [0, 1] {
            let id = BitmapId::new(1, generation);
            assert_eq!(
                store.apply(&create(id, 1, 1)),
                Err(BitmapError::SlotOccupied { id, occupant: 0 })
            );
        }
        assert_eq!(store.get(BitmapId::new(1, 0)).unwrap().width(), 2);
    }

    #[test]
    fn slots_are_independent() {
        let mut store = BitmapStore::new();
        let (a, b) = (BitmapId::new(0, 0), BitmapId::new(u32::MAX, 7));
        store.apply(&create(a, 1, 1)).unwrap();
        store.apply(&create(b, 1, 1)).unwrap();
        store.apply(&fill(b, Rect::new(0, 0, 1, 1), RED)).unwrap();
        store.apply(&BitmapDelta::Destroy { id: a }).unwrap();
        assert_eq!(store.get(b).unwrap().pixel(0, 0), RED);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn update_outside_the_bitmap_is_rejected_and_changes_nothing() {
        let mut store = BitmapStore::new();
        let id = BitmapId::new(0, 0);
        store.apply(&create(id, 2, 2)).unwrap();
        for rect in [
            Rect::new(1, 1, 2, 1),
            Rect::new(-1, 0, 1, 1),
            Rect::new(0, 2, 1, 1),
        ] {
            assert_eq!(
                store.apply(&fill(id, rect, RED)),
                Err(BitmapError::UpdateOutOfBounds {
                    id,
                    rect,
                    width: 2,
                    height: 2,
                })
            );
        }
        assert_eq!(store.get(id).unwrap().rgba(), &[0; 16]);
    }

    #[test]
    fn update_with_wrong_byte_count_is_rejected() {
        let mut store = BitmapStore::new();
        let id = BitmapId::new(0, 0);
        store.apply(&create(id, 2, 2)).unwrap();
        let delta = BitmapDelta::Update {
            id,
            rect: Rect::new(0, 0, 2, 1),
            rgba: vec![255; 7],
        };
        assert_eq!(
            store.apply(&delta),
            Err(BitmapError::UpdateLengthMismatch {
                id,
                expected: 8,
                found: 7,
            })
        );
        assert_eq!(store.get(id).unwrap().rgba(), &[0; 16]);
    }

    #[test]
    fn empty_update_is_accepted() {
        let mut store = BitmapStore::new();
        let id = BitmapId::new(0, 0);
        store.apply(&create(id, 2, 2)).unwrap();
        let delta = BitmapDelta::Update {
            id,
            rect: Rect::new(1, 1, 0, 0),
            rgba: vec![],
        };
        assert_eq!(store.apply(&delta), Ok(()));
    }

    #[test]
    fn oversized_bitmap_is_rejected_without_allocating() {
        let mut store = BitmapStore::new();
        let id = BitmapId::new(0, 0);
        assert_eq!(
            store.apply(&create(id, u32::MAX, u32::MAX)),
            Err(BitmapError::TooLarge {
                id,
                width: u32::MAX,
                height: u32::MAX,
                limit: MAX_BITMAP_PIXELS,
            })
        );
        assert!(store.is_empty());
    }
}
