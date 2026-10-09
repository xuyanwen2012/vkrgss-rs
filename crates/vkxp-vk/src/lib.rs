//! Vulkan backend: binning, compositing, presentation, pacing.
//!
//! Side: host.
//!
//! What exists today is the headless compositor: [`VkCompositor`] takes the same messages as the
//! CPU reference compositor and produces the same frames, byte for byte, on a device chosen
//! with [`DeviceChoice`]. It needs no window, surface or display.
//!
//! - `bitmaps` keeps the slot table that `BitmapDelta`s drive; the compositor mirrors it onto
//!   one `R8G8B8A8_UINT` image per bitmap.
//! - `records` validates a packet exactly as the reference does and encodes its ops for the
//!   shader; `binning` assigns the records to 16x16 tiles.
//! - `shaders/compose.comp` evaluates the ops per pixel, in order, with integer arithmetic
//!   mirrored from `vkxp-pixel` (`shaders/pixel.glsl`) and the exact transform
//!   (`shaders/transform.glsl`).
//! - The frame stays on the device; [`VkCompositor::read_frame`] copies it out.

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

pub use compositor::{Frame, VkCompositor};
pub use device::{DeviceChoice, DeviceInfo, DeviceKind, available_devices};
pub use error::{BitmapError, DeviceError, Limit, SlotState, VkError};
pub use validation::ValidationLog;
