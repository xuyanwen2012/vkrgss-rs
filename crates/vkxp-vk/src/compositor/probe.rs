//! What the tests record of rendering and inject into it.
//!
//! [`Probe`] exists in every build. Outside the tests it has no field and its methods do
//! nothing, so the compositor calls them without an attribute.

#[cfg(test)]
use vkxp_proto::BitmapDelta;

use crate::VkError;
#[cfg(test)]
use crate::compositor::{Pending, VkCompositor};

/// The records and the injected failures of the compositor's own work.
#[derive(Default)]
pub(super) struct Probe {
    /// Failure injection: the number of frame buffers that may still be created or grown
    /// before one such allocation fails, once.
    #[cfg(test)]
    frame_buffer_failure: Option<u32>,
    /// The number of frame buffers created so far, first ones and larger ones alike.
    #[cfg(test)]
    frame_buffers_created: u32,
}

#[cfg(test)]
impl Probe {
    /// Before a frame buffer named `what` is created: fails when that is the injected failure.
    pub(super) fn before_frame_buffer(&mut self, what: &'static str) -> Result<(), VkError> {
        match self.frame_buffer_failure {
            Some(0) => {
                self.frame_buffer_failure = None;
                return Err(VkError::Memory {
                    what,
                    message: "injected failure".to_owned(),
                });
            }
            Some(remaining) => self.frame_buffer_failure = Some(remaining - 1),
            None => {}
        }
        Ok(())
    }

    /// Counts a frame buffer that was created.
    pub(super) fn frame_buffer_created(&mut self) {
        self.frame_buffers_created += 1;
    }
}

#[cfg(not(test))]
impl Probe {
    #[inline(always)]
    pub(super) fn before_frame_buffer(&mut self, _what: &'static str) -> Result<(), VkError> {
        Ok(())
    }

    #[inline(always)]
    pub(super) fn frame_buffer_created(&mut self) {}
}

#[cfg(test)]
impl VkCompositor {
    /// Whether the device was created with `VK_KHR_swapchain_mutable_format`.
    pub(crate) fn has_swapchain_mutable_format(&self) -> bool {
        self.gpu.swapchain_mutable_format
    }

    /// Whether the queue of the device has timestamps.
    pub(crate) fn has_timestamps(&self) -> bool {
        self.gpu.timestamps.is_some()
    }

    /// Largest width or height of an image on the device.
    pub(crate) fn max_image_dimension(&self) -> u32 {
        self.gpu.max_image_dimension
    }

    /// Makes one creation of a frame buffer (the bitmap staging buffer, then the four storage
    /// buffers of a frame), a first one or a larger one, fail, after `successes` more of them
    /// have succeeded.
    pub(crate) fn fail_frame_buffer_after(&mut self, successes: u32) {
        self.probe.frame_buffer_failure = Some(successes);
    }

    /// The number of frame buffers created so far.
    pub(crate) fn frame_buffers_created(&self) -> u32 {
        self.probe.frame_buffers_created
    }

    /// Applies bitmap deltas in order and carries out their GPU work, without drawing.
    pub(crate) fn apply_bitmaps(&mut self, deltas: &[BitmapDelta]) -> Result<(), VkError> {
        let mut pending = Pending::default();
        let checked = self.queue_deltas(deltas, &mut pending);
        self.execute(pending, None)?;
        checked
    }

    /// The pixels of a live bitmap, read back from its image.
    pub(crate) fn read_bitmap(&mut self, id: vkxp_proto::BitmapId) -> Result<Vec<u8>, VkError> {
        self.table
            .get(id)
            .map_err(|source| VkError::InvalidOpBitmap { index: 0, source })?;
        match self.images.get(&id.index) {
            Some(image) => {
                self.read_image(image.image, (image.width, image.height), self.bitmap_use())
            }
            None => Ok(Vec::new()),
        }
    }
}
