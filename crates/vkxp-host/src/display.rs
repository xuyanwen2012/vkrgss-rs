//! What a session asks of whatever shows its frames.

use vkxp_proto::{BitmapId, FramePacket, FrameRate};
use vkxp_vk::Mix;

/// The surface a session draws on: a window, an off-screen target, or a recording fake.
///
/// A display has a newest frame (the last packet it rendered), a held frame (what
/// [`hold`](Display::hold) captured, black before that) and a [`Mix`] of them. What it shows
/// is the newest frame alone outside a transition, which is how it starts, and at a step of a
/// transition the integer mix of the two per channel, which at weight 0 is the held frame.
///
/// A mask turns the mix into the masked one of a transition picture: the step's threshold
/// then says up to which level of the mask the newest frame is shown, so it appears first
/// where the mask bitmap is dark.
pub trait Display {
    /// Why a packet was rejected or could not be shown.
    type Error: std::fmt::Display;

    /// Composites the packet as the newest frame and shows the result in the current mix.
    /// On an error the newest frame and the image on screen stay what they were.
    fn render(&mut self, packet: &FramePacket) -> Result<(), Self::Error>;

    /// Captures what the display shows in its current state, the mix of the held and the
    /// newest frame, as the held frame. Outside a transition that is the newest frame; in the
    /// middle of a cross-fade it is the mixed image on screen.
    fn hold(&mut self) -> Result<(), Self::Error>;

    /// Sets the mix. It applies from the next time something is shown.
    fn set_mix(&mut self, mix: Mix);

    /// Sets the bitmap that masks the mix, with the vagueness of its edge in mask levels, or
    /// `None` for the plain mix. It applies from the next time something is shown, and only while the display
    /// has that bitmap; the bitmap may arrive with a packet rendered later.
    fn set_mask(&mut self, mask: Option<(BitmapId, u32)>);

    /// Whether the display has the bitmap `id`, from the packets rendered so far.
    fn has_bitmap(&self, id: BitmapId) -> bool;

    /// Tells the display the frame rate of the packet it is about to render. A display that
    /// can ask for the time a frame appears on screen spaces its frames by the period of
    /// that rate; the others ignore it.
    fn set_frame_rate(&mut self, _rate: FrameRate) {}

    /// The frame about to be rendered does not follow the one before it by a period: the
    /// display starts spacing its frames anew from it.
    fn restart_pacing(&mut self) {}

    /// Shows the current state again without a new frame.
    fn redraw(&mut self) -> Result<(), Self::Error>;

    /// Sets the window title.
    fn set_title(&mut self, title: &str);
}
