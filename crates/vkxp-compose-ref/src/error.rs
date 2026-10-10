//! Errors of the bitmap store and the compositor.

use thiserror::Error;
use vkxp_proto::{BitmapId, Rect, Size, TableId};

/// What a bitmap slot holds, reported when a reference does not match it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    /// No bitmap lives in the slot.
    Empty,
    /// A bitmap of another generation lives in the slot.
    Occupied { generation: u32 },
}

impl std::fmt::Display for SlotState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotState::Empty => f.write_str("its slot is empty"),
            SlotState::Occupied { generation } => {
                write!(f, "its slot now holds generation {generation}")
            }
        }
    }
}

/// A bitmap delta or lookup that the store refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BitmapError {
    /// The id names no live bitmap: it was never created, was destroyed, or its slot has been
    /// reused by a later generation.
    #[error("invalid bitmap reference {id}: {slot}")]
    InvalidReference { id: BitmapId, slot: SlotState },
    #[error("cannot create bitmap {id}: its slot still holds generation {occupant}")]
    SlotOccupied { id: BitmapId, occupant: u32 },
    #[error("bitmap {id} of {width}x{height} pixels exceeds the limit of {limit} pixels")]
    TooLarge {
        id: BitmapId,
        width: u32,
        height: u32,
        limit: u64,
    },
    #[error("update of bitmap {id} covers {rect:?}, outside its {width}x{height} pixels")]
    UpdateOutOfBounds {
        id: BitmapId,
        rect: Rect,
        width: u32,
        height: u32,
    },
    #[error("update of bitmap {id} carries {found} bytes, expected {expected}")]
    UpdateLengthMismatch {
        id: BitmapId,
        expected: u64,
        found: usize,
    },
}

/// A table delta or lookup that the store refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TableError {
    /// The id names no table: it was never set, or was destroyed.
    #[error("unknown table {id}")]
    Unknown { id: TableId },
    #[error("table {id} of {xsize}x{ysize}x{zsize} elements carries {found}")]
    LengthMismatch {
        id: TableId,
        xsize: u32,
        ysize: u32,
        zsize: u32,
        found: usize,
    },
}

/// Why a frame packet produced no frame.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ComposeError {
    #[error("op {index} is invalid: {source}")]
    InvalidOpBitmap { index: usize, source: BitmapError },
    #[error("op {index} reads {src:?} of bitmap {bitmap}, outside its {width}x{height} pixels")]
    SourceOutOfBounds {
        index: usize,
        bitmap: BitmapId,
        src: Rect,
        width: u32,
        height: u32,
    },
    #[error("op {index} is invalid: {source}")]
    InvalidOpTable { index: usize, source: TableError },
    #[error("bitmap delta {index} is invalid: {source}")]
    InvalidDelta { index: usize, source: BitmapError },
    #[error("table delta {index} is invalid: {source}")]
    InvalidTableDelta { index: usize, source: TableError },
    #[error(
        "logical size {}x{} is not between 1x1 and {limit}x{limit}",
        size.width,
        size.height
    )]
    InvalidLogicalSize { size: Size, limit: u32 },
}
