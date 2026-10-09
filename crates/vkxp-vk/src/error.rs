//! Errors of the Vulkan backend.
//!
//! The variants that describe an invalid packet correspond one to one to the errors of the CPU
//! reference compositor and carry the same indices and ids, so a caller can treat the two
//! backends alike. The reference's types are not reused: the reference is not a dependency of
//! this crate outside its tests.

use ash::vk;
use thiserror::Error;
use vkxp_proto::{BitmapId, OpKind, Rect, Size};

use crate::DeviceInfo;

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

/// A bitmap delta or lookup that the bitmap table refused.
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

/// A limit of the device or of this backend that a valid packet can exceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Limit {
    /// Index of a bitmap slot, bounded by the size of the slot table.
    BitmapSlots,
    /// Width or height of a bitmap, bounded by the device's 2D image size.
    BitmapDimension,
    /// Width or height of the frame, bounded by the device's 2D image size.
    FrameDimension,
    /// Bytes of per-frame data (op records or tile lists) in one buffer.
    FrameDataBytes,
}

impl std::fmt::Display for Limit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Limit::BitmapSlots => "bitmap slot index",
            Limit::BitmapDimension => "bitmap width or height",
            Limit::FrameDimension => "frame width or height",
            Limit::FrameDataBytes => "per-frame data size in bytes",
        })
    }
}

/// Why the backend could not start, or why a frame packet produced no frame.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum VkError {
    /// The op kind is declared in the protocol but not implemented by this backend.
    #[error("unsupported op kind `{kind}` (op {index})")]
    UnsupportedOp { kind: OpKind, index: usize },
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
    #[error("bitmap delta {index} is invalid: {source}")]
    InvalidDelta { index: usize, source: BitmapError },
    #[error(
        "logical size {}x{} is not between 1x1 and {limit}x{limit}",
        size.width,
        size.height
    )]
    InvalidLogicalSize { size: Size, limit: u32 },
    /// The packet is valid but exceeds what the device or this backend supports.
    #[error("limit exceeded: {limit} is at most {maximum}, requested {requested}")]
    Limit {
        limit: Limit,
        requested: u64,
        maximum: u64,
    },
    #[error(transparent)]
    Device(#[from] DeviceError),
    #[error("Vulkan call `{call}` failed: {result}")]
    Vulkan {
        call: &'static str,
        result: vk::Result,
    },
    #[error("cannot allocate memory for {what}: {message}")]
    Memory { what: &'static str, message: String },
}

impl VkError {
    /// Adapter for `map_err` on the result of a Vulkan call.
    pub(crate) fn vulkan(call: &'static str) -> impl FnOnce(vk::Result) -> VkError {
        move |result| VkError::Vulkan { call, result }
    }
}

/// Why no device could be opened.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DeviceError {
    #[error("cannot load the Vulkan library: {0}")]
    Loader(String),
    #[error("no Vulkan device meets the requirements.{}", list(devices))]
    NoSuitableDevice { devices: Vec<DeviceInfo> },
    #[error("no Vulkan device matches {choice}.{}", list(devices))]
    NotFound {
        choice: String,
        devices: Vec<DeviceInfo>,
    },
    #[error(
        "the Vulkan device chosen by {choice} does not meet the requirements.{}",
        list(devices)
    )]
    Unsuitable {
        choice: String,
        devices: Vec<DeviceInfo>,
    },
}

/// The available devices, one per line, each with what it lacks.
fn list(devices: &[DeviceInfo]) -> String {
    if devices.is_empty() {
        return " No Vulkan device is available.".to_owned();
    }
    let mut text = " Available devices:".to_owned();
    for device in devices {
        text.push_str("\n  ");
        text.push_str(&device.to_string());
    }
    text
}
