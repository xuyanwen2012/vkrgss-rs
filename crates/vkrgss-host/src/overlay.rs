//! What a window can show over the game and drive with touches: the on-screen buttons of a
//! phone. The windowed runner hands it the touches and the keys of the window and draws its
//! picture over every frame shown ([`vkrgss_vk::VkCompositor::set_overlay`]); frames written
//! to files never hold it.

use vkrgss_proto::{InputSnapshot, Size};
use vkrgss_vk::OverlayImage;
use winit::event::Touch;
use winit::keyboard::{KeyCode, PhysicalKey};

/// What an event asks of the session, through the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayAction {
    /// The buttons the overlay holds from now on, with those of the gamepads.
    Hold(InputSnapshot),
    /// A key goes down (`true`) or up, as on a keyboard.
    Key(KeyCode, bool),
    /// The game is asked to quit, as when the window is closed.
    Quit,
}

/// A picture over the game and what touches on it do.
pub trait Overlay {
    /// A finger touched the window, whose client area is `window` pixels large, moved on it,
    /// or left it.
    fn touch(&mut self, window: Size, touch: &Touch) -> Vec<OverlayAction>;

    /// A key of the window went down or up. The session also gets the key, after the
    /// actions.
    fn key(&mut self, key: PhysicalKey, pressed: bool) -> Vec<OverlayAction>;

    /// Whether the picture for a window of `window` pixels differs from the one last given.
    fn changed(&self, window: Size) -> bool;

    /// The picture for a window of `window` pixels, or `None` when nothing is to be drawn.
    fn picture(&mut self, window: Size) -> Option<OverlayImage>;
}
