//! What the tests record of rendering and inject into it.
//!
//! [`Probe`] exists in every build. Outside the tests it has no field and its methods do
//! nothing, so the compositor calls them without an attribute.

use ash::prelude::VkResult;
#[cfg(test)]
use ash::vk;

use crate::VkError;
#[cfg(test)]
use crate::compositor::VkCompositor;

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
    /// Failure injection: whether the next allocation of a frame target fails.
    #[cfg(test)]
    target_failure: bool,
    /// Failure injection: what the next wait for a submission returns once it has waited.
    #[cfg(test)]
    wait_result: Option<vk::Result>,
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

    /// Before a frame target is allocated: fails when that is the injected failure.
    pub(super) fn before_target(&mut self) -> Result<(), VkError> {
        if std::mem::take(&mut self.target_failure) {
            return Err(VkError::Memory {
                what: "frame target",
                message: "injected failure".to_owned(),
            });
        }
        Ok(())
    }

    /// What a wait for a submission that returned `waited` returns: the injected result, once,
    /// or `waited`.
    pub(super) fn wait_result(&mut self, waited: VkResult<()>) -> VkResult<()> {
        match self.wait_result.take() {
            Some(result) => Err(result),
            None => waited,
        }
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

    #[inline(always)]
    pub(super) fn before_target(&mut self) -> Result<(), VkError> {
        Ok(())
    }

    #[inline(always)]
    pub(super) fn wait_result(&mut self, waited: VkResult<()>) -> VkResult<()> {
        waited
    }
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

    /// Makes the next allocation of a frame target fail.
    pub(crate) fn fail_next_target(&mut self) {
        self.probe.target_failure = true;
    }

    /// The number of frame buffers created so far.
    pub(crate) fn frame_buffers_created(&self) -> u32 {
        self.probe.frame_buffers_created
    }

    /// Makes the next wait for a submission return `result` once it has waited: the device
    /// stays as it is, and only the compositor sees the result, as `TIMEOUT` for a wait that
    /// ran out or `ERROR_DEVICE_LOST`.
    pub(crate) fn inject_wait_result(&mut self, result: vk::Result) {
        self.probe.wait_result = Some(result);
    }

    /// The size of the staging buffer in bytes, if there is one.
    pub(crate) fn staging_buffer_bytes(&self) -> Option<u64> {
        self.frame_buffers[super::FrameBuffer::Staging as usize]
            .as_ref()
            .map(|(_, bytes)| *bytes)
    }

    /// Builds a compositor on the test device whose commands `replace` sets as
    /// [`replace_device_commands`](VkCompositor::replace_device_commands) does, before any
    /// object is made from the device.
    pub(crate) fn with_device_commands(
        replace: impl FnOnce(&mut ash::DeviceFnV1_0, &mut ash::DeviceFnV1_3),
    ) -> Result<VkCompositor, VkError> {
        let mut gpu = crate::device::Gpu::new(&crate::tests::harness::test_device(), None)?;
        let device = &gpu.device;
        let (mut v1_0, mut v1_3) = (device.fp_v1_0().clone(), device.fp_v1_3().clone());
        replace(&mut v1_0, &mut v1_3);
        gpu.device = ash::Device::from_parts_1_3(
            device.handle(),
            v1_0,
            device.fp_v1_1().clone(),
            device.fp_v1_2().clone(),
            v1_3,
        );
        VkCompositor::finish(gpu)
    }

    /// Replaces commands of the device, as `replace` sets them in its tables of Vulkan 1.0
    /// and 1.3, for the calls the compositor makes through the device from now on: what a
    /// driver that answers them otherwise would do. The allocator and the extension loaders
    /// keep the real commands.
    pub(crate) fn replace_device_commands(
        &mut self,
        replace: impl FnOnce(&mut ash::DeviceFnV1_0, &mut ash::DeviceFnV1_3),
    ) {
        let device = &self.gpu.device;
        let (mut v1_0, mut v1_3) = (device.fp_v1_0().clone(), device.fp_v1_3().clone());
        replace(&mut v1_0, &mut v1_3);
        self.gpu.device = ash::Device::from_parts_1_3(
            device.handle(),
            v1_0,
            device.fp_v1_1().clone(),
            device.fp_v1_2().clone(),
            v1_3,
        );
    }

    /// The number of submissions so far.
    pub(crate) fn submissions(&self) -> u64 {
        self.submissions
    }

    /// Whether a submission may still be in use by the device: the compose of the last
    /// render, until a wait has seen it complete.
    pub(crate) fn has_compose_in_flight(&self) -> bool {
        self.in_flight.is_some()
    }

    /// The pixels of a live bitmap, read back from its image.
    pub(crate) fn read_bitmap(&mut self, id: vkrgss_proto::BitmapId) -> Result<Vec<u8>, VkError> {
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
