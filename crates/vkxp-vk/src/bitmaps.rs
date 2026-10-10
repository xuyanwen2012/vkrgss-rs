//! The bitmaps the guest has announced, kept by slot and generation.
//!
//! This is the bookkeeping half of the bitmap table: which slot holds which generation at what
//! size, and whether a delta is acceptable. It has no Vulkan types, so its rules are tested
//! without a device; the compositor mirrors every accepted [`Change`] onto the GPU images. The
//! rules and errors are those of the reference compositor's store, plus the limits a GPU adds.

use std::collections::BTreeMap;

use vkxp_proto::{BitmapDelta, BitmapId, Rect};

use crate::device::BITMAP_SLOTS;
use crate::{BitmapError, Limit, SlotState, VkError};

/// Largest bitmap the reference compositor accepts, in pixels; a larger one is an invalid delta
/// for every backend.
pub(crate) const MAX_BITMAP_PIXELS: u64 = 1 << 28;

/// One occupied slot: the generation living there and its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Entry {
    pub generation: u32,
    pub width: u32,
    pub height: u32,
}

impl Entry {
    /// The rectangle covering the whole bitmap.
    pub(crate) fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width, self.height)
    }
}

/// What an accepted delta means for the stored pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    /// A slot gained a bitmap, to be filled with transparent black. Either side may be 0.
    Created { slot: u32, width: u32, height: u32 },
    /// The pixels inside `rect`, which is not empty and lies inside the bitmap, are replaced.
    Updated { slot: u32, rect: Rect },
    /// A slot lost its bitmap.
    Destroyed { slot: u32 },
    /// The delta is valid and changes no pixel: an update of an empty rectangle.
    Nothing,
}

/// Why a delta was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeltaError {
    /// The delta is invalid for every backend.
    Invalid(BitmapError),
    /// The delta is valid but beyond this backend or device.
    Limit {
        limit: Limit,
        requested: u64,
        maximum: u64,
    },
}

impl DeltaError {
    /// The packet-level error for the delta at `index`.
    pub(crate) fn at(self, index: usize) -> VkError {
        match self {
            DeltaError::Invalid(source) => VkError::InvalidDelta { index, source },
            DeltaError::Limit {
                limit,
                requested,
                maximum,
            } => VkError::Limit {
                limit,
                requested,
                maximum,
            },
        }
    }
}

/// All live bitmaps, driven by [`BitmapDelta`]s.
///
/// A slot is named by the index of a [`BitmapId`]. A reference is valid only while the slot
/// holds the very generation the id names, so an id kept after `Destroy` is rejected even when
/// the slot has a new occupant.
#[derive(Debug, Clone)]
pub(crate) struct BitmapTable {
    slots: BTreeMap<u32, Entry>,
    /// Largest width or height of an image on the device.
    max_dimension: u32,
}

impl BitmapTable {
    pub(crate) fn new(max_dimension: u32) -> BitmapTable {
        BitmapTable {
            slots: BTreeMap::new(),
            max_dimension,
        }
    }

    /// Number of live bitmaps.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }

    /// The live bitmap `id` names, or an invalid-reference error.
    pub(crate) fn get(&self, id: BitmapId) -> Result<&Entry, BitmapError> {
        match self.slots.get(&id.index) {
            Some(entry) if entry.generation == id.generation => Ok(entry),
            other => Err(BitmapError::InvalidReference {
                id,
                slot: match other {
                    Some(entry) => SlotState::Occupied {
                        generation: entry.generation,
                    },
                    None => SlotState::Empty,
                },
            }),
        }
    }

    /// Checks one delta and applies it. On error the table is unchanged.
    ///
    /// The checks every backend makes come first, in the reference's order, so a delta that is
    /// invalid for the reference is reported as invalid here too, never as a limit.
    pub(crate) fn apply(&mut self, delta: &BitmapDelta) -> Result<Change, DeltaError> {
        match delta {
            BitmapDelta::Create { id, width, height } => self.create(*id, *width, *height),
            BitmapDelta::Update { id, rect, rgba } => {
                self.update(*id, *rect, rgba).map_err(DeltaError::Invalid)
            }
            BitmapDelta::Destroy { id } => {
                self.get(*id).map_err(DeltaError::Invalid)?;
                self.slots.remove(&id.index);
                Ok(Change::Destroyed { slot: id.index })
            }
        }
    }

    /// Takes back a `Created` change whose image could not be made.
    pub(crate) fn forget(&mut self, slot: u32) {
        self.slots.remove(&slot);
    }

    fn create(&mut self, id: BitmapId, width: u32, height: u32) -> Result<Change, DeltaError> {
        if let Some(entry) = self.slots.get(&id.index) {
            return Err(DeltaError::Invalid(BitmapError::SlotOccupied {
                id,
                occupant: entry.generation,
            }));
        }
        if u64::from(width) * u64::from(height) > MAX_BITMAP_PIXELS {
            return Err(DeltaError::Invalid(BitmapError::TooLarge {
                id,
                width,
                height,
                limit: MAX_BITMAP_PIXELS,
            }));
        }
        if id.index >= BITMAP_SLOTS {
            return Err(DeltaError::Limit {
                limit: Limit::BitmapSlots,
                requested: u64::from(id.index),
                maximum: u64::from(BITMAP_SLOTS - 1),
            });
        }
        // A bitmap without pixels needs no image, so its other side is not limited.
        let longest = width.max(height);
        if width != 0 && height != 0 && longest > self.max_dimension {
            return Err(DeltaError::Limit {
                limit: Limit::BitmapDimension,
                requested: u64::from(longest),
                maximum: u64::from(self.max_dimension),
            });
        }
        let generation = id.generation;
        self.slots.insert(
            id.index,
            Entry {
                generation,
                width,
                height,
            },
        );
        Ok(Change::Created {
            slot: id.index,
            width,
            height,
        })
    }

    fn update(&mut self, id: BitmapId, rect: Rect, rgba: &[u8]) -> Result<Change, BitmapError> {
        let entry = self.get(id)?;
        if !entry.bounds().contains_rect(&rect) {
            return Err(BitmapError::UpdateOutOfBounds {
                id,
                rect,
                width: entry.width,
                height: entry.height,
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
        Ok(if rect.is_empty() {
            Change::Nothing
        } else {
            Change::Updated {
                slot: id.index,
                rect,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    //! Mirrors the tests of the reference compositor's store, as far as they concern which
    //! deltas are accepted. What the pixels are afterwards is tested against the reference with
    //! a device, in `tests/bitmaps.rs`.

    use super::*;

    const MAX_DIMENSION: u32 = 4096;

    fn table() -> BitmapTable {
        BitmapTable::new(MAX_DIMENSION)
    }

    fn create(id: BitmapId, width: u32, height: u32) -> BitmapDelta {
        BitmapDelta::Create { id, width, height }
    }

    fn fill(id: BitmapId, rect: Rect) -> BitmapDelta {
        BitmapDelta::Update {
            id,
            rect,
            rgba: vec![255; (rect.width * rect.height * 4) as usize],
        }
    }

    fn invalid<T>(error: BitmapError) -> Result<T, DeltaError> {
        Err(DeltaError::Invalid(error))
    }

    #[test]
    fn create_records_the_bitmap() {
        let mut table = table();
        let id = BitmapId::new(0, 0);
        assert_eq!(table.len(), 0);
        assert_eq!(
            table.apply(&create(id, 3, 2)),
            Ok(Change::Created {
                slot: 0,
                width: 3,
                height: 2
            })
        );
        let entry = table.get(id).unwrap();
        assert_eq!((entry.width, entry.height), (3, 2));
        assert_eq!(entry.bounds(), Rect::new(0, 0, 3, 2));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn update_inside_the_bitmap_is_accepted() {
        let mut table = table();
        let id = BitmapId::new(4, 1);
        table.apply(&create(id, 4, 3)).unwrap();
        for rect in [
            Rect::new(0, 0, 4, 3),
            Rect::new(1, 1, 2, 2),
            Rect::new(3, 2, 1, 1),
        ] {
            assert_eq!(
                table.apply(&fill(id, rect)),
                Ok(Change::Updated { slot: 4, rect })
            );
        }
    }

    #[test]
    fn destroy_removes_the_bitmap() {
        let mut table = table();
        let id = BitmapId::new(2, 0);
        table.apply(&create(id, 1, 1)).unwrap();
        assert_eq!(
            table.apply(&BitmapDelta::Destroy { id }),
            Ok(Change::Destroyed { slot: 2 })
        );
        assert_eq!(table.len(), 0);
        assert_eq!(
            table.get(id),
            Err(BitmapError::InvalidReference {
                id,
                slot: SlotState::Empty,
            })
        );
    }

    #[test]
    fn reused_slot_rejects_the_old_generation() {
        let mut table = table();
        let old = BitmapId::new(5, 0);
        let new = BitmapId::new(5, 1);
        table.apply(&create(old, 1, 1)).unwrap();
        table.apply(&BitmapDelta::Destroy { id: old }).unwrap();
        table.apply(&create(new, 2, 2)).unwrap();

        let stale = BitmapError::InvalidReference {
            id: old,
            slot: SlotState::Occupied { generation: 1 },
        };
        assert_eq!(table.get(old), Err(stale.clone()));
        assert_eq!(
            stale.to_string(),
            "invalid bitmap reference 5v0: its slot now holds generation 1"
        );
        // Neither an update nor a destroy through the old id touches the new occupant.
        assert_eq!(
            table.apply(&fill(old, Rect::new(0, 0, 1, 1))),
            invalid(stale.clone())
        );
        assert_eq!(
            table.apply(&BitmapDelta::Destroy { id: old }),
            invalid(stale)
        );
        assert_eq!(table.get(new).unwrap().width, 2);
    }

    #[test]
    fn never_created_bitmap_is_an_invalid_reference() {
        let mut table = table();
        // Beyond the slot capacity as well: a reference there is invalid, not a limit.
        for id in [BitmapId::new(9, 9), BitmapId::new(u32::MAX, 0)] {
            let error = BitmapError::InvalidReference {
                id,
                slot: SlotState::Empty,
            };
            assert_eq!(table.get(id), Err(error.clone()));
            assert_eq!(
                table.apply(&fill(id, Rect::new(0, 0, 1, 1))),
                invalid(error.clone())
            );
            assert_eq!(table.apply(&BitmapDelta::Destroy { id }), invalid(error));
        }
    }

    #[test]
    fn creating_into_an_occupied_slot_is_rejected() {
        let mut table = table();
        table.apply(&create(BitmapId::new(1, 0), 2, 2)).unwrap();
        for generation in [0, 1] {
            let id = BitmapId::new(1, generation);
            assert_eq!(
                table.apply(&create(id, 1, 1)),
                invalid(BitmapError::SlotOccupied { id, occupant: 0 })
            );
        }
        assert_eq!(table.get(BitmapId::new(1, 0)).unwrap().width, 2);
    }

    #[test]
    fn slots_are_independent() {
        let mut table = table();
        let (a, b) = (BitmapId::new(0, 0), BitmapId::new(BITMAP_SLOTS - 1, 7));
        table.apply(&create(a, 1, 1)).unwrap();
        table.apply(&create(b, 1, 1)).unwrap();
        table.apply(&BitmapDelta::Destroy { id: a }).unwrap();
        assert!(table.get(b).is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn update_outside_the_bitmap_is_rejected() {
        let mut table = table();
        let id = BitmapId::new(0, 0);
        table.apply(&create(id, 2, 2)).unwrap();
        for rect in [
            Rect::new(1, 1, 2, 1),
            Rect::new(-1, 0, 1, 1),
            Rect::new(0, 2, 1, 1),
        ] {
            assert_eq!(
                table.apply(&fill(id, rect)),
                invalid(BitmapError::UpdateOutOfBounds {
                    id,
                    rect,
                    width: 2,
                    height: 2,
                })
            );
        }
    }

    #[test]
    fn update_with_wrong_byte_count_is_rejected() {
        let mut table = table();
        let id = BitmapId::new(0, 0);
        table.apply(&create(id, 2, 2)).unwrap();
        let delta = BitmapDelta::Update {
            id,
            rect: Rect::new(0, 0, 2, 1),
            rgba: vec![255; 7],
        };
        assert_eq!(
            table.apply(&delta),
            invalid(BitmapError::UpdateLengthMismatch {
                id,
                expected: 8,
                found: 7,
            })
        );
    }

    #[test]
    fn empty_update_is_accepted_wherever_it_lies() {
        let mut table = table();
        let id = BitmapId::new(0, 0);
        table.apply(&create(id, 2, 2)).unwrap();
        for rect in [Rect::new(1, 1, 0, 0), Rect::new(-7, 900, 0, 3)] {
            let delta = BitmapDelta::Update {
                id,
                rect,
                rgba: vec![],
            };
            assert_eq!(table.apply(&delta), Ok(Change::Nothing));
        }
    }

    #[test]
    fn oversized_bitmap_is_invalid_not_a_limit() {
        let mut table = table();
        // Also beyond the slot capacity and the image size: the reference's verdict wins.
        let id = BitmapId::new(u32::MAX, 0);
        assert_eq!(
            table.apply(&create(id, u32::MAX, u32::MAX)),
            invalid(BitmapError::TooLarge {
                id,
                width: u32::MAX,
                height: u32::MAX,
                limit: MAX_BITMAP_PIXELS,
            })
        );
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn slot_beyond_the_capacity_is_a_limit() {
        let mut table = table();
        assert_eq!(
            table.apply(&create(BitmapId::new(BITMAP_SLOTS, 0), 1, 1)),
            Err(DeltaError::Limit {
                limit: Limit::BitmapSlots,
                requested: 65536,
                maximum: 65535,
            })
        );
        assert_eq!(table.len(), 0);
        assert!(
            table
                .apply(&create(BitmapId::new(BITMAP_SLOTS - 1, 0), 1, 1))
                .is_ok()
        );
    }

    #[test]
    fn bitmap_larger_than_the_device_image_size_is_a_limit() {
        let mut table = table();
        for (index, (width, height)) in [(MAX_DIMENSION + 1, 1), (1, MAX_DIMENSION + 1)]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                table.apply(&create(BitmapId::new(index as u32, 0), width, height)),
                Err(DeltaError::Limit {
                    limit: Limit::BitmapDimension,
                    requested: u64::from(MAX_DIMENSION) + 1,
                    maximum: u64::from(MAX_DIMENSION),
                })
            );
        }
        assert_eq!(table.len(), 0);
        let id = BitmapId::new(0, 0);
        assert!(
            table
                .apply(&create(id, MAX_DIMENSION, MAX_DIMENSION))
                .is_ok()
        );
        // Without pixels there is no image, so nothing to limit.
        assert!(
            table
                .apply(&create(BitmapId::new(1, 0), MAX_DIMENSION + 1, 0))
                .is_ok()
        );
    }

    #[test]
    fn delta_errors_carry_their_index_into_the_packet_error() {
        let id = BitmapId::new(0, 0);
        let source = BitmapError::SlotOccupied { id, occupant: 0 };
        assert_eq!(
            DeltaError::Invalid(source.clone()).at(3),
            VkError::InvalidDelta { index: 3, source }
        );
        let limit = DeltaError::Limit {
            limit: Limit::BitmapSlots,
            requested: 70000,
            maximum: 65535,
        };
        let error = limit.at(0);
        assert_eq!(
            error.to_string(),
            "limit exceeded: bitmap slot index is at most 65535, requested 70000"
        );
    }
}
