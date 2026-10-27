//! The commands of a packet, their submission, and the copy of an image to the host.

use ash::vk;
use gpu_allocator::MemoryLocation;

use std::collections::{HashMap, HashSet};

use super::{
    FrameBuffer, FrameData, PUSH_CONSTANT_BYTES, Pending, SUBMISSION_TIMEOUT, Step, VkCompositor,
    query,
};
use crate::VkError;
use crate::resources::{
    COLOR_LAYERS, COLOR_RANGE, GpuBuffer, ImageUse, make_host_readable, transition, transition_all,
    write_words,
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
            // SAFETY: the command buffer is recording. Every image named by a step is alive
            // until `execute` has waited for this submission: live ones are in `self.images`
            // and destroyed ones in `pending.retired`. A new image is in `UNDEFINED` and its
            // first step is its clear; any other is in `SHADER_READ_ONLY_OPTIMAL`, where the
            // previous submission left it for its readers. Upload rectangles were checked
            // against the image size and their bytes lie inside the staging buffer, which is
            // only null when one large enough could not be had.
            unsafe {
                self.gpu.begin_label(commands, c"upload");
                record_steps(device, commands, &pending.steps, staging_buffer, bitmap_use);
                self.gpu.end_label(commands);
            }
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
                &[
                    frame.size.width,
                    frame.size.height,
                    frame.tiles.tiles_x,
                    u32::from(frame.brightness),
                ],
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
    pub(super) fn bitmap_use(&self) -> ImageUse {
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

    /// Ends recording, submits the command buffer and blocks until the device has finished it,
    /// or for [`SUBMISSION_TIMEOUT`] at most, after which the device counts as lost.
    pub(super) fn submit_and_wait(&mut self) -> Result<(), VkError> {
        // SAFETY: there are no other semaphores.
        unsafe { self.submit_with(&[], &[]) }
    }

    /// Ends recording and submits the command buffer, which waits for the semaphores of
    /// `waits` and, once done, signals those of `signals` as well as the next value of the
    /// timeline; then blocks until the timeline has that value, so until the device has
    /// finished the commands. A wait that runs out after [`SUBMISSION_TIMEOUT`], like a
    /// lost device, is [`VkError::DeviceLost`] and marks the compositor lost.
    ///
    /// # Safety
    ///
    /// The semaphores of `waits` and `signals` must be live binary semaphores of this device;
    /// each of `waits` must have a signal operation pending or done that no other wait
    /// consumes, and each of `signals` must be unsignalled with no signal operation pending.
    pub(super) unsafe fn submit_with(
        &mut self,
        waits: &[vk::SemaphoreSubmitInfo<'_>],
        signals: &[vk::SemaphoreSubmitInfo<'_>],
    ) -> Result<(), VkError> {
        let device = &self.gpu.device;
        let value = self.submissions + 1;
        let mut all_signals = Vec::with_capacity(signals.len() + 1);
        all_signals.extend_from_slice(signals);
        all_signals.push(
            vk::SemaphoreSubmitInfo::default()
                .semaphore(self.timeline)
                .value(value)
                .stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS),
        );
        let buffers = [vk::CommandBufferSubmitInfo::default().command_buffer(self.commands)];
        let submit = vk::SubmitInfo2::default()
            .wait_semaphore_infos(waits)
            .command_buffer_infos(&buffers)
            .signal_semaphore_infos(&all_signals);
        // SAFETY: the command buffer is recording and everything it references stays alive
        // until the wait below returns; the queue and the timeline belong to this device, the
        // device was created with synchronization2 and timeline semaphores, `value` is greater
        // than every value signalled on the timeline before, and the queue is only used from
        // here. The caller vouches for the semaphores of `waits` and `signals`.
        unsafe {
            device
                .end_command_buffer(self.commands)
                .map_err(VkError::vulkan("vkEndCommandBuffer"))?;
            device
                .queue_submit2(self.gpu.queue, &[submit], vk::Fence::null())
                .map_err(VkError::vulkan("vkQueueSubmit2"))?;
        }
        self.submissions = value;
        let (semaphores, values) = ([self.timeline], [value]);
        let wait = vk::SemaphoreWaitInfo::default()
            .semaphores(&semaphores)
            .values(&values);
        let timeout = u64::try_from(SUBMISSION_TIMEOUT.as_nanos()).unwrap_or(u64::MAX);
        // SAFETY: the timeline belongs to this device and the value was submitted above.
        let waited = unsafe { device.wait_semaphores(&wait, timeout) };
        let result = match self.probe.wait_result(waited) {
            Ok(()) => return Ok(()),
            Err(vk::Result::TIMEOUT) => {
                tracing::error!(
                    "a submission has not completed after {} seconds; the device is taken as lost",
                    SUBMISSION_TIMEOUT.as_secs()
                );
                vk::Result::ERROR_DEVICE_LOST
            }
            Err(result) => result,
        };
        let error = VkError::from_result("vkWaitSemaphores", result);
        if error.is_device_lost() {
            // The submission may still be running: nothing it uses may be touched again.
            self.lost = Some("vkWaitSemaphores");
        }
        Err(error)
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
        if result.is_err() && self.lost.is_some() {
            // The copy may still be running; see `execute`.
            return result.map(|()| Vec::new());
        }
        if result.is_err() {
            // SAFETY: the device is live; see `execute`.
            let _ = unsafe { self.gpu.device.device_wait_idle() };
        }
        let pixels = result.map(|()| buffer.bytes()[..size as usize].to_vec());
        buffer.destroy(&self.gpu.device, &mut self.allocator);
        pixels
    }

    fn copy_image_to_buffer(
        &mut self,
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
            // The caller reads the buffer through its mapping once the submission is done.
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
}

/// Records the bitmap steps of a packet: one dependency that hands every image they write to
/// the transfer stage, the clears and copies in delta order, and one dependency that hands
/// the images back to their readers in `bitmap_use`.
///
/// A step that writes every pixel of its image, a clear or a whole upload with a staging
/// buffer to copy from, makes the steps before it on that image moot; those are left out, and
/// an image whose first remaining step is such a step starts from `UNDEFINED`, its old
/// contents discarded. Two remaining writes to one image, which the guest does not send but
/// the protocol allows, are ordered by a barrier between the transfers.
///
/// # Safety
///
/// `commands` must be a command buffer of `device` in the recording state. Every image named
/// by a step must be live until the submission has been waited for, and be new (in
/// `UNDEFINED`, with a clear as its first step) or in `SHADER_READ_ONLY_OPTIMAL` as
/// `bitmap_use` says, with transfer-destination usage. Upload rectangles lie inside their
/// images and their bytes inside `staging`, which is null when no buffer large enough could
/// be had, in which case no copy is recorded.
unsafe fn record_steps(
    device: &ash::Device,
    commands: vk::CommandBuffer,
    steps: &[Step<'_>],
    staging: vk::Buffer,
    bitmap_use: ImageUse,
) {
    let image = |step: &Step<'_>| match step {
        Step::Clear { image } | Step::Upload { image, .. } => *image,
    };
    let replaces = |step: &Step<'_>| match step {
        Step::Clear { .. } => true,
        Step::Upload { whole, .. } => *whole && staging != vk::Buffer::null(),
    };
    // The last step that writes every pixel of each image; the steps on that image before it
    // are left out.
    let mut last_whole = HashMap::new();
    for (index, step) in steps.iter().enumerate() {
        if replaces(step) {
            last_whole.insert(image(step), index);
        }
    }
    let kept: Vec<&Step<'_>> = steps
        .iter()
        .enumerate()
        .filter(|(index, step)| {
            last_whole
                .get(&image(step))
                .is_none_or(|last| index >= last)
        })
        .map(|(_, step)| step)
        .collect();

    let mut before = Vec::new();
    let mut seen = HashSet::new();
    for step in &kept {
        if seen.insert(image(step)) {
            let from = match replaces(step) {
                true => ImageUse::UNDEFINED,
                false => bitmap_use,
            };
            before.push((image(step), from, ImageUse::TRANSFER_WRITE));
        }
    }
    let after: Vec<_> = before
        .iter()
        .map(|&(image, _, to)| (image, to, bitmap_use))
        .collect();

    // SAFETY: guaranteed by the caller; each image appears once in `before` and `after`, and
    // is in the layout of the use it is taken from, as the steps are recorded in order.
    unsafe {
        transition_all(device, commands, &before);
        let mut written = HashSet::new();
        for step in kept {
            if matches!(step, Step::Upload { .. }) && staging == vk::Buffer::null() {
                continue;
            }
            if !written.insert(image(step)) {
                // A second write to an image since the last barrier: the first has to be done
                // before it starts.
                let barriers = [vk::MemoryBarrier2::default()
                    .src_stage_mask(vk::PipelineStageFlags2::ALL_TRANSFER)
                    .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                    .dst_stage_mask(vk::PipelineStageFlags2::ALL_TRANSFER)
                    .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)];
                let info = vk::DependencyInfo::default().memory_barriers(&barriers);
                device.cmd_pipeline_barrier2(commands, &info);
                written.clear();
                written.insert(image(step));
            }
            match step {
                Step::Clear { image } => device.cmd_clear_color_image(
                    commands,
                    *image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &vk::ClearColorValue { uint32: [0; 4] },
                    &[COLOR_RANGE],
                ),
                Step::Upload {
                    image,
                    rect,
                    offset,
                    ..
                } => {
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
                    device.cmd_copy_buffer_to_image(
                        commands,
                        staging,
                        *image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[region],
                    );
                }
            }
        }
        transition_all(device, commands, &after);
    }
}
