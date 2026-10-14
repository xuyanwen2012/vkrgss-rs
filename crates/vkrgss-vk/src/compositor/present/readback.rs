//! Reading a frame back: the draw into an off-screen target.

use ash::vk;
use vkrgss_proto::Size;

use super::presenter::{DrawCall, PassTarget, record_pass};
use super::scaling::{Placement, filling_constants, push_constants};
use crate::compositor::{Frame, VkCompositor};
use crate::resources::{GpuImage, ImageUse};
use crate::{Limit, VkError};

/// Format of the off-screen target. Its values are what a `UNORM` swapchain image would hold.
const OFFSCREEN_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;

impl VkCompositor {
    /// Draws what [`present`](VkCompositor::present) would show in a window of `size` pixels
    /// into an off-screen target and reads it back: the frame scaled by the largest factor
    /// that fits, centred, black around it, mixed with the held frame as the current mix says.
    /// Before the first frame the result is black. Alpha is 255 everywhere.
    ///
    /// The target is an `R8G8B8A8_UNORM` image, so the bytes are what a `UNORM` swapchain
    /// image would hold. Works with and without a surface.
    pub fn scale_frame(&mut self, size: Size) -> Result<Frame, VkError> {
        if size.width == 0 || size.height == 0 {
            return Ok(Frame {
                size,
                rgba: Vec::new(),
            });
        }
        let longest = size.width.max(size.height);
        if longest > self.gpu.max_image_dimension {
            return Err(VkError::Limit {
                limit: Limit::FrameDimension,
                requested: u64::from(longest),
                maximum: u64::from(self.gpu.max_image_dimension),
            });
        }
        self.last_scaled = Some(size);
        let rgba = self.with_offscreen(size, Placement::Letterbox, |compositor, image| {
            compositor.read_image(image, (size.width, size.height), ImageUse::TRANSFER_READ)
        })?;
        Ok(Frame { size, rgba })
    }

    /// Draws what a window of `size` pixels would show into a new off-screen target, or with
    /// [`Placement::Fill`] the frames stretched over all of it, and hands its image, in the
    /// `TRANSFER_SRC_OPTIMAL` layout, to `then`. The target is destroyed afterwards, so `then`
    /// has to wait for whatever it submits. `size` has no zero dimension and is within the
    /// device's limit.
    pub(super) fn with_offscreen<T>(
        &mut self,
        size: Size,
        placement: Placement,
        then: impl FnOnce(&mut VkCompositor, vk::Image) -> Result<T, VkError>,
    ) -> Result<T, VkError> {
        let source = self.prepare_sources()?;
        let device = &self.gpu.device;
        let presenter = self.presenter.as_mut().expect("made with the sources");
        let pipeline = presenter.pipeline(&self.gpu, OFFSCREEN_FORMAT)?;
        let draw = source.and_then(|sources| {
            Some(DrawCall {
                pipeline,
                pipeline_layout: presenter.pipeline_layout,
                descriptors: presenter.descriptors,
                constants: match placement {
                    Placement::Letterbox => {
                        push_constants(sources, size, self.mix, false, self.filter)?
                    }
                    Placement::Fill => filling_constants(sources, size, self.mix, self.filter),
                },
            })
        });

        let extent = vk::Extent2D {
            width: size.width,
            height: size.height,
        };
        let image = GpuImage::with_format(
            device,
            &mut self.allocator,
            "off-screen target",
            (size.width, size.height),
            OFFSCREEN_FORMAT,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
            vk::ImageCreateFlags::empty(),
        )?;
        let target = PassTarget {
            image: image.image,
            view: image.view,
            extent,
            before: ImageUse::UNDEFINED,
            after: ImageUse::TRANSFER_READ,
        };

        let drawn = self.submit_or_idle(|compositor| {
            compositor.begin_commands()?;
            // SAFETY: the command buffer is recording, on a queue that draws since there is a
            // presenter. The image and its view were made above for `extent` and the format
            // of the pipeline, are new, and live until this submission has been waited for. A
            // draw call only exists when a frame does, in which case `prepare_sources`
            // pointed the set at the frame target (`GENERAL`), the held frame and the mask
            // (`SHADER_READ_ONLY_OPTIMAL`), each left to the fragment stage by its last
            // barrier.
            unsafe {
                compositor.gpu.begin_label(compositor.commands, c"present");
                record_pass(
                    &compositor.gpu.device,
                    compositor.commands,
                    &target,
                    draw.as_ref(),
                );
                compositor.gpu.end_label(compositor.commands);
            }
            compositor.submit_and_wait()
        });
        let result = drawn.and_then(|()| then(self, image.image));
        // Every submission that used the image was waited for, or the device was idled after
        // a failure.
        image.destroy(&self.gpu.device, &mut self.allocator);
        result
    }
}
