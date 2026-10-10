//! Plain data shared by both sides: frame packet, op list, audio commands, input snapshot.
//!
//! Side: shared.
//!
//! Everything the guest (script thread) and the host (window, GPU, audio) exchange is defined
//! here. The types are self-contained values: no handles, no references, no floating point.
//! Inside one process they move through channels as they are; [`codec`] gives them a versioned
//! binary form for recordings and test fixtures, and [`framing`] puts single messages of that
//! form on a byte stream. [`link`] is the pair of endpoints the two sides hold, with a guest
//! end over a Unix stream socket for a guest in a process of its own. [`tiles`] is the
//! numbering of the tiles of a tilemap, which the guest and every backend read alike.
//!
//! This is protocol version 7. Over version 1 a [`FramePacket`] carries the guest's
//! [`FrameRate`], a [`Quad`] carries a tone and a colour for its own texels, the messages
//! [`GuestToHost::FrameReset`] and [`HostToGuest::Reset`] exist, and [`Button`] has F9. Over
//! version 2 a [`Quad`] states its [`Sampling`]: the affine transform it always had, or a
//! stretch into a rectangle of screen pixels. Over version 3 the blend modes include
//! [`BlendMode::Window`]. Over version 4 an [`InputSnapshot`] carries the raw state of the
//! Windows virtual keys and the pointer position. Over version 5 a [`FramePacket`] carries
//! [`TableDelta`]s beside its bitmap changes and [`Op::TilemapLayer`] is a specified op.
//! Over version 6 the reserved op kind `Plane` is gone: every op kind is one that the backends
//! evaluate.
//! Streams of older versions are rejected.
//!
//! The binary form encodes enum variants by position, so reordering or removing variants, or
//! changing fields, requires a new [`codec::PROTOCOL_VERSION`].

#![forbid(unsafe_code)]

pub mod codec;
pub mod framing;
pub mod link;
pub mod recording;
pub mod tiles;

mod audio;
mod fixed;
mod frame;
mod input;
mod message;
mod types;

pub use audio::{AudioChannel, AudioCommand};
pub use fixed::Fixed;
pub use frame::{
    BitmapDelta, BlendMode, FramePacket, FrameRate, FrameRateOutOfRange, MAX_LOGICAL_DIMENSION, Op,
    OpKind, Quad, RectEffect, STRETCH_FRAC_BITS, Sampling, TableDelta, TilemapLayer, Transform,
    stretch_index, stretch_step,
};
pub use input::{Button, InputSnapshot};
pub use message::{GuestToHost, HostToGuest};
pub use types::{BitmapId, Color, Rect, Size, TableId, Tone};
