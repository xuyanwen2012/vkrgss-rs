//! Vulkan backend: binning, compositing, presentation, pacing.
//!
//! Side: host.
//!
//! [`VkCompositor`] takes the same messages as the CPU reference compositor and produces the
//! same frames, byte for byte, on a device chosen with [`DeviceChoice`]. Created with
//! [`VkCompositor::new`] it is headless and needs no window, surface or display. Created with
//! [`VkCompositor::for_window`] it also owns a swapchain and shows its frames.
//!
//! - `bitmaps` keeps the slot table that `BitmapDelta`s drive; the compositor mirrors it onto
//!   one `R8G8B8A8_UINT` image per bitmap.
//! - `records` validates a packet exactly as the reference does and encodes its ops for the
//!   shader; `binning` assigns the records to 16x16 tiles.
//! - `shaders/compose.comp` evaluates the ops per pixel, in order, with integer arithmetic
//!   mirrored from `vkxp-pixel` (`shaders/pixel.glsl`) and the exact transform
//!   (`shaders/transform.glsl`).
//! - The frame stays on the device; [`VkCompositor::read_frame`] copies it out.
//! - Presentation (`compositor/present.rs`, `shaders/present.frag`) scales the frame into the
//!   window with letterboxing, mixing it with a held frame for freeze and transition. The same
//!   draw can go to an off-screen target with [`VkCompositor::scale_frame`]. This step uses
//!   floating point and is never read back into the compositor.

mod binning;
mod bitmaps;
mod compositor;
mod device;
mod error;
mod records;
mod resources;
mod validation;

#[cfg(test)]
mod tests;

pub use compositor::{Filter, Frame, Mix, VkCompositor, letterbox};
pub use device::{DeviceChoice, DeviceInfo, DeviceKind, available_devices};
pub use error::{BitmapError, DeviceError, Limit, SlotState, VkError};
pub use validation::ValidationLog;
