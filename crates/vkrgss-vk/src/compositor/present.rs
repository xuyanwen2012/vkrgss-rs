//! Presentation: scaling the frame into a swapchain image or an off-screen target.
//!
//! One graphics pipeline draws a triangle over the whole target. `shaders/present.frag` puts
//! the frame inside the letterbox rectangle computed here and black outside it, mixing the
//! newest frame with the held one on the way, by one weight or per pixel under the mask of a
//! transition. The held frame is the picture a freeze captured at its own size, or for RGSS3
//! a buffer of 640 by 480 texels whose top left corner a freeze writes
//! ([`VkCompositor::freeze_into_buffer`]). The same draw goes to a swapchain image
//! ([`VkCompositor::present`]) or to an image that is read back
//! ([`VkCompositor::scale_frame`]), which is how scaling and mixing are tested without a
//! window.
//!
//! Like the rest of the compositor this is synchronous: each call submits its work and waits
//! for it before it returns or hands the image to the presentation engine.

mod format;
mod held;
mod lifecycle;
mod presenter;
mod probe;
mod readback;
mod scaling;
mod show;
mod state;
mod swapchain;
#[cfg(test)]
mod tests;

pub(super) use presenter::Presenter;
pub(super) use probe::Probe;
pub(super) use scaling::Mask;
pub use scaling::{Filter, Mix, letterbox};
