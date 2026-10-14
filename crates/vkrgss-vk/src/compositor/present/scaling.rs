//! Where the frames go in a target and how they get there: the letterbox rectangle, the
//! filter, the mix, and the push constants that carry them to `shaders/present.frag`.

use vkrgss_proto::{BitmapId, Rect, Size};

use crate::resources::write_words;
use crate::shared::{
    FLAG_HELD_NEAREST, FLAG_LINEAR, FLAG_MASK, FLAG_MIX, FLAG_NEAREST, FLAG_SAME_GRID,
    FLAG_SRGB_TARGET,
};

/// Bytes of the push constants: for the newest and then the held frame the letterbox origin
/// and size and the frame size, then the mix weight, the flags, the threshold and the
/// vagueness, four bytes per component.
pub(super) const PUSH_CONSTANT_BYTES: usize = 64;

/// What is shown of the held and the newest frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mix {
    /// The newest frame alone: no transition is active.
    Newest,
    /// One step of a transition, with the values of both of its forms.
    Step {
        /// The weight `an` of the newest frame in a cross-fade: per channel the result is
        /// `(old * (256 - an) + new * an) >> 8`.
        weight: u8,
        /// The threshold `T` of a masked transition: the mask level up to which the newest
        /// frame is shown alone.
        threshold: u8,
    },
}

impl Mix {
    /// The held frame alone, where there is no mask.
    pub const HELD: Mix = Mix::Step {
        weight: 0,
        threshold: 0,
    };
}

/// The largest vagueness the shader's arithmetic is given. `Graphics.transition` passes none
/// above it, and the ones above 255 already behave alike.
pub(super) const MAX_VAGUENESS: u32 = 65535;

/// Where a frame of `frame` pixels goes in a window of `window` pixels: scaled by one factor
/// in both directions, the largest at which it still fits, and centred.
///
/// The side that does not fill the window is rounded down to whole pixels. The rectangle is
/// empty when either size has a zero dimension, or when the window is so narrow that the
/// scaled frame has no pixel.
pub fn letterbox(frame: Size, window: Size) -> Rect {
    let (frame_w, frame_h) = (u64::from(frame.width), u64::from(frame.height));
    let (window_w, window_h) = (u64::from(window.width), u64::from(window.height));
    if frame_w == 0 || frame_h == 0 || window_w == 0 || window_h == 0 {
        return Rect::default();
    }
    // The window is at least as wide as the frame's shape: the height is what limits.
    let (width, height) = if frame_w * window_h <= frame_h * window_w {
        (frame_w * window_h / frame_h, window_h)
    } else {
        (window_w, frame_h * window_w / frame_w)
    };
    if width == 0 || height == 0 {
        return Rect::default();
    }
    Rect::new(
        ((window_w - width) / 2) as i32,
        ((window_h - height) / 2) as i32,
        width as u32,
        height as u32,
    )
}

/// How the frame is scaled into its letterbox rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    /// Nearest when the factor is a whole number, sharp bilinear otherwise.
    #[default]
    Auto,
    /// Each output pixel shows the frame pixel under its centre.
    Nearest,
    /// Each output pixel interpolates between the four nearest frame pixels.
    Linear,
    /// Nearest, except in a band one output pixel wide along each frame pixel boundary,
    /// which interpolates.
    Sharp,
}

impl Filter {
    /// Whether a frame of `frame` pixels shown in `rect` has each of its pixels shown as it
    /// is, without interpolation.
    pub(super) fn is_nearest(self, frame: Size, rect: &Rect) -> bool {
        match self {
            Filter::Auto => is_whole_factor(frame, rect),
            Filter::Nearest => true,
            Filter::Linear | Filter::Sharp => false,
        }
    }
}

/// Whether `rect` shows a frame of `frame` pixels at a whole-number factor.
pub(super) fn is_whole_factor(frame: Size, rect: &Rect) -> bool {
    !rect.is_empty()
        && rect.width.is_multiple_of(frame.width)
        && rect.height.is_multiple_of(frame.height)
        && rect.width / frame.width == rect.height / frame.height
}

/// The picture that orders a transition: where it is dark the newest frame appears first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::compositor) struct Mask {
    pub(super) bitmap: BitmapId,
    /// Width of the soft edge in mask levels, at most [`MAX_VAGUENESS`]. 0 is a hard edge.
    pub(super) vagueness: u32,
}

/// The sizes of the two frames a draw reads. They differ when the logical size changed after
/// the held frame was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Sources {
    pub(super) newest: Size,
    pub(super) held: Size,
    /// The vagueness of the mask the draw reads, or `None` for a mix by the one weight.
    pub(super) vagueness: Option<u32>,
}

/// The push constants that put the frames of `sources` into a target of `target` pixels, each
/// in its own letterbox rectangle, or `None` when no pixel of either would be visible.
pub(super) fn push_constants(
    sources: Sources,
    target: Size,
    mix: Mix,
    srgb_target: bool,
    filter: Filter,
) -> Option<[u8; PUSH_CONSTANT_BYTES]> {
    let newest = letterbox(sources.newest, target);
    let held = letterbox(sources.held, target);
    if newest.is_empty() && held.is_empty() {
        return None;
    }
    Some(encode_constants(
        sources,
        newest,
        held,
        mix,
        srgb_target,
        filter,
    ))
}

/// The push constants that stretch frames of one size, that of the newest frame of `sources`,
/// over the whole of a target of `target` pixels, which has to be the letterbox rectangle of
/// such a frame in some window.
/// The draw then gives exactly the pixels that rectangle has in that window.
pub(super) fn filling_constants(
    sources: Sources,
    target: Size,
    mix: Mix,
    filter: Filter,
) -> [u8; PUSH_CONSTANT_BYTES] {
    let sources = Sources {
        held: sources.newest,
        ..sources
    };
    let whole = Rect::from_size(target);
    encode_constants(sources, whole, whole, mix, false, filter)
}

/// The push constants for the frames of `sources` in the rectangles `newest` and `held`.
fn encode_constants(
    sources: Sources,
    newest: Rect,
    held: Rect,
    mix: Mix,
    srgb_target: bool,
    filter: Filter,
) -> [u8; PUSH_CONSTANT_BYTES] {
    let mut flags = 0;
    if filter.is_nearest(sources.newest, &newest) {
        flags |= FLAG_NEAREST;
    }
    if filter.is_nearest(sources.held, &held) {
        flags |= FLAG_HELD_NEAREST;
    }
    if filter == Filter::Linear {
        flags |= FLAG_LINEAR;
    }
    if sources.newest == sources.held {
        flags |= FLAG_SAME_GRID;
    }
    if srgb_target {
        flags |= FLAG_SRGB_TARGET;
    }
    let (weight, threshold) = match mix {
        Mix::Newest => (0, 0),
        Mix::Step { weight, threshold } => {
            flags |= FLAG_MIX;
            if sources.vagueness.is_some() {
                flags |= FLAG_MASK;
            }
            (u32::from(weight), u32::from(threshold))
        }
    };
    let vagueness = sources.vagueness.unwrap_or(0);
    let mut bytes = [0; PUSH_CONSTANT_BYTES];
    write_words(
        &mut bytes,
        &[
            newest.x as u32,
            newest.y as u32,
            newest.width,
            newest.height,
            sources.newest.width,
            sources.newest.height,
            held.x as u32,
            held.y as u32,
            held.width,
            held.height,
            sources.held.width,
            sources.held.height,
            weight,
            flags,
            threshold,
            vagueness,
        ],
    );
    bytes
}

/// How a draw places the frames in its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Placement {
    /// Each frame in the letterbox rectangle of its size, as a window shows them.
    Letterbox,
    /// Both frames, which have the same size, stretched over the whole target.
    Fill,
}
