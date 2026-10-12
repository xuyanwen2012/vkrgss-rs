//! The commands of a packet, their submission, and the copy of an image to the host.

use ash::vk;
use gpu_allocator::MemoryLocation;

use super::{FrameBuffer, FrameData, PUSH_CONSTANT_BYTES, Pending, Step, VkCompositor, query};
use crate::VkError;
use crate::resources::{
    COLOR_LAYERS, COLOR_RANGE, GpuBuffer, ImageUse, make_host_readable, transition, write_words,
};

impl VkCompositor {
    /// Records and submits the bitmap steps and, when everything it needs could be made, the
    /// frame.
    ///
    /// The table already counts the packet's deltas as applied, so the bitmap steps are
    /// submitted even when a buffer cannot be allocated: every image ends up in the layout the
    /// rest of the backend expects. Without a staging buffer that holds the uploads they copy
    /// nothing and those bitmaps keep their previous pixels; without a frame buffer or the
    /// target there is no draw. Either way the allocation error is what the caller gets.
    pub(super) fn submit(
        &mut self,
        pending: &Pending<'_>,
        frame: Option<&FrameData>,
    ) -> Result<(), VkError> {
        let mut failure = None;
        let started = self.stage_start();

        let mut staging_buffer = vk::Buffer::null();
        if pending.staging_len > 0 {
            match self.frame_buffer(FrameBuffer::Staging, pending.staging_len) {
                Ok(buffer) => {
                    let bytes = buffer.bytes_mut();
                    for step in &pending.steps {
                        if let Step::Upload { offset, rgba, .. } = step {
                            let start = *offset as usize;
                            bytes[start..start + rgba.len()].copy_from_slice(rgba);
                        }
                    }
                    staging_buffer = buffer.buffer;
                }
                Err(error) => failure = Some(error),
            }
        }

        // The frame, with whether its target is new. Made before recording starts.
        let mut draw = None;
        if let (Some(frame), None) = (frame, &failure) {
            match self.prepare_frame_resources(frame) {
                Ok(new_target) => draw = Some((frame, new_target)),
                Err(error) => failure = Some(error),
            }
        }
        if pending.steps.is_empty() && draw.is_none() {
            return failure.map_or(Ok(()), Err);
        }

        self.begin_commands()?;
        let (bitmap_use, frame_use) = (self.bitmap_use(), self.frame_use());
        let (device, commands) = (&self.gpu.device, self.commands);
        if !pending.steps.is_empty() {
            // SAFETY: the command buffer is recording.
            unsafe { self.gpu.begin_label(commands, c"upload") };
        }
        for step in &pending.steps {
            // SAFETY: the command buffer is recording. Every image named by a step is alive
            // until `execute` has waited for this submission: live ones are in `self.images`
            // and destroyed ones in `pending.retired`. A new image starts in `UNDEFINED`; any
            // other is in `SHADER_READ_ONLY_OPTIMAL`, where the previous step or submission
            // left it for its readers. Upload rectangles were checked against the image size and their bytes
            // lie inside the staging buffer, which is only null when one large enough could not
            // be had, in which case no copy is recorded.
            unsafe {
                match step {
                    Step::Clear { image } => {
                        transition(
                            device,
                            commands,
                            *image,
                            ImageUse::UNDEFINED,
                            ImageUse::TRANSFER_WRITE,
                        );
                        device.cmd_clear_color_image(
                            commands,
                            *image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            &vk::ClearColorValue { uint32: [0; 4] },
                            &[COLOR_RANGE],
                        );
                    }
                    Step::Upload {
                        image,
                        rect,
                        offset,
                        ..
                    } => {
                        transition(
                            device,
                            commands,
                            *image,
                            bitmap_use,
                            ImageUse::TRANSFER_WRITE,
                        );
                        let region = vk::BufferImageCopy::default()
                            .buffer_offset(*offset)
                            .image_subresource(COLOR_LAYERS)
                            .image_offset(vk::Offset3D {
                                x: rect.x,
                                y: rect.y,
                                z: 0,
                            })
                            .image_extent(vk::Extent3D {
                                width: rect.width,
                                height: rect.height,
                                depth: 1,
                            });
                        if staging_buffer != vk::Buffer::null() {
                            device.cmd_copy_buffer_to_image(
                                commands,
                                staging_buffer,
                                *image,
                                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                                &[region],
                            );
                        }
                    }
                }
                let (Step::Clear { image } | Step::Upload { image, .. }) = step;
                transition(
                    device,
                    commands,
                    *image,
                    ImageUse::TRANSFER_WRITE,
                    bitmap_use,
                );
            }
        }
        if !pending.steps.is_empty() {
            // SAFETY: the command buffer is recording and the label above is open in it.
            unsafe { self.gpu.end_label(commands) };
        }

        if let Some((frame, new_target)) = draw {
            let target = self
                .target
                .as_ref()
                .expect("made with the frame resources")
                .image;
            let mut constants = [0; PUSH_CONSTANT_BYTES as usize];
            write_words(
                &mut constants,
                &[frame.size.width, frame.size.height, frame.tiles.tiles_x],
            );
            // SAFETY: the command buffer is recording. The pipeline, its layout and the set
            // belong to this device and match each other. Every descriptor the shader can use
            // is valid: `prepare_frame_resources` saw to the five buffers and the target, each
            // written when it was made, and a quad or tilemap record
            // only names a slot whose image is live and, after the steps above, in
            // `SHADER_READ_ONLY_OPTIMAL`. The buffers hold what the tile headers and indices
            // address, the table buffer holds every element a tilemap record's offsets and
            // sizes reach, the target has the size in the push constants, and the tile grid is
            // within the dispatch limits every device has. The target is new or in `GENERAL`,
            // where the dispatch before left it for its readers. The label and the timestamps
            // are recorded outside any render pass, each end after its start.
            unsafe {
                self.gpu.begin_label(commands, c"compose");
                let before = match new_target {
                    true => ImageUse::UNDEFINED,
                    false => frame_use,
                };
                transition(
                    device,
                    commands,
                    target,
                    before,
                    ImageUse::COMPUTE_STORAGE_WRITE,
                );
                self.record_span_start(query::COMPOSE);
                device.cmd_bind_pipeline(commands, vk::PipelineBindPoint::COMPUTE, self.pipeline);
                device.cmd_bind_descriptor_sets(
                    commands,
                    vk::PipelineBindPoint::COMPUTE,
                    self.pipeline_layout,
                    0,
                    &[self.descriptors],
                    &[],
                );
                device.cmd_push_constants(
                    commands,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::COMPUTE,
                    0,
                    &constants,
                );
                device.cmd_dispatch(commands, frame.tiles.tiles_x, frame.tiles.tiles_y, 1);
                self.record_span_end(query::COMPOSE);
                // Hands the composed frame to the presentation pass and the copies.
                transition(
                    device,
                    commands,
                    target,
                    ImageUse::COMPUTE_STORAGE_WRITE,
                    frame_use,
                );
                self.gpu.end_label(commands);
            }
        }
        if draw.is_some() {
            // Until the dispatch has completed the target holds no whole frame.
            self.has_frame = false;
        }
        let recorded = self.stage_start();
        self.submit_and_wait()?;
        if draw.is_some() {
            self.has_frame = true;
            let compose_device = self.read_span(query::COMPOSE);
            if let (Some(timing), Some(started), Some(recorded)) =
                (&mut self.frame_timing, started, recorded)
            {
                timing.upload = Some(recorded.duration_since(started));
                timing.submit = Some(recorded.elapsed());
                timing.compose_device = compose_device;
            }
        }
        failure.map_or(Ok(()), Err)
    }

    /// How the image of a bitmap is used between its uploads: the compute dispatch fetches
    /// from it, and so does the fragment shader of the presentation pass while the bitmap is a
    /// transition mask, which only a queue that can draw gets to.
    fn bitmap_use(&self) -> ImageUse {
        match self.gpu.can_draw {
            true => ImageUse::COMPUTE_SAMPLED_READ.and(ImageUse::FRAGMENT_SAMPLED_READ),
            false => ImageUse::COMPUTE_SAMPLED_READ,
        }
    }

    /// How the frame target is used between dispatches, in the `GENERAL` layout it stays in:
    /// copies read it (read-back, freeze), and so does the fragment shader of the
    /// presentation pass on a queue that can draw.
    pub(super) fn frame_use(&self) -> ImageUse {
        let readers = match self.gpu.can_draw {
            true => ImageUse::TRANSFER_READ.and(ImageUse::FRAGMENT_SAMPLED_READ),
            false => ImageUse::TRANSFER_READ,
        };
        readers.in_layout(vk::ImageLayout::GENERAL)
    }

    /// Resets the command buffer and starts recording into it.
    pub(super) fn begin_commands(&self) -> Result<(), VkError> {
        let info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        // SAFETY: the command buffer belongs to this device, its pool allows resetting single
        // buffers, and it is not pending: every submission is waited for.
        unsafe {
            self.gpu
                .device
                .begin_command_buffer(self.commands, &info)
                .map_err(VkError::vulkan("vkBeginCommandBuffer"))
        }
    }

    /// Ends recording, submits the command buffer and blocks until the device has finished it.
    pub(super) fn submit_and_wait(&self) -> Result<(), VkError> {
        let device = &self.gpu.device;
        let buffers = [self.commands];
        let submit = vk::SubmitInfo::default().command_buffers(&buffers);
        // SAFETY: the command buffer is recording and everything it references stays alive
        // until the wait below returns; the queue and fence belong to this device, the fence
        // is unsignalled, and the queue is only used from here.
        unsafe {
            device
                .end_command_buffer(self.commands)
                .map_err(VkError::vulkan("vkEndCommandBuffer"))?;
            device
                .queue_submit(self.gpu.queue, &[submit], self.fence)
                .map_err(VkError::vulkan("vkQueueSubmit"))?;
            device
                .wait_for_fences(&[self.fence], true, u64::MAX)
                .map_err(VkError::vulkan("vkWaitForFences"))?;
            device
                .reset_fences(&[self.fence])
                .map_err(VkError::vulkan("vkResetFences"))
        }
    }

    /// Copies the pixels of an image to host memory: RGBA8 in row order.
    ///
    /// `resting` is the use the image is in, which is restored; an image whose use already
    /// lets copies read it, in `GENERAL` or `TRANSFER_SRC_OPTIMAL`, is copied from as it is.
    pub(super) fn read_image(
        &mut self,
        image: vk::Image,
        (width, height): (u32, u32),
        resting: ImageUse,
    ) -> Result<Vec<u8>, VkError> {
        let size = u64::from(width) * u64::from(height) * 4;
        let buffer = GpuBuffer::new(
            &self.gpu.device,
            &mut self.allocator,
            "readback",
            size,
            vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuToCpu,
        )?;
        let result = self.copy_image_to_buffer(image, (width, height), resting, buffer.buffer);
        if result.is_err() {
            // SAFETY: the device is live; see `execute`.
            let _ = unsafe { self.gpu.device.device_wait_idle() };
        }
        let pixels = result.map(|()| buffer.bytes()[..size as usize].to_vec());
        buffer.destroy(&self.gpu.device, &mut self.allocator);
        pixels
    }

    fn copy_image_to_buffer(
        &self,
        image: vk::Image,
        (width, height): (u32, u32),
        resting: ImageUse,
        buffer: vk::Buffer,
    ) -> Result<(), VkError> {
        self.begin_commands()?;
        let (device, commands) = (&self.gpu.device, self.commands);
        let copying = match resting.layout {
            vk::ImageLayout::GENERAL | vk::ImageLayout::TRANSFER_SRC_OPTIMAL => resting,
            _ => ImageUse::TRANSFER_READ,
        };
        let region = vk::BufferImageCopy::default()
            .image_subresource(COLOR_LAYERS)
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });
        // SAFETY: the command buffer is recording; the caller passes a live image of this size
        // in the use `resting`, created with transfer-source usage, and a live buffer that
        // holds the whole image; both outlive the wait in `submit_and_wait`.
        unsafe {
            if copying != resting {
                transition(device, commands, image, resting, copying);
            }
            device.cmd_copy_image_to_buffer(commands, image, copying.layout, buffer, &[region]);
            // The caller reads the buffer through its mapping once the fence has signalled.
            make_host_readable(
                device,
                commands,
                buffer,
                vk::PipelineStageFlags2::ALL_TRANSFER,
                vk::AccessFlags2::TRANSFER_WRITE,
            );
            if copying != resting {
                transition(device, commands, image, copying, resting);
            }
        }
        self.submit_and_wait()
    }

    /// The pixels of a live bitmap, read back from its image.
    #[cfg(test)]
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
