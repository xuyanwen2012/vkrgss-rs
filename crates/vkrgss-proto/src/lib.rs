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
//! numbering of the tiles of a tilemap of RGSS1, which the guest and every backend read
//! alike, [`tiles_rgss2`] that of RGSS2, and [`tiles_rgss3`] what that of RGSS3 adds to it.
//!
//! This is protocol version 15. Over version 1 a [`FramePacket`] carries the guest's
//! [`FrameRate`], a [`Quad`] carries a tone and a colour for its own texels, the messages
//! [`GuestToHost::FrameReset`] and [`HostToGuest::Reset`] exist, and [`Button`] has F9. Over
//! version 2 a [`Quad`] states its [`Sampling`]: the affine transform it always had, or a
//! stretch into a rectangle of screen pixels. Over version 3 the blend modes include
//! [`BlendMode::Window`]. Over version 4 an [`InputSnapshot`] carries the raw state of the
//! Windows virtual keys and the pointer position. Over version 5 a [`FramePacket`] carries
//! [`TableDelta`]s beside its bitmap changes and [`Op::TilemapLayer`] is a specified op.
//! Over version 6 the reserved op kind `Plane` is gone: every op kind is one that the backends
//! evaluate. Over version 7 the state messages exist: [`HostToGuest::SaveState`],
//! [`HostToGuest::LoadState`] and [`HostToGuest::Resume`], and [`GuestToHost::Attach`],
//! [`GuestToHost::Outcome`], [`GuestToHost::StateSaved`], [`GuestToHost::StateLoaded`] and
//! [`GuestToHost::StateFailed`]. Over version 8 [`GuestToHost::StateFailed`] says whether a
//! save or a load failed. Over version 9 the guest presses and releases keys for the player
//! with [`GuestToHost::InjectKey`] and says with [`GuestToHost::PointerVisible`] whether the
//! pointer is to be seen. Over version 10 a [`FramePacket`] carries a brightness and
//! [`GuestToHost::Transition`] names the [`TransitionRule`] its frames are stepped by. Over
//! version 11 the op [`Op::TilemapPlane`] draws a plane of a tilemap of RGSS2, by the tile
//! numbering of [`tiles_rgss2`]. Over version 12 a [`TilemapPlane`] names its [`PlaneRules`],
//! those of RGSS2 or of RGSS3. Over version 13 [`GuestToHost::Freeze`] names the
//! [`FreezeRule`] by which the frozen picture is kept: as the frame at its own size, or in the
//! buffer of RGSS3. Over version 14 [`AudioCommand::Play`] carries the sample frame at which
//! its sound starts and the time over which the music returns after a music effect, its pitch
//! runs from 1 to 65535, and [`AudioCommand::Volume`] exists. Streams of older versions are
//! rejected.
//!
//! The binary form encodes enum variants by position, so reordering or removing variants, or
//! changing fields, requires a new [`codec::PROTOCOL_VERSION`].

// `unsafe` only in the platform code of the stream transport of the link: the receive that
// does not wait.
#![deny(unsafe_code)]

pub mod codec;
pub mod framing;
pub mod link;
pub mod recording;
pub mod tiles;
pub mod tiles_rgss2;
pub mod tiles_rgss3;

mod audio;
mod fixed;
mod frame;
mod input;
mod message;
mod types;

pub use audio::{AudioChannel, AudioCommand};
pub use fixed::Fixed;
pub use frame::{
    BitmapDelta, BlendMode, FramePacket, FrameRate, FrameRateOutOfRange, MAX_BITMAP_PIXELS,
    MAX_LOGICAL_DIMENSION, Op, OpKind, Plane, PlaneRules, Quad, RectEffect, STRETCH_FRAC_BITS,
    Sampling, TableDelta, TilemapLayer, TilemapPlane, Transform, stretch_index, stretch_step,
};
pub use input::{Button, InputSnapshot};
pub use message::{FreezeRule, GuestToHost, HostToGuest, TransitionRule};
pub use types::{BitmapId, Color, Rect, Size, TableId, Tone};
