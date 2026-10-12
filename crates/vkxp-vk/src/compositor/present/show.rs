//! Showing a frame in the window: the acquire, the draw and the present.

use ash::vk;
use vkxp_proto::Size;

use super::presenter::{DrawCall, PassTarget, record_pass};
use super::scaling::push_constants;
use super::swapchain::ACQUIRE_TIMEOUT;
use crate::VkError;
use crate::compositor::pacer::TimingCalls;
use crate::compositor::{VkCompositor, query};
use crate::present_timing as ffi;
use crate::resources::ImageUse;

impl VkCompositor {
    /// Shows the current state in the window: the newest frame mixed with the held frame at
    /// the current weight, scaled into the window and centred, black around it. Before the
    /// first frame the window is black.
    ///
    /// Returns whether an image was presented. Nothing is presented, and no error is raised,
    /// while the window has no pixels, or when the surface hands out no image within a tenth
    /// of a second, as that of a hidden window may. After such a wait the presents that
    /// follow do not wait at all until the surface hands out an image again, and that
    /// present shows the newest frame. A swapchain that the surface reports as out of date
    /// is replaced and the present tried once more.
    ///
    /// Where the device and the surface support it, the present carries a target time; see
    /// [`set_frame_rate`](VkCompositor::set_frame_rate).
    ///
    /// Fails with [`VkError::NoSurface`] on a compositor that was created without a surface.
    pub fn present(&mut self) -> Result<bool, VkError> {
        if self.gpu.surface.is_none() {
            return Err(VkError::NoSurface);
        }
        if self.surface_size.width == 0 || self.surface_size.height == 0 {
            return Ok(false);
        }
        let source = self.prepare_sources()?;
        // Whether this present shows a frame for the first time, which is what is paced.
        let frame = std::mem::take(&mut self.unshown_frame);
        // One more attempt after an out-of-date swapchain, and one that does not count after
        // a present the pacing extension refused, which happens once at most.
        let mut attempts = 2;
        while attempts > 0 {
            attempts -= 1;
            if !self.ensure_swapchain()? {
                return Ok(false);
            }
            let presenter = self.presenter.as_mut().expect("made with the sources");
            let loader = presenter
                .swapchain_loader
                .clone()
                .expect("made with the swapchain");
            let chain = presenter.swapchain.as_mut().expect("just ensured");
            // A surface that gave no image in time is not waited for again until it gives
            // one, so a hidden window costs one wait and not one per frame.
            let timeout = match self.starved {
                true => 0,
                false => ACQUIRE_TIMEOUT,
            };
            #[cfg(test)]
            let injected = self.acquire_timeouts > 0;
            #[cfg(not(test))]
            let injected = false;
            let acquired = if injected {
                #[cfg(test)]
                {
                    self.acquire_timeouts -= 1;
                }
                Err(vk::Result::TIMEOUT)
            } else {
                // SAFETY: the swapchain and the semaphore are live and belong to this device,
                // and the semaphore is unsignalled: the draw that waited on it last has
                // completed, an acquire that gave no image left it as it was, and a swapchain
                // whose draw failed is replaced together with its semaphores.
                unsafe {
                    loader.acquire_next_image(
                        chain.handle,
                        timeout,
                        chain.acquired,
                        vk::Fence::null(),
                    )
                }
            };
            let index = match acquired {
                Ok((index, suboptimal)) => {
                    self.starved = false;
                    chain.stale |= suboptimal;
                    index
                }
                // No image in time, or none at once. The frame is rendered and stays the one
                // to show: like a window without pixels, this present counts for nothing.
                Err(vk::Result::TIMEOUT | vk::Result::NOT_READY) => {
                    self.starved = true;
                    self.unshown_frame = frame;
                    return Ok(false);
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    chain.stale = true;
                    continue;
                }
                Err(result) => {
                    return Err(VkError::Vulkan {
                        call: "vkAcquireNextImageKHR",
                        result,
                    });
                }
            };
            // Only now that there is an image to show the frame in: a present that shows
            // nothing must not move the target times on. This may wait for the frame before
            // to be shown.
            #[cfg(test)]
            let stand_in = self.timing_stand_in.is_some();
            #[cfg(not(test))]
            let stand_in = false;
            let calls = chain
                .timing
                .as_mut()
                .map(|timing| timing as &mut dyn TimingCalls);
            #[cfg(test)]
            let calls = match &mut self.timing_stand_in {
                Some(calls) => Some(&mut **calls as &mut dyn TimingCalls),
                None => calls,
            };
            let plan = self.pacer.plan(calls, frame);
            // The target of a stand-in is on no clock of the swapchain and is not sent.
            let timing_info = plan
                .as_ref()
                .filter(|_| !stand_in)
                .zip(chain.timing.as_ref())
                .map(|(plan, timing)| timing.present_info(plan));
            let (handle, extent) = (chain.handle, chain.extent);
            // An image that comes round again was presented; what it holds is not wanted.
            let pass_target = PassTarget {
                image: chain.images[index as usize],
                view: chain.views[index as usize],
                extent,
                before: ImageUse::ACQUIRED,
                after: ImageUse::PRESENT,
            };
            let (wait, signal) = (chain.acquired, chain.drawn[index as usize]);
            let target = Size::new(extent.width, extent.height);
            let draw = source.and_then(|sources| {
                Some(DrawCall {
                    pipeline: chain.pipeline,
                    pipeline_layout: presenter.pipeline_layout,
                    descriptors: presenter.descriptors,
                    constants: push_constants(
                        sources,
                        target,
                        self.mix,
                        chain.format.srgb_target,
                        self.filter,
                    )?,
                })
            });

            let started = self.stage_start();
            let drawn = self.submit_or_idle(|compositor| {
                #[cfg(test)]
                if std::mem::take(&mut compositor.fail_draw) {
                    return Err(VkError::Memory {
                        what: "the draw",
                        message: "injected failure".to_owned(),
                    });
                }
                compositor.begin_commands()?;
                // SAFETY: the command buffer is recording, on the queue that presents and
                // draws. The image and its view belong to the live swapchain, which is
                // `extent` pixels large, and the view has the format of the pipeline; the image was
                // just acquired, so it may be taken from `UNDEFINED`. A draw call only exists
                // when a frame does, in which case `prepare_sources` pointed the set at the
                // frame target (`GENERAL`), the held frame and the mask
                // (`SHADER_READ_ONLY_OPTIMAL`), each left to the fragment stage by its last
                // barrier. The label and the timestamps are recorded around the pass, each
                // end after its start.
                unsafe {
                    compositor.gpu.begin_label(compositor.commands, c"present");
                    compositor.record_span_start(query::PRESENT);
                    record_pass(
                        &compositor.gpu.device,
                        compositor.commands,
                        &pass_target,
                        draw.as_ref(),
                    );
                    compositor.record_span_end(query::PRESENT);
                    compositor.gpu.end_label(compositor.commands);
                }
                compositor.submit_drawn(wait, signal)
            });
            if let (Some(started), Ok(())) = (started, &drawn) {
                let present_device = self.read_span(query::PRESENT);
                let timing = self.frame_timing.get_or_insert_default();
                timing.present = Some(started.elapsed());
                timing.present_device = present_device;
            }
            if let Err(error) = drawn {
                // The image is acquired and the two semaphores are in an unknown state. With
                // swapchain maintenance the image is given back and the semaphores are
                // replaced; otherwise a new swapchain brings new ones.
                if !self.release_undrawn(index) {
                    self.mark_swapchain_stale();
                }
                return Err(error);
            }

            let (swapchains, indices, waits) = ([handle], [index], [signal]);
            let mut info = vk::PresentInfoKHR::default()
                .wait_semaphores(&waits)
                .swapchains(&swapchains)
                .image_indices(&indices);
            // With swapchain maintenance every present carries a fence.
            let present_fences = match self.take_present_fence() {
                Ok(fence) => [fence],
                Err(error) => {
                    // The image is drawn and its semaphore signalled, and neither is used.
                    self.mark_swapchain_stale();
                    return Err(error);
                }
            };
            let mut fence_info =
                vk::SwapchainPresentFenceInfoEXT::default().fences(&present_fences);
            if present_fences[0] != vk::Fence::null() {
                info = info.push_next(&mut fence_info);
                #[cfg(test)]
                {
                    self.fenced_presents += 1;
                }
            }
            // The structures of the pacing extensions, chained in when the present is paced.
            let present_ids = [plan.and_then(|plan| plan.present_id).unwrap_or(0)];
            let mut id_info = ffi::PresentId2 {
                swapchain_count: 1,
                p_present_ids: present_ids.as_ptr(),
                ..Default::default()
            };
            let mut timings_info = ffi::PresentTimingsInfo {
                swapchain_count: 1,
                ..Default::default()
            };
            if let Some(timing_info) = &timing_info {
                timings_info.p_timing_infos = timing_info;
                timings_info.p_next = info.p_next;
                info.p_next = (&raw const timings_info).cast();
            }
            if present_ids[0] != 0 {
                id_info.p_next = info.p_next;
                info.p_next = (&raw const id_info).cast();
            }
            // SAFETY: the queue can present to the surface, which was checked when the device
            // was chosen; the image was acquired from this swapchain and drawn by a submission
            // that has completed and signalled the semaphore. The chained structures, which
            // have the layout of the header, only exist for a swapchain that was created with
            // the flags of their extensions on a device that has them enabled; they and what
            // they point to live until the call returns; the target time is in a time domain
            // of the swapchain and the present id is greater than every one before it. A
            // fence is only chained on a device with swapchain maintenance, belongs to this
            // device, is unsignalled and is carried by no other present.
            match unsafe { loader.queue_present(self.gpu.queue, &info) } {
                Ok(suboptimal) => {
                    self.pacer.presented(plan.as_ref(), frame);
                    if suboptimal {
                        self.mark_swapchain_stale();
                    }
                    return Ok(true);
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.mark_swapchain_stale(),
                // The results queue of the swapchain is full. The presentation engine did
                // not take the image: its acquisition is released, and the wait on the
                // semaphore the draw signalled is queued, so the semaphore is unsignalled
                // again. The swapchain is as it was before the acquire, and the frame is
                // acquired, drawn and presented once more, now without the extension.
                Err(result) if self.pacer.present_failed(plan.as_ref(), result) => attempts += 1,
                Err(result) => {
                    self.mark_swapchain_stale();
                    return Err(VkError::Vulkan {
                        call: "vkQueuePresentKHR",
                        result,
                    });
                }
            }
        }
        Ok(false)
    }

    /// Ends recording and submits the command buffer so that it starts drawing once the
    /// swapchain image is available (`wait`) and signals `signal` when it is done, then
    /// blocks until the device has finished it.
    fn submit_drawn(&self, wait: vk::Semaphore, signal: vk::Semaphore) -> Result<(), VkError> {
        let device = &self.gpu.device;
        let (buffers, waits, signals) = ([self.commands], [wait], [signal]);
        let stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(&waits)
            .wait_dst_stage_mask(&stages)
            .command_buffers(&buffers)
            .signal_semaphores(&signals);
        // SAFETY: the command buffer is recording and everything it references stays alive
        // until the wait below returns. `wait` was signalled by the acquire of the image the
        // commands draw into, and `signal` is unsignalled: the presentation engine consumed
        // its previous signal before it handed the image out again. The queue and the fence
        // belong to this device, the fence is unsignalled, and the queue is only used from
        // here.
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
}
