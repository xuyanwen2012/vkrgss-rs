//! What a session asks of whatever shows its frames.

use vkxp_proto::FramePacket;

/// The surface a session draws on: a window, an off-screen target, or a recording fake.
///
/// A display has a newest frame (the last packet it rendered), a held frame (what
/// [`hold`](Display::hold) captured, black before that) and a mix weight between them. What it
/// shows is the held frame at weight 0, the newest frame at weight 1 and their per-channel mix
/// in between. It starts at weight 1.
pub trait Display {
    /// Why a packet was rejected or could not be shown.
    type Error: std::fmt::Display;

    /// Composites the packet as the newest frame and shows the result at the current weight.
    /// On an error the newest frame and the image on screen stay what they were.
    fn render(&mut self, packet: &FramePacket) -> Result<(), Self::Error>;

    /// Captures what the display shows in its current state, the mix of the held and the
    /// newest frame at the current weight, as the held frame. At weight 1 that is the newest
    /// frame; in the middle of a cross-fade it is the mixed image on screen.
    fn hold(&mut self) -> Result<(), Self::Error>;

    /// Sets the mix weight, from 0 (held frame only) to 1 (newest frame only). It applies from
    /// the next time something is shown.
    fn set_mix(&mut self, weight: f32);

    /// Shows the current state again without a new frame.
    fn redraw(&mut self) -> Result<(), Self::Error>;

    /// Sets the window title.
    fn set_title(&mut self, title: &str);
}
