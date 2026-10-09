//! Vulkan backend: binning, compositing, presentation, pacing.
//!
//! Side: host.
//!
//! What exists today is the headless compositor: [`VkCompositor`] takes the same messages as the
//! CPU reference compositor and produces the same frames, byte for byte, on a device chosen
//! with [`DeviceChoice`]. It needs no window, surface or display.

mod compositor;
mod device;
mod error;
mod validation;

#[cfg(test)]
mod tests;

pub use compositor::VkCompositor;
pub use device::{DeviceChoice, DeviceInfo, DeviceKind, available_devices};
pub use error::{BitmapError, DeviceError, Limit, SlotState, VkError};
pub use validation::ValidationLog;
