//! Measuring the stages of a frame: the clock and the timestamp queries.

use std::time::{Duration, Instant};

use ash::vk;

use super::VkCompositor;
use crate::FrameTiming;

impl VkCompositor {
    /// Turns the measuring of frames on or off. It starts off, and then no clock is read and
    /// no query is recorded. While it is on, each [`render`](VkCompositor::render) starts a
    /// [`FrameTiming`] and fills in its stages, and a [`present`](VkCompositor::present)
    /// that shows something adds its own; [`take_frame_timing`] hands the value out.
    /// Measuring changes no frame.
    ///
    /// [`take_frame_timing`]: VkCompositor::take_frame_timing
    pub fn set_frame_timing(&mut self, on: bool) {
        self.timing_on = on;
        if !on {
            self.frame_timing = None;
        }
    }

    /// What was measured since the last call: the stages of the newest frame rendered, with
    /// those of the presents since. `None` when nothing was measured, as always while the
    /// measuring is off, and after a render that failed.
    pub fn take_frame_timing(&mut self) -> Option<FrameTiming> {
        self.frame_timing.take()
    }

    /// The clock reading a measured stage starts at, or `None` while frames are not measured.
    pub(super) fn stage_start(&self) -> Option<Instant> {
        self.timing_on.then(Instant::now)
    }

    /// Whether device durations are measured: frames are, and the queue has timestamps.
    fn times_device(&self) -> bool {
        self.timing_on && self.query_pool != vk::QueryPool::null()
    }

    /// Records the start of the device duration whose queries begin at `first`, when device
    /// durations are measured.
    ///
    /// # Safety
    ///
    /// The command buffer must be recording, outside a render pass.
    pub(super) unsafe fn record_span_start(&self, first: u32) {
        if self.times_device() {
            let (device, commands) = (&self.gpu.device, self.commands);
            // SAFETY: the pool has timestamp queries `first` and `first + 1`, which no
            // pending command buffer uses since every submission is waited for, and the
            // queue has timestamps; the rest is guaranteed by the caller.
            unsafe {
                device.cmd_reset_query_pool(commands, self.query_pool, first, 2);
                device.cmd_write_timestamp2(
                    commands,
                    vk::PipelineStageFlags2::ALL_COMMANDS,
                    self.query_pool,
                    first,
                );
            }
        }
    }

    /// Records the end of the device duration whose queries begin at `first`.
    ///
    /// # Safety
    ///
    /// The command buffer must be recording, outside a render pass, after a
    /// [`record_span_start`](VkCompositor::record_span_start) of the same queries in it.
    pub(super) unsafe fn record_span_end(&self, first: u32) {
        if self.times_device() {
            // SAFETY: as in `record_span_start`, which reset the query.
            unsafe {
                self.gpu.device.cmd_write_timestamp2(
                    self.commands,
                    vk::PipelineStageFlags2::ALL_COMMANDS,
                    self.query_pool,
                    first + 1,
                );
            }
        }
    }

    /// The device duration whose queries begin at `first`, once the submission that wrote
    /// them has been waited for. `None` when device durations are not measured or the
    /// queries cannot be read.
    pub(super) fn read_span(&self, first: u32) -> Option<Duration> {
        let timestamps = self.gpu.timestamps.filter(|_| self.times_device())?;
        let mut ticks = [0u64; 2];
        // SAFETY: the pool is live and has the two queries, which the submission just waited
        // for wrote; the results are 64-bit values, as the flag says. `WAIT` costs nothing
        // once the submission is done, and MoltenVK answers `NOT_READY` without it even then.
        unsafe {
            self.gpu.device.get_query_pool_results(
                self.query_pool,
                first,
                &mut ticks,
                vk::QueryResultFlags::TYPE_64 | vk::QueryResultFlags::WAIT,
            )
        }
        .ok()?;
        Some(Duration::from_nanos(timestamps.nanos(ticks[0], ticks[1])))
    }
}
