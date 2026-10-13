// The numbers that the Rust code of this crate and its shaders must agree on.
//
// This file is the one definition of each. The crate uses it as the module `shared`, and
// `build.rs` includes it too and writes `shared.glsl` from it, which the shaders include. It
// holds only `pub const` items of type `u32` and has no imports, so that both can read it.
// A constant added here must also be named in the list in `build.rs`.

/// Number of 32-bit words in the record of an op.
pub const RECORD_WORDS: u32 = 24;
/// Number of 32-bit words in the record of a tilemap layer.
pub const TILEMAP_WORDS: u32 = 25;

/// Values of the `kind` word of an op record.
pub const OP_CLEAR: u32 = 0;
pub const OP_QUAD: u32 = 1;
pub const OP_RECT_EFFECT: u32 = 2;
pub const OP_TILEMAP: u32 = 3;
pub const OP_TILEMAP_PLANE: u32 = 4;

/// Values of the `blend` word of a quad.
pub const BLEND_NORMAL: u32 = 0;
pub const BLEND_ADD: u32 = 1;
pub const BLEND_SUBTRACT: u32 = 2;
pub const BLEND_WINDOW: u32 = 3;

/// Values of the `sampling` word of a quad.
pub const SAMPLING_AFFINE: u32 = 0;
pub const SAMPLING_STRETCH: u32 = 1;
pub const SAMPLING_STRETCH_MIRRORED: u32 = 2;

/// The slot word of a sheet a tilemap layer goes without.
pub const NO_BITMAP: u32 = u32::MAX;
/// Set in the frame word of an autotile whose sheet holds whole tiles, one per frame of 32
/// pixels.
pub const WHOLE_TILES: u32 = 1 << 31;

// Flags of the fragment shader that presents.

/// Each target pixel takes one texel of the newest frame: the filter is nearest.
pub const FLAG_NEAREST: u32 = 1;
/// The target has an sRGB format, which encodes what is written to it.
pub const FLAG_SRGB_TARGET: u32 = 2;
/// The same as `FLAG_NEAREST` for the held frame.
pub const FLAG_HELD_NEAREST: u32 = 4;
/// Both frames have the same size, and so the same rectangle.
pub const FLAG_SAME_GRID: u32 = 8;
/// The mix is driven per texel by the mask. Only with `FLAG_MIX`.
pub const FLAG_MASK: u32 = 16;
/// Where a frame is not read with nearest, the filter is linear instead of sharp bilinear.
pub const FLAG_LINEAR: u32 = 32;
/// A transition is active: the held frame is mixed in. Without it the newest frame is shown.
pub const FLAG_MIX: u32 = 64;
