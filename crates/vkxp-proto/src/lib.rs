//! Plain data shared by both sides: frame packet, op list, audio commands, input snapshot.
//!
//! Side: shared.
//!
//! Everything the guest (script thread) and the host (window, GPU, audio) exchange is defined
//! here. The types are self-contained values: no handles, no references, no floating point.
//! Inside one process they move through channels as they are; [`codec`] gives them a versioned
//! binary form for recordings, test fixtures and a later process split. [`link`] is the pair of
//! endpoints the two threads hold.
//!
//! This is protocol version 6. Over version 1 a [`FramePacket`] carries the guest's
//! [`FrameRate`], a [`Quad`] carries a tone and a colour for its own texels, the messages
//! [`GuestToHost::FrameReset`] and [`HostToGuest::Reset`] exist, and [`Button`] has F9. Over
//! version 2 a [`Quad`] states its [`Sampling`]: the affine transform it always had, or a
//! stretch into a rectangle of screen pixels. Over version 3 the blend modes include
//! [`BlendMode::Window`]. Over version 4 an [`InputSnapshot`] carries the raw state of the
//! Windows virtual keys and the pointer position. Over version 5 a [`FramePacket`] carries
//! [`TableDelta`]s beside its bitmap changes and [`Op::TilemapLayer`] is a specified op.
//! Streams of older versions are rejected.
//!
//! The binary form encodes enum variants by position, so reordering or removing variants, or
//! changing fields, requires a new [`codec::PROTOCOL_VERSION`].

#![forbid(unsafe_code)]

pub mod codec;
pub mod link;
pub mod recording;

mod audio;
mod fixed;
mod frame;
mod input;
mod message;
mod types;

pub use audio::{AudioChannel, AudioCommand};
pub use fixed::Fixed;
pub use frame::{
    BitmapDelta, BlendMode, FramePacket, FrameRate, FrameRateOutOfRange, Op, OpKind, Quad,
    RectEffect, STRETCH_FRAC_BITS, Sampling, TableDelta, TilemapLayer, Transform, stretch_index,
    stretch_step,
};
pub use input::{Button, InputSnapshot};
pub use message::{GuestToHost, HostToGuest};
pub use types::{BitmapId, Color, Rect, Size, TableId, Tone};
