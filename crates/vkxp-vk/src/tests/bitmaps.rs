//! Bitmap storage on the device: what the images hold after deltas, compared with the bitmaps
//! of the reference compositor's store.

use vkxp_compose_ref::{BitmapStore, fixtures};
use vkxp_proto::{BitmapDelta, BitmapId, Color, Rect};

use super::harness::TestCompositor;
use crate::{BitmapError, Limit, SlotState, VkError};

fn create(id: BitmapId, width: u32, height: u32) -> BitmapDelta {
    BitmapDelta::Create { id, width, height }
}

fn update(id: BitmapId, rect: Rect, rgba: Vec<u8>) -> BitmapDelta {
    BitmapDelta::Update { id, rect, rgba }
}

/// Applies the deltas to both backends and asserts that every listed bitmap holds the same
/// bytes in both.
fn assert_same_pixels(deltas: &[BitmapDelta], ids: &[BitmapId]) {
    let mut compositor = TestCompositor::new();
    let mut store = BitmapStore::new();
    compositor.apply_bitmaps(deltas).unwrap();
    for delta in deltas {
        store.apply(delta).unwrap();
    }
    for &id in ids {
        let expected = store.get(id).unwrap();
        assert_eq!(compositor.read_bitmap(id).unwrap(), expected.rgba(), "{id}");
    }
}

#[test]
fn created_bitmap_is_transparent_black() {
    let id = BitmapId::new(0, 0);
    let mut compositor = TestCompositor::new();
    compositor.apply_bitmaps(&[create(id, 3, 2)]).unwrap();
    assert_eq!(compositor.read_bitmap(id).unwrap(), vec![0; 24]);
}

#[test]
fn full_upload_reads_back_unchanged() {
    let id = BitmapId::new(3, 0);
    let (width, height) = (37, 21);
    assert_same_pixels(
        &[
            create(id, width, height),
            update(
                id,
                Rect::new(0, 0, width, height),
                fixtures::gradient(width, height),
            ),
        ],
        &[id],
    );
}

#[test]
fn update_keeps_row_order() {
    let id = BitmapId::new(0, 0);
    let mut compositor = TestCompositor::new();
    let rgba: Vec<u8> = (0..16).collect();
    compositor
        .apply_bitmaps(&[
            create(id, 2, 2),
            update(id, Rect::new(0, 0, 2, 2), rgba.clone()),
        ])
        .unwrap();
    assert_eq!(compositor.read_bitmap(id).unwrap(), rgba);
}

/// Several partial updates, overlapping and touching the edges, in one delta list and across
/// lists, on two bitmaps at once.
#[test]
fn partial_updates_match_the_reference_store() {
    let (a, b) = (BitmapId::new(0, 0), BitmapId::new(7, 2));
    let deltas = vec![
        create(a, 48, 40),
        create(b, 5, 64),
        update(
            a,
            Rect::new(0, 0, 48, 40),
            fixtures::checkerboard(
                48,
                40,
                4,
                Color::new(32, 32, 48, 255),
                Color::new(224, 224, 208, 128),
            ),
        ),
        update(a, Rect::new(8, 4, 16, 16), fixtures::gradient(16, 16)),
        update(b, Rect::new(1, 10, 4, 30), fixtures::gradient(4, 30)),
        update(
            a,
            Rect::new(16, 12, 32, 28),
            fixtures::disc(32, Color::new(255, 160, 64, 255))[..32 * 28 * 4].to_vec(),
        ),
        update(a, Rect::new(47, 39, 1, 1), vec![1, 2, 3, 4]),
        update(a, Rect::new(0, 39, 48, 1), fixtures::gradient(48, 1)),
        update(b, Rect::new(0, 0, 1, 64), fixtures::gradient(1, 64)),
    ];
    assert_same_pixels(&deltas, &[a, b]);

    // The same content arriving one delta list at a time.
    let mut compositor = TestCompositor::new();
    let mut store = BitmapStore::new();
    for delta in &deltas {
        compositor
            .apply_bitmaps(std::slice::from_ref(delta))
            .unwrap();
        store.apply(delta).unwrap();
        for id in [a, b] {
            if let Ok(expected) = store.get(id) {
                assert_eq!(compositor.read_bitmap(id).unwrap(), expected.rgba(), "{id}");
            }
        }
    }
}

#[test]
fn reused_slot_holds_only_the_new_bitmap() {
    let (old, new) = (BitmapId::new(5, 0), BitmapId::new(5, 1));
    let mut compositor = TestCompositor::new();
    // Destroyed and recreated inside one delta list, then again across lists.
    compositor
        .apply_bitmaps(&[
            create(old, 2, 2),
            update(old, Rect::new(0, 0, 2, 2), vec![255; 16]),
            BitmapDelta::Destroy { id: old },
            create(new, 3, 1),
            update(new, Rect::new(1, 0, 1, 1), vec![9, 8, 7, 6]),
        ])
        .unwrap();
    assert_eq!(
        compositor.read_bitmap(new).unwrap(),
        [0, 0, 0, 0, 9, 8, 7, 6, 0, 0, 0, 0]
    );
    assert_eq!(
        compositor.read_bitmap(old),
        Err(VkError::InvalidOpBitmap {
            index: 0,
            source: BitmapError::InvalidReference {
                id: old,
                slot: SlotState::Occupied { generation: 1 },
            },
        })
    );

    let newer = BitmapId::new(5, 2);
    compositor
        .apply_bitmaps(&[BitmapDelta::Destroy { id: new }])
        .unwrap();
    compositor.apply_bitmaps(&[create(newer, 1, 1)]).unwrap();
    assert_eq!(compositor.read_bitmap(newer).unwrap(), [0, 0, 0, 0]);
}

#[test]
fn bitmap_without_pixels_is_stored_without_an_image() {
    let id = BitmapId::new(1, 0);
    let mut compositor = TestCompositor::new();
    compositor
        .apply_bitmaps(&[create(id, 0, 9), update(id, Rect::new(0, 3, 0, 2), vec![])])
        .unwrap();
    assert_eq!(compositor.read_bitmap(id).unwrap(), Vec::<u8>::new());
    compositor
        .apply_bitmaps(&[BitmapDelta::Destroy { id }])
        .unwrap();
}

/// The deltas before an invalid one are applied on the device too, and the ones after are not.
#[test]
fn invalid_delta_keeps_earlier_deltas_and_drops_later_ones() {
    let (first, second) = (BitmapId::new(0, 0), BitmapId::new(1, 0));
    let mut compositor = TestCompositor::new();
    let error = compositor
        .apply_bitmaps(&[
            create(first, 2, 1),
            update(first, Rect::new(0, 0, 2, 1), vec![1, 2, 3, 4, 5, 6, 7, 8]),
            update(first, Rect::new(1, 0, 2, 1), vec![0; 8]),
            update(first, Rect::new(0, 0, 1, 1), vec![9; 4]),
            create(second, 1, 1),
        ])
        .unwrap_err();
    assert_eq!(
        error,
        VkError::InvalidDelta {
            index: 2,
            source: BitmapError::UpdateOutOfBounds {
                id: first,
                rect: Rect::new(1, 0, 2, 1),
                width: 2,
                height: 1,
            },
        }
    );
    assert_eq!(
        compositor.read_bitmap(first).unwrap(),
        [1, 2, 3, 4, 5, 6, 7, 8]
    );
    assert!(compositor.read_bitmap(second).is_err());
}

#[test]
fn bitmap_beyond_the_device_image_size_is_a_limit_error() {
    let mut compositor = TestCompositor::new();
    let maximum = compositor.max_image_dimension();
    let id = BitmapId::new(0, 0);
    assert_eq!(
        compositor.apply_bitmaps(&[create(id, maximum + 1, 1)]),
        Err(VkError::Limit {
            limit: Limit::BitmapDimension,
            requested: u64::from(maximum) + 1,
            maximum: u64::from(maximum),
        })
    );
    // The slot is still free, and a bitmap at the limit works.
    compositor.apply_bitmaps(&[create(id, maximum, 1)]).unwrap();
    assert_eq!(
        compositor.read_bitmap(id).unwrap(),
        vec![0; maximum as usize * 4]
    );
}

#[test]
fn bitmap_slot_beyond_the_table_is_a_limit_error() {
    let mut compositor = TestCompositor::new();
    assert_eq!(
        compositor.apply_bitmaps(&[create(BitmapId::new(65536, 0), 1, 1)]),
        Err(VkError::Limit {
            limit: Limit::BitmapSlots,
            requested: 65536,
            maximum: 65535,
        })
    );
    let last = BitmapId::new(65535, 0);
    compositor
        .apply_bitmaps(&[
            create(last, 1, 1),
            update(last, Rect::new(0, 0, 1, 1), vec![4; 4]),
        ])
        .unwrap();
    assert_eq!(compositor.read_bitmap(last).unwrap(), [4; 4]);
}
