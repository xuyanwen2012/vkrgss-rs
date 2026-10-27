//! Pacing of the presents of a swapchain with `VK_EXT_present_timing`.
//!
//! [`Pacer`] decides what a present carries: a target time from the schedule, and whether
//! its visible time is asked for. It reaches the extension only through [`TimingCalls`],
//! which [`SwapchainTiming`] implements for a swapchain and a test implements with a
//! stand-in. The first error from any of those calls is kept as a report and ends the use of
//! the extension: every later present goes out as it does on a device without it.

use std::time::Instant;

use ash::vk;

use crate::VkError;
use crate::device::Gpu;
use crate::pacing::{IntervalSummary, PresentSchedule, PresentStats};
use crate::present_timing as ffi;

/// The call of the extension that failed, and its result.
pub(super) type Failure = (&'static str, vk::Result);

/// What the swapchain reports about earlier presents.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Feedback {
    /// How many presents were reported completely, each freeing a slot of the results queue.
    pub completed: u32,
    /// The times at which those presents became visible, oldest first, on the clock
    /// [`TimingCalls::now`] read before this call.
    pub visible: Vec<u64>,
    /// Whether [`TimingCalls::now`] reads another clock from now on. The times above are
    /// still on the one before.
    pub clock_changed: bool,
}

/// The calls of the extensions on one swapchain.
pub(super) trait TimingCalls {
    /// Whether [`wait`](TimingCalls::wait) can be used.
    fn can_wait(&self) -> bool;
    /// How many presents can await their feedback at a time. 0 when the surface reports no
    /// times, in which case [`feedback`](TimingCalls::feedback) is not called.
    fn queue_size(&self) -> u32;
    /// Waits until the present with this id has reached the display, at most `timeout`
    /// nanoseconds. Running out of time is not an error.
    fn wait(&mut self, present_id: u64, timeout: u64) -> Result<(), Failure>;
    fn feedback(&mut self) -> Result<Feedback, Failure>;
    /// The time domain the swapchain's times are read in, or `None` when none of those it
    /// offers can be read.
    fn time_domain(&self) -> Option<vk::TimeDomainKHR>;
    /// The time of the swapchain's clock, in nanoseconds, from
    /// `vkGetCalibratedTimestampsKHR`. Called only for a domain other than the monotonic
    /// clock.
    fn calibrated_now(&mut self) -> Result<u64, Failure>;
    /// Reads the swapchain's time domains again and chooses one of them.
    fn choose_domain(&mut self) -> Result<(), Failure>;
}

/// What one present carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Plan {
    /// The target time on the swapchain's clock. Never 0, which would mean none.
    pub target: u64,
    /// The id of the present, when a later one waits for it.
    pub present_id: Option<u64>,
    /// Whether the time the present becomes visible is asked for.
    pub query: bool,
    /// Whether the statistics come from the swapchain's feedback.
    feedback: bool,
}

/// The pacing state of a compositor's presents. It outlives the swapchains.
pub(super) struct Pacer {
    schedule: PresentSchedule,
    stats: PresentStats,
    /// Frames per second of the frame being shown.
    rate: u32,
    /// Whether a call of the extension has failed, after which none is made.
    failed: bool,
    /// The failure, until the owner takes it.
    report: Option<String>,
    /// The id the newest present with an id carried. Ids grow over all swapchains.
    newest_id: u64,
    /// The id of the newest present of the current swapchain, to wait for.
    awaited: Option<u64>,
    /// Presents whose feedback has not been read yet.
    outstanding: u32,
    /// Whether the newest time in the statistics came from the swapchain's feedback.
    from_feedback: bool,
    /// The origin of the presenter's own clock.
    origin: Instant,
    /// The call of the extension that found the device lost, if one has.
    lost: Option<&'static str>,
}

impl Pacer {
    pub(super) fn new() -> Pacer {
        Pacer {
            schedule: PresentSchedule::new(),
            stats: PresentStats::new(),
            rate: 40,
            failed: false,
            report: None,
            newest_id: 0,
            awaited: None,
            outstanding: 0,
            from_feedback: false,
            origin: Instant::now(),
            lost: None,
        }
    }

    pub(super) fn set_frame_rate(&mut self, rate: u32) {
        self.rate = rate.max(1);
    }

    pub(super) fn restart(&mut self) {
        self.schedule.restart();
    }

    /// Whether the extension is still to be used where a swapchain supports it.
    pub(super) fn wants_timing(&self) -> bool {
        !self.failed
    }

    /// Whether the presents of the swapchain with `calls` carry target times: the extension
    /// is still used and the swapchain has a time domain that can be read.
    pub(super) fn paces(&self, calls: Option<&dyn TimingCalls>) -> bool {
        self.wants_timing() && calls.is_some_and(|calls| calls.time_domain().is_some())
    }

    pub(super) fn summary(&self) -> Option<IntervalSummary> {
        self.stats.summary()
    }

    pub(super) fn take_report(&mut self) -> Option<String> {
        self.report.take()
    }

    /// A new swapchain presents from now on: nothing of the old one is waited for, and its
    /// clock may be another one, so no target of the old one is carried over. The statistics
    /// are only interrupted when their times come from that clock.
    pub(super) fn swapchain_replaced(&mut self) {
        self.awaited = None;
        self.outstanding = 0;
        self.schedule.restart();
        if self.from_feedback {
            self.stats.interrupt();
        }
    }

    /// The present of `plan` failed with `result`. Returns whether the failure is the
    /// extension's own, a full results queue, in which case the extension is not used from
    /// here on and the present is to be made again without it. The pacer only asks for a
    /// result while it counts a free slot, so a full queue means the count cannot be relied
    /// on.
    pub(super) fn present_failed(&mut self, plan: Option<&Plan>, result: vk::Result) -> bool {
        let full = plan.is_some() && result == ffi::ERROR_PRESENT_TIMING_QUEUE_FULL;
        if full {
            self.fail(("vkQueuePresentKHR", result));
        }
        full
    }

    /// Fails with [`VkError::DeviceLost`] when a call of the extension found the device lost,
    /// which the caller passes on instead of presenting without the extension.
    pub(super) fn check_device(&self) -> Result<(), VkError> {
        match self.lost {
            Some(call) => Err(VkError::DeviceLost { call }),
            None => Ok(()),
        }
    }

    /// A call of the extension failed: it is not used from here on. A lost device is kept
    /// for [`check_device`](Pacer::check_device).
    pub(super) fn fail(&mut self, (call, result): Failure) {
        if result == vk::Result::ERROR_DEVICE_LOST {
            self.lost.get_or_insert(call);
        }
        if !self.failed {
            self.failed = true;
            self.report = Some(format!(
                "`{call}` failed: {result}; presenting without target times from now on"
            ));
        }
        self.awaited = None;
        self.outstanding = 0;
    }

    /// What the next present carries, or `None` when it goes out without the extension:
    /// there are no `calls` for the swapchain, its clock cannot be read, or a call has
    /// failed, now or earlier.
    ///
    /// `frame` says whether the present shows a new frame. Only then does the schedule move
    /// on; anything else is shown at the target of the frame it repeats.
    pub(super) fn plan(
        &mut self,
        calls: Option<&mut dyn TimingCalls>,
        frame: bool,
    ) -> Option<Plan> {
        let calls = calls.filter(|_| !self.failed)?;
        match self.try_plan(calls, frame) {
            Ok(plan) => plan,
            Err(failure) => {
                self.fail(failure);
                None
            }
        }
    }

    fn try_plan(
        &mut self,
        calls: &mut dyn TimingCalls,
        frame: bool,
    ) -> Result<Option<Plan>, Failure> {
        let can_wait = calls.can_wait();
        if can_wait && let Some(id) = self.awaited {
            // Two periods: long enough for a frame that is on its way, short enough that a
            // window nobody shows does not stop the game.
            calls.wait(id, 2_000_000_000 / u64::from(self.rate))?;
        }
        let queue_size = calls.queue_size();
        if queue_size > 0 {
            let feedback = calls.feedback()?;
            self.outstanding = self.outstanding.saturating_sub(feedback.completed);
            // These times are still on the clock in use until now.
            for time in feedback.visible {
                self.stats.record(time);
            }
            if feedback.clock_changed {
                self.schedule.restart();
                self.stats.interrupt();
            }
        }
        let Some(mut now) = read_clock(calls)? else {
            return Ok(None);
        };
        if now == 0 {
            // The swapchain's clock is another one, as when its domains change.
            calls.choose_domain()?;
            self.schedule.restart();
            if self.from_feedback {
                self.stats.interrupt();
            }
            let Some(again) = read_clock(calls)? else {
                return Ok(None);
            };
            now = again;
        }
        let target = match frame {
            true => self.schedule.next(now, self.rate),
            false => self.schedule.current().unwrap_or(now),
        };
        let present_id = can_wait.then(|| {
            self.newest_id += 1;
            self.newest_id
        });
        Ok(Some(Plan {
            target: target.max(1),
            present_id,
            // A frame beyond what the results queue holds goes unmeasured.
            query: frame && self.outstanding < queue_size,
            feedback: queue_size > 0,
        }))
    }

    /// A present went out, with `plan` or without the extension.
    pub(super) fn presented(&mut self, plan: Option<&Plan>, frame: bool) {
        if let Some(plan) = plan {
            self.awaited = plan.present_id.or(self.awaited);
            self.outstanding += u32::from(plan.query);
        }
        if !frame {
            return;
        }
        let from_feedback = plan.is_some_and(|plan| plan.feedback);
        if from_feedback != self.from_feedback {
            // The two clocks have nothing to do with each other.
            self.stats.interrupt();
            self.from_feedback = from_feedback;
        }
        if !from_feedback {
            let now = self.origin.elapsed().as_nanos();
            self.stats.record(u64::try_from(now).unwrap_or(u64::MAX));
        }
    }
}

/// What the surface of a swapchain supports of the pacing extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SurfacePacing {
    /// The present stages whose times the surface reports.
    pub stage_queries: u32,
    /// Whether presents can carry ids and be waited for.
    pub wait: bool,
}

impl SurfacePacing {
    /// The flags a swapchain is created with to be paced.
    pub(super) fn swapchain_flags(&self) -> vk::SwapchainCreateFlagsKHR {
        let mut flags = ffi::SWAPCHAIN_CREATE_PRESENT_TIMING;
        if self.wait {
            flags |= ffi::SWAPCHAIN_CREATE_PRESENT_ID_2 | ffi::SWAPCHAIN_CREATE_PRESENT_WAIT_2;
        }
        flags
    }
}

/// Asks the surface whether presents to it can be given an absolute target time, on a device
/// that was opened with `VK_EXT_present_timing`. `Ok(None)` when they cannot.
pub(super) fn surface_pacing(gpu: &Gpu) -> Result<Option<SurfacePacing>, Failure> {
    let Some(surface) = &gpu.surface else {
        return Ok(None);
    };
    let Some(loader) = surface.capabilities2.as_ref().filter(|_| gpu.pacing.timing) else {
        return Ok(None);
    };
    let mut timing = ffi::PresentTimingSurfaceCapabilities::default();
    let mut id = ffi::SurfaceCapabilitiesPresentId2::default();
    let mut wait = ffi::SurfaceCapabilitiesPresentWait2::default();
    if gpu.pacing.wait {
        id.p_next = (&raw mut wait).cast();
        timing.p_next = (&raw mut id).cast();
    }
    let mut capabilities = vk::SurfaceCapabilities2KHR {
        p_next: (&raw mut timing).cast(),
        ..Default::default()
    };
    let info = vk::PhysicalDeviceSurfaceInfo2KHR::default().surface(surface.handle);
    // SAFETY: the physical device and the surface belong to the live instance, which has the
    // extension of this call. The chain holds live structures of extensions the device was
    // created with, each with the layout and the structure type of the header.
    unsafe {
        loader.get_physical_device_surface_capabilities2(gpu.physical, &info, &mut capabilities)
    }
    .map_err(|result| ("vkGetPhysicalDeviceSurfaceCapabilities2KHR", result))?;
    let supported = timing.present_timing_supported == vk::TRUE
        && timing.present_at_absolute_time_supported == vk::TRUE;
    Ok(supported.then_some(SurfacePacing {
        stage_queries: timing.present_stage_queries,
        wait: gpu.pacing.wait
            && id.present_id2_supported == vk::TRUE
            && wait.present_wait2_supported == vk::TRUE,
    }))
}

/// The present stage whose time stands for "visible": the latest of the stages the surface
/// reports, which is the one nearest to the display. 0 when it reports none.
fn visible_stage(stage_queries: u32) -> u32 {
    ffi::PRESENT_STAGES
        .into_iter()
        .rev()
        .find(|stage| stage_queries & stage != 0)
        .unwrap_or(0)
}

/// The time domain to use of those a swapchain offers, each with its id: the system's
/// monotonic clock if it is among them, else a clock of the swapchain, else the first. Any
/// other domain than the monotonic clock is used only when it is one of the `calibrateable`
/// domains of the device; `None` when no domain is left.
fn choose_time_domain(
    offered: &[(vk::TimeDomainKHR, u64)],
    calibrateable: &[vk::TimeDomainKHR],
) -> Option<(vk::TimeDomainKHR, u64)> {
    let usable: Vec<_> = offered
        .iter()
        .filter(|(domain, _)| {
            *domain == vk::TimeDomainKHR::CLOCK_MONOTONIC || calibrateable.contains(domain)
        })
        .collect();
    let preferred = [
        vk::TimeDomainKHR::CLOCK_MONOTONIC,
        ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL,
        ffi::TIME_DOMAIN_PRESENT_STAGE_LOCAL,
    ];
    preferred
        .iter()
        .find_map(|wanted| usable.iter().find(|(domain, _)| domain == wanted))
        .or(usable.first())
        .copied()
        .copied()
}

/// The time of the swapchain's clock in nanoseconds, or `None` when it has no domain that
/// can be read. The monotonic clock is read from the system.
fn read_clock(calls: &mut dyn TimingCalls) -> Result<Option<u64>, Failure> {
    match calls.time_domain() {
        None => Ok(None),
        Some(vk::TimeDomainKHR::CLOCK_MONOTONIC) => Ok(Some(monotonic_now())),
        Some(_) => calls.calibrated_now().map(Some),
    }
}

/// The time of `CLOCK_MONOTONIC` in nanoseconds, which is what
/// `VK_TIME_DOMAIN_CLOCK_MONOTONIC_KHR` is defined as.
fn monotonic_now() -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: the pointer is to a live `timespec`, and the clock exists on every Linux.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) };
    // Neither field of the monotonic clock is negative.
    (time.tv_sec as u64) * 1_000_000_000 + time.tv_nsec as u64
}

/// How many slots the results queue of a swapchain gets. Feedback is read before every
/// present, so this is how many presents the presentation engine may take to report.
const QUEUE_SIZE: u32 = 32;

/// How many reports are read at a time.
const BATCH: usize = 16;

/// The pacing extensions on one swapchain. It holds no object of its own and is dropped
/// with the swapchain.
pub(super) struct SwapchainTiming {
    commands: ffi::Commands,
    calibrated: ash::khr::calibrated_timestamps::Device,
    device: vk::Device,
    swapchain: vk::SwapchainKHR,
    /// The stage whose time is asked for, or 0 when the surface reports none.
    stage: u32,
    wait: bool,
    /// The time domains the device can calibrate.
    calibrateable: Vec<vk::TimeDomainKHR>,
    /// The time domain in use with its id, or `None` when the swapchain offers none that can
    /// be read.
    clock: Option<(vk::TimeDomainKHR, u64)>,
    /// The swapchain's counter of its time domains when the domain was chosen.
    domains_counter: u64,
}

impl SwapchainTiming {
    /// Prepares a swapchain that was created with the flags of `pacing` on a device that was
    /// opened with the extensions: allocates its results queue and chooses its clock from the
    /// domains it offers and the device's `calibrateable` ones.
    ///
    /// # Safety
    ///
    /// `swapchain` must be a live swapchain of `device`, created with
    /// [`SurfacePacing::swapchain_flags`] of `pacing`, and must outlive the value returned;
    /// `commands` and `calibrated` must have been loaded for `device`.
    pub(super) unsafe fn new(
        commands: ffi::Commands,
        calibrated: ash::khr::calibrated_timestamps::Device,
        device: vk::Device,
        swapchain: vk::SwapchainKHR,
        pacing: SurfacePacing,
        calibrateable: Vec<vk::TimeDomainKHR>,
    ) -> Result<SwapchainTiming, Failure> {
        let mut timing = SwapchainTiming {
            commands,
            calibrated,
            device,
            swapchain,
            stage: visible_stage(pacing.stage_queries),
            wait: pacing.wait && commands.wait_for_present2.is_some(),
            calibrateable,
            clock: None,
            domains_counter: 0,
        };
        if timing.stage != 0 {
            // SAFETY: the swapchain is live and was created for present timing.
            let result = unsafe {
                (commands.set_swapchain_present_timing_queue_size)(device, swapchain, QUEUE_SIZE)
            };
            if result != vk::Result::SUCCESS {
                return Err(("vkSetSwapchainPresentTimingQueueSizeEXT", result));
            }
        }
        timing.choose_domain()?;
        Ok(timing)
    }

    /// The stage a target time and a reading of the clock refer to where the clock is one of
    /// a present stage.
    fn target_stage(&self) -> u32 {
        match self.stage {
            0 => ffi::PRESENT_STAGES[3],
            stage => stage,
        }
    }

    /// The id of the time domain in use. Only a swapchain with one gets a plan.
    fn domain_id(&self) -> u64 {
        self.clock.map_or(0, |(_, id)| id)
    }

    /// The timing information the present of `plan` carries.
    pub(super) fn present_info(&self, plan: &Plan) -> ffi::PresentTimingInfo {
        ffi::PresentTimingInfo {
            target_time: plan.target,
            time_domain_id: self.domain_id(),
            present_stage_queries: if plan.query { self.stage } else { 0 },
            target_time_domain_present_stage: self.target_stage(),
            ..Default::default()
        }
    }
}

impl TimingCalls for SwapchainTiming {
    fn can_wait(&self) -> bool {
        self.wait
    }

    fn queue_size(&self) -> u32 {
        if self.stage != 0 { QUEUE_SIZE } else { 0 }
    }

    fn wait(&mut self, present_id: u64, timeout: u64) -> Result<(), Failure> {
        let Some(wait_for_present2) = self.commands.wait_for_present2 else {
            return Ok(());
        };
        let info = ffi::PresentWait2Info {
            present_id,
            timeout,
            ..Default::default()
        };
        // SAFETY: the swapchain is live and was created for present waits (see `new`); the
        // id is one a successful present of this swapchain carried, as the pacer keeps it.
        match unsafe { wait_for_present2(self.device, self.swapchain, &info) } {
            // An out-of-date swapchain is replaced by the present that follows.
            vk::Result::SUCCESS
            | vk::Result::TIMEOUT
            | vk::Result::SUBOPTIMAL_KHR
            | vk::Result::ERROR_OUT_OF_DATE_KHR => Ok(()),
            result => Err(("vkWaitForPresent2KHR", result)),
        }
    }

    fn feedback(&mut self) -> Result<Feedback, Failure> {
        let mut stages = [ffi::PresentStageTime::default(); BATCH];
        let mut timings = [ffi::PastPresentationTiming::default(); BATCH];
        for (index, timing) in timings.iter_mut().enumerate() {
            // One stage is asked for with every present.
            timing.present_stage_count = 1;
            // SAFETY: the index is within the array.
            timing.p_present_stages = unsafe { stages.as_mut_ptr().add(index) };
        }
        let info = ffi::PastPresentationTimingInfo {
            swapchain: self.swapchain,
            ..Default::default()
        };
        let mut properties = ffi::PastPresentationTimingProperties {
            presentation_timing_count: BATCH as u32,
            p_presentation_timings: timings.as_mut_ptr(),
            ..Default::default()
        };
        // SAFETY: the swapchain is live (see `new`); the properties point to `BATCH` reports,
        // each of which points to room for the one stage every present asks for.
        let result = unsafe {
            (self.commands.get_past_presentation_timing)(self.device, &info, &mut properties)
        };
        match result {
            // What did not fit is read before the next present.
            vk::Result::SUCCESS | vk::Result::INCOMPLETE => {}
            // The present that follows replaces the swapchain.
            vk::Result::ERROR_OUT_OF_DATE_KHR => return Ok(Feedback::default()),
            result => return Err(("vkGetPastPresentationTimingEXT", result)),
        }
        let mut feedback = Feedback::default();
        let written = (properties.presentation_timing_count as usize).min(BATCH);
        for (timing, stage) in timings.iter().zip(&stages).take(written) {
            if timing.report_complete != vk::TRUE {
                continue;
            }
            feedback.completed += 1;
            // A time of 0 means the stage has none; a time on another clock than the one in
            // use cannot be compared with the others.
            if timing.present_stage_count > 0
                && stage.time != 0
                && self.clock.is_some()
                && timing.time_domain_id == self.domain_id()
            {
                feedback.visible.push(stage.time);
            }
        }
        if properties.time_domains_counter != self.domains_counter {
            self.choose_domain()?;
            feedback.clock_changed = true;
        }
        Ok(feedback)
    }

    fn time_domain(&self) -> Option<vk::TimeDomainKHR> {
        self.clock.map(|(domain, _)| domain)
    }

    fn calibrated_now(&mut self) -> Result<u64, Failure> {
        let (domain, domain_id) = self
            .clock
            .ok_or(("vkGetCalibratedTimestampsKHR", vk::Result::ERROR_UNKNOWN))?;
        let local = [
            ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL,
            ffi::TIME_DOMAIN_PRESENT_STAGE_LOCAL,
        ]
        .contains(&domain);
        let swapchain_info = ffi::SwapchainCalibratedTimestampInfo {
            swapchain: self.swapchain,
            present_stage: match domain {
                ffi::TIME_DOMAIN_PRESENT_STAGE_LOCAL => self.target_stage(),
                _ => 0,
            },
            time_domain_id: domain_id,
            ..Default::default()
        };
        let mut info = vk::CalibratedTimestampInfoKHR::default().time_domain(domain);
        if local {
            info.p_next = (&raw const swapchain_info).cast();
        }
        // SAFETY: the device was created with the calibrated-timestamps extension; the
        // domain is one the live swapchain offers and the device lists as calibrateable, and
        // a domain of the swapchain comes with the structure that names the swapchain, which
        // has the layout of the header.
        let (timestamps, _) = unsafe { self.calibrated.get_calibrated_timestamps(&[info]) }
            .map_err(|result| ("vkGetCalibratedTimestampsKHR", result))?;
        Ok(timestamps[0])
    }

    fn choose_domain(&mut self) -> Result<(), Failure> {
        const CALL: &str = "vkGetSwapchainTimeDomainPropertiesEXT";
        let query = |properties: &mut ffi::SwapchainTimeDomainProperties, counter: &mut u64| {
            // SAFETY: the swapchain is live (see `new`), and the arrays the properties point
            // to, if any, hold as many elements as its count says.
            let result = unsafe {
                (self.commands.get_swapchain_time_domain_properties)(
                    self.device,
                    self.swapchain,
                    properties,
                    counter,
                )
            };
            match result {
                vk::Result::SUCCESS | vk::Result::INCOMPLETE => Ok(()),
                result => Err((CALL, result)),
            }
        };
        let mut counter = 0;
        let mut properties = ffi::SwapchainTimeDomainProperties::default();
        query(&mut properties, &mut counter)?;
        let count = properties.time_domain_count as usize;
        let mut domains = vec![vk::TimeDomainKHR::DEVICE; count];
        let mut ids = vec![0_u64; count];
        if count > 0 {
            properties.p_time_domains = domains.as_mut_ptr();
            properties.p_time_domain_ids = ids.as_mut_ptr();
            query(&mut properties, &mut counter)?;
        }
        let written = (properties.time_domain_count as usize).min(count);
        let offered: Vec<_> = domains.into_iter().zip(ids).take(written).collect();
        // Without a domain the swapchain presents as one without the extension.
        self.clock = choose_time_domain(&offered, &self.calibrateable);
        if self.clock.is_none() {
            // As on the Pixel 7a, whose driver offers only a clock of a present stage and
            // does not list it among the domains it calibrates, so that it cannot be read.
            let offered: Vec<_> = offered.iter().map(|(domain, _)| *domain).collect();
            tracing::info!(
                "present timing: the swapchain offers no time domain that can be read \
                 (it offers {offered:?}, the device calibrates {:?}); frames are paced by \
                 the timer",
                self.calibrateable
            );
        }
        self.domains_counter = counter;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    /// A swapchain that reports every present as visible at its target, one call late, and
    /// fails a chosen call from a chosen moment on. Its clock is read with the calibrated
    /// call unless it is the monotonic clock; by default it offers its own clock, which the
    /// device lists.
    struct StandIn {
        /// The time of its own clock.
        now: u64,
        wait: bool,
        queue_size: u32,
        /// The call that fails, if any.
        failing: Option<&'static str>,
        /// What the call that fails returns.
        failing_result: vk::Result,
        /// The names of the calls made, in order.
        calls: Vec<&'static str>,
        waited_for: Vec<(u64, u64)>,
        /// Visible times not reported yet.
        pending: Vec<u64>,
        /// Whether the next feedback says that the clock is another one from then on.
        clock_changes: bool,
        /// The domains it offers, with their ids.
        offered: Vec<(vk::TimeDomainKHR, u64)>,
        /// The domains the device can calibrate.
        calibrateable: Vec<vk::TimeDomainKHR>,
        domain: Option<vk::TimeDomainKHR>,
        /// The time its clock reads once the domain has been chosen again.
        rechosen_now: u64,
    }

    impl Default for StandIn {
        fn default() -> StandIn {
            StandIn::offering(
                vec![(ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL, 1)],
                vec![ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL],
            )
        }
    }

    impl StandIn {
        /// A stand-in whose domain is chosen as a swapchain's is when it is created.
        fn offering(
            offered: Vec<(vk::TimeDomainKHR, u64)>,
            calibrateable: Vec<vk::TimeDomainKHR>,
        ) -> StandIn {
            StandIn {
                now: 0,
                wait: false,
                queue_size: 0,
                failing: None,
                failing_result: vk::Result::ERROR_UNKNOWN,
                calls: Vec::new(),
                waited_for: Vec::new(),
                pending: Vec::new(),
                clock_changes: false,
                domain: choose_time_domain(&offered, &calibrateable).map(|(domain, _)| domain),
                offered,
                calibrateable,
                rechosen_now: 0,
            }
        }

        fn call(&mut self, name: &'static str) -> Result<(), Failure> {
            self.calls.push(name);
            match self.failing {
                Some(failing) if failing == name => Err((name, self.failing_result)),
                _ => Ok(()),
            }
        }
    }

    impl TimingCalls for StandIn {
        fn can_wait(&self) -> bool {
            self.wait
        }

        fn queue_size(&self) -> u32 {
            self.queue_size
        }

        fn wait(&mut self, present_id: u64, timeout: u64) -> Result<(), Failure> {
            self.call("wait")?;
            self.waited_for.push((present_id, timeout));
            Ok(())
        }

        fn feedback(&mut self) -> Result<Feedback, Failure> {
            self.call("feedback")?;
            let visible = std::mem::take(&mut self.pending);
            Ok(Feedback {
                completed: visible.len() as u32,
                visible,
                clock_changed: std::mem::take(&mut self.clock_changes),
            })
        }

        fn time_domain(&self) -> Option<vk::TimeDomainKHR> {
            self.domain
        }

        fn calibrated_now(&mut self) -> Result<u64, Failure> {
            self.call("calibrated")?;
            Ok(self.now)
        }

        fn choose_domain(&mut self) -> Result<(), Failure> {
            self.call("choose")?;
            self.domain =
                choose_time_domain(&self.offered, &self.calibrateable).map(|(domain, _)| domain);
            self.now = self.rechosen_now;
            Ok(())
        }
    }

    /// Presents one frame through the pacer, as the presenter does.
    fn present(pacer: &mut Pacer, swapchain: &mut StandIn) -> Option<Plan> {
        let plan = pacer.plan(Some(swapchain), true);
        if let Some(plan) = &plan
            && plan.query
        {
            swapchain.pending.push(plan.target);
        }
        pacer.presented(plan.as_ref(), true);
        plan
    }

    /// A present that finds no swapchain image shows nothing and plans nothing: the frame
    /// it could not show gets the target one period after the frame before it, however many
    /// presents found no image in between.
    #[test]
    fn presents_without_an_image_do_not_move_the_target_on() {
        use crate::tests::harness::{assert_clean, test_device};
        use crate::{ValidationLog, VkCompositor};
        use vkrgss_proto::{FramePacket, FrameRate, Size};

        let log = ValidationLog::new();
        let mut compositor =
            VkCompositor::with_headless_surface(test_device(), Some(&log)).unwrap();
        // A fixed clock; no ids and no results, so the presents themselves go out plain.
        compositor.present_probe.timing_stand_in = Some(Box::new(StandIn {
            now: 1000 * MS,
            ..StandIn::default()
        }));
        compositor.set_frame_rate(40);
        compositor.resize_surface(Size::new(64, 48));
        let packet = FramePacket {
            frame_index: 0,
            logical_size: Size::new(20, 18),
            frame_rate: FrameRate::DEFAULT,
            brightness: 255,
            tables: vec![],
            bitmaps: vec![],
            ops: vec![],
        };
        let target = |compositor: &VkCompositor| compositor.pacer.schedule.current();

        compositor.render(&packet).unwrap();
        assert_eq!(compositor.present(), Ok(true));
        assert_eq!(target(&compositor), Some(1000 * MS));

        compositor.render(&packet).unwrap();
        compositor.fail_acquires(2);
        for _ in 0..2 {
            assert_eq!(compositor.present(), Ok(false));
            assert_eq!(target(&compositor), Some(1000 * MS));
        }
        assert_eq!(compositor.present(), Ok(true));
        assert_eq!(target(&compositor), Some(1025 * MS));
        // The frame was shown; a present that repeats it keeps its target.
        assert_eq!(compositor.present(), Ok(true));
        assert_eq!(target(&compositor), Some(1025 * MS));

        assert_clean(&log, "while presenting with a stand-in clock");
        drop(compositor);
        assert_clean(&log, "while the compositor was dropped");
    }

    /// A call of the extension that finds the device lost ends the present as a lost device,
    /// before the draw, instead of presenting without the extension; nothing is submitted
    /// after it.
    #[test]
    fn lost_device_in_a_pacing_call_fails_the_present() {
        use crate::VkError;
        use crate::tests::harness::{assert_clean, test_device};
        use crate::{ValidationLog, VkCompositor};
        use vkrgss_proto::{FramePacket, FrameRate, Size};

        let log = ValidationLog::new();
        let mut compositor =
            VkCompositor::with_headless_surface(test_device(), Some(&log)).unwrap();
        compositor.present_probe.timing_stand_in = Some(Box::new(StandIn {
            now: 1000 * MS,
            ..StandIn::default()
        }));
        compositor.resize_surface(Size::new(64, 48));
        let packet = FramePacket {
            frame_index: 0,
            logical_size: Size::new(20, 18),
            frame_rate: FrameRate::DEFAULT,
            brightness: 255,
            tables: vec![],
            bitmaps: vec![],
            ops: vec![],
        };
        compositor.render(&packet).unwrap();
        assert_eq!(compositor.present(), Ok(true));
        compositor.present_probe.timing_stand_in = Some(Box::new(StandIn {
            now: 1000 * MS,
            failing: Some("calibrated"),
            failing_result: vk::Result::ERROR_DEVICE_LOST,
            ..StandIn::default()
        }));
        compositor.render(&packet).unwrap();
        let submissions = compositor.submissions();
        let lost = Err(VkError::DeviceLost { call: "calibrated" });
        assert_eq!(compositor.present().map(|_| ()), lost);
        assert_eq!(compositor.submissions(), submissions, "nothing was drawn");
        assert_eq!(compositor.present().map(|_| ()), lost);
        assert_eq!(compositor.render(&packet), lost);
        assert_eq!(compositor.submissions(), submissions);
        assert_clean(&log, "around the lost device");
    }

    #[test]
    fn frames_get_targets_one_period_apart_and_ids_that_the_next_present_waits_for() {
        let mut pacer = Pacer::new();
        let mut swapchain = StandIn {
            now: 1000 * MS,
            wait: true,
            queue_size: 4,
            ..StandIn::default()
        };
        let mut targets = Vec::new();
        for _ in 0..4 {
            let plan = present(&mut pacer, &mut swapchain).unwrap();
            assert!(plan.query);
            targets.push((plan.target, plan.present_id));
            swapchain.now += 25 * MS;
        }
        assert_eq!(
            targets,
            [(1000, 1), (1025, 2), (1050, 3), (1075, 4)].map(|(ms, id)| (ms * MS, Some(id)))
        );
        // Each present after the first waited for the one before it, two periods at most.
        assert_eq!(swapchain.waited_for, [1, 2, 3].map(|id| (id, 50 * MS)));
        assert_eq!(
            &swapchain.calls[..5],
            ["feedback", "calibrated", "wait", "feedback", "calibrated"]
        );
        // Three reports have been read: two intervals of one period.
        let summary = pacer.summary().unwrap();
        assert_eq!(
            (summary.count, summary.smallest, summary.largest),
            (2, 25 * MS, 25 * MS)
        );
        assert_eq!(pacer.take_report(), None);

        pacer.set_frame_rate(20);
        let plan = present(&mut pacer, &mut swapchain).unwrap();
        assert_eq!(plan.target, 1125 * MS, "Scenario \"Period change\"");
        assert_eq!(swapchain.waited_for.last(), Some(&(4, 100 * MS)));

        pacer.restart();
        swapchain.now = 5000 * MS;
        assert_eq!(
            present(&mut pacer, &mut swapchain).unwrap().target,
            5000 * MS
        );
    }

    #[test]
    fn present_that_shows_no_new_frame_repeats_the_target_and_is_not_measured() {
        let mut pacer = Pacer::new();
        let mut swapchain = StandIn {
            now: 10 * MS,
            queue_size: 4,
            ..StandIn::default()
        };
        // Before any frame the target is the clock's time.
        let plan = pacer.plan(Some(&mut swapchain), false).unwrap();
        pacer.presented(Some(&plan), false);
        assert_eq!(
            (plan.target, plan.query, plan.present_id),
            (10 * MS, false, None)
        );

        let frame = present(&mut pacer, &mut swapchain).unwrap();
        for _ in 0..3 {
            let again = pacer.plan(Some(&mut swapchain), false).unwrap();
            pacer.presented(Some(&again), false);
            assert_eq!((again.target, again.query), (frame.target, false));
        }
        assert_eq!(
            present(&mut pacer, &mut swapchain).unwrap().target,
            frame.target + 25 * MS
        );
    }

    #[test]
    fn frames_beyond_the_results_queue_are_not_asked_about() {
        let mut pacer = Pacer::new();
        let mut swapchain = StandIn {
            queue_size: 2,
            ..StandIn::default()
        };
        // Nothing is ever reported.
        let queries: Vec<bool> = (0..4)
            .map(|_| {
                let plan = pacer.plan(Some(&mut swapchain), true).unwrap();
                pacer.presented(Some(&plan), true);
                plan.query
            })
            .collect();
        assert_eq!(queries, [true, true, false, false]);
    }

    /// The run-time fallback: a failing call is reported once and the extension is left
    /// alone from then on, with the statistics taken from the presenter's own clock.
    #[test]
    fn failing_call_is_reported_once_and_presents_go_on_without_target_times() {
        for failing in ["wait", "feedback", "calibrated"] {
            let mut pacer = Pacer::new();
            let mut swapchain = StandIn {
                wait: true,
                queue_size: 4,
                ..StandIn::default()
            };
            assert!(present(&mut pacer, &mut swapchain).is_some());
            assert!(present(&mut pacer, &mut swapchain).is_some());
            assert!(pacer.wants_timing());

            swapchain.failing = Some(failing);
            assert_eq!(present(&mut pacer, &mut swapchain), None, "{failing}");
            assert!(!pacer.wants_timing());
            let report = pacer.take_report().expect("the failure is reported");
            assert!(
                report.contains(failing) && report.contains("without target times"),
                "{report}"
            );

            // Even when the call would work again, nothing is asked of the swapchain.
            swapchain.failing = None;
            let calls = swapchain.calls.len();
            let measured = pacer.summary().map_or(0, |summary| summary.count);
            for _ in 0..3 {
                assert_eq!(present(&mut pacer, &mut swapchain), None);
            }
            assert_eq!(swapchain.calls.len(), calls);
            assert_eq!(pacer.take_report(), None, "reported once");
            // The presents since the failure are measured by the presenter's clock.
            assert_eq!(pacer.summary().unwrap().count, measured + 3);
        }
    }

    #[test]
    fn swapchain_without_the_extension_is_measured_by_the_presenters_clock() {
        let mut pacer = Pacer::new();
        for _ in 0..3 {
            assert_eq!(pacer.plan(None, true), None);
            pacer.presented(None, true);
            // A present that repeats the frame is no frame.
            assert_eq!(pacer.plan(None, false), None);
            pacer.presented(None, false);
        }
        assert_eq!(pacer.summary().unwrap().count, 2);
        assert!(pacer.wants_timing());
        assert_eq!(pacer.take_report(), None);
    }

    /// With and without times reported by the surface: the clock of the new swapchain may
    /// be another one either way.
    #[test]
    fn replaced_swapchain_is_not_waited_for_and_starts_the_schedule_from_its_clock() {
        for queue_size in [4, 0] {
            let mut pacer = Pacer::new();
            let mut old = StandIn {
                now: 1000 * MS,
                wait: true,
                queue_size,
                ..StandIn::default()
            };
            present(&mut pacer, &mut old);
            present(&mut pacer, &mut old);
            pacer.swapchain_replaced();
            let mut new = StandIn {
                now: 7 * MS,
                wait: true,
                queue_size,
                ..StandIn::default()
            };
            let plan = present(&mut pacer, &mut new).unwrap();
            assert_eq!(
                (plan.target, plan.present_id),
                (7 * MS, Some(3)),
                "ids keep growing; results queue of {queue_size}"
            );
            assert_eq!(new.waited_for, []);
            new.now += 25 * MS;
            assert_eq!(present(&mut pacer, &mut new).unwrap().target, 32 * MS);
        }
    }

    #[test]
    fn replaced_swapchain_keeps_the_intervals_measured_by_the_presenters_clock() {
        let mut pacer = Pacer::new();
        for _ in 0..3 {
            pacer.presented(None, true);
        }
        pacer.swapchain_replaced();
        pacer.presented(None, true);
        assert_eq!(pacer.summary().unwrap().count, 3);
    }

    /// The feedback that announces another clock still carries times of the one before:
    /// they are measured against each other, and no interval spans the two clocks.
    #[test]
    fn times_of_the_clock_before_a_change_are_not_measured_against_those_after_it() {
        let mut pacer = Pacer::new();
        let mut swapchain = StandIn {
            now: 1000 * MS,
            queue_size: 4,
            ..StandIn::default()
        };
        // Visible at 1000 and 1025 ms of the first clock.
        present(&mut pacer, &mut swapchain);
        swapchain.now += 25 * MS;
        present(&mut pacer, &mut swapchain);

        // The feedback read for the third frame reports 1025 ms and the change.
        swapchain.clock_changes = true;
        swapchain.now = 7 * MS;
        let mut targets = Vec::new();
        for _ in 0..3 {
            targets.push(present(&mut pacer, &mut swapchain).unwrap().target);
            swapchain.now += 25 * MS;
        }
        assert_eq!(targets, [7, 32, 57].map(|ms| ms * MS), "from the new clock");
        // Read so far: 1000 and 1025 ms, then 7 and 32 ms.
        let summary = pacer.summary().unwrap();
        assert_eq!(
            (summary.count, summary.smallest, summary.largest),
            (2, 25 * MS, 25 * MS)
        );
    }

    /// The run-time fallback for the one error of the extension that the present itself
    /// returns.
    #[test]
    fn present_refused_for_a_full_results_queue_is_reported_once_and_made_again_untimed() {
        let mut pacer = Pacer::new();
        let mut swapchain = StandIn {
            wait: true,
            queue_size: 4,
            ..StandIn::default()
        };
        assert!(present(&mut pacer, &mut swapchain).is_some());

        // Any other failure of a present is not the extension's.
        let plan = pacer.plan(Some(&mut swapchain), true).unwrap();
        assert!(!pacer.present_failed(Some(&plan), vk::Result::ERROR_DEVICE_LOST));
        assert!(!pacer.present_failed(None, ffi::ERROR_PRESENT_TIMING_QUEUE_FULL));
        assert!(pacer.wants_timing());
        assert_eq!(pacer.take_report(), None);

        assert!(pacer.present_failed(Some(&plan), ffi::ERROR_PRESENT_TIMING_QUEUE_FULL));
        assert!(!pacer.wants_timing());
        let report = pacer.take_report().expect("the failure is reported");
        assert!(
            report.contains("vkQueuePresentKHR") && report.contains("without target times"),
            "{report}"
        );

        // The present is made again, and every later one, with nothing of the extension.
        let calls = swapchain.calls.len();
        for _ in 0..3 {
            assert_eq!(present(&mut pacer, &mut swapchain), None);
        }
        assert_eq!(swapchain.calls.len(), calls);
        assert_eq!(pacer.take_report(), None, "reported once");
        assert_eq!(ffi::ERROR_PRESENT_TIMING_QUEUE_FULL.as_raw(), -1000208000);
    }

    #[test]
    fn visible_stage_is_the_latest_the_surface_reports() {
        assert_eq!(visible_stage(0xF), 0x8);
        assert_eq!(visible_stage(0x7), 0x4);
        assert_eq!(visible_stage(0x3), 0x2);
        assert_eq!(visible_stage(0x1), 0x1);
        assert_eq!(visible_stage(0), 0);
        assert_eq!(visible_stage(0x10), 0, "a stage this binding does not know");
    }

    #[test]
    fn time_domain_is_the_monotonic_clock_then_a_listed_clock_of_the_swapchain() {
        let stage = (ffi::TIME_DOMAIN_PRESENT_STAGE_LOCAL, 11);
        let swapchain = (ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL, 12);
        let monotonic = (vk::TimeDomainKHR::CLOCK_MONOTONIC, 13);
        let device = (vk::TimeDomainKHR::DEVICE, 14);
        let all = [stage.0, swapchain.0, device.0];
        assert_eq!(
            choose_time_domain(&[stage, swapchain, monotonic], &[]),
            Some(monotonic),
            "the monotonic clock needs no listing"
        );
        assert_eq!(
            choose_time_domain(&[device, stage, swapchain], &all),
            Some(swapchain)
        );
        assert_eq!(choose_time_domain(&[device, stage], &all), Some(stage));
        assert_eq!(choose_time_domain(&[device], &all), Some(device));
        assert_eq!(choose_time_domain(&[], &all), None);
        // Only listed domains other than the monotonic clock.
        assert_eq!(
            choose_time_domain(&[device, stage, swapchain], &[stage.0]),
            Some(stage)
        );
        assert_eq!(
            choose_time_domain(&[device, stage, swapchain], &[device.0]),
            Some(device)
        );
        assert_eq!(choose_time_domain(&[swapchain], &[]), None);
    }

    #[test]
    fn monotonic_clock_is_read_from_the_system() {
        let mut pacer = Pacer::new();
        let mut swapchain = StandIn::offering(
            vec![
                (ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL, 1),
                (vk::TimeDomainKHR::CLOCK_MONOTONIC, 2),
            ],
            vec![ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL],
        );
        assert_eq!(swapchain.domain, Some(vk::TimeDomainKHR::CLOCK_MONOTONIC));
        let before = monotonic_now();
        let plan = present(&mut pacer, &mut swapchain).unwrap();
        let after = monotonic_now();
        assert!(
            (before..=after).contains(&plan.target),
            "Scenario \"Monotonic clock\""
        );
        assert!(!swapchain.calls.contains(&"calibrated"));
    }

    #[test]
    fn swapchain_clock_the_device_cannot_calibrate_is_not_used() {
        let mut pacer = Pacer::new();
        let mut swapchain = StandIn::offering(
            vec![(ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL, 1)],
            vec![vk::TimeDomainKHR::DEVICE],
        );
        swapchain.queue_size = 4;
        assert_eq!(swapchain.domain, None);
        assert!(!pacer.paces(Some(&swapchain)));
        for _ in 0..3 {
            assert_eq!(
                present(&mut pacer, &mut swapchain),
                None,
                "Scenario \"Swapchain domain the device cannot calibrate\""
            );
        }
        assert!(!swapchain.calls.contains(&"calibrated"));
        // Not a failure: a later swapchain may have a clock that can be read.
        assert!(pacer.wants_timing());
        assert_eq!(pacer.take_report(), None);
        // Measured by the presenter's clock, as without the extension.
        assert_eq!(pacer.summary().unwrap().count, 2);
    }

    #[test]
    fn swapchain_clock_the_device_lists_is_read_calibrated() {
        let mut pacer = Pacer::new();
        let mut swapchain = StandIn::offering(
            vec![(ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL, 1)],
            vec![vk::TimeDomainKHR::DEVICE, ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL],
        );
        swapchain.now = 1000 * MS;
        assert_eq!(swapchain.domain, Some(ffi::TIME_DOMAIN_SWAPCHAIN_LOCAL));
        let plan = present(&mut pacer, &mut swapchain).unwrap();
        assert_eq!(
            plan.target,
            1000 * MS,
            "Scenario \"Swapchain domain the device can calibrate\""
        );
        assert_eq!(swapchain.calls, ["calibrated"]);
        assert!(pacer.paces(Some(&swapchain)));
        assert!(!pacer.paces(None));
    }

    #[test]
    fn time_of_zero_chooses_the_domain_again_and_restarts_the_schedule() {
        let mut pacer = Pacer::new();
        let mut swapchain = StandIn {
            now: 1000 * MS,
            queue_size: 4,
            ..StandIn::default()
        };
        // Visible at 1000 and 1025 ms of the first clock.
        present(&mut pacer, &mut swapchain);
        swapchain.now += 25 * MS;
        present(&mut pacer, &mut swapchain);

        swapchain.now = 0;
        swapchain.rechosen_now = 7 * MS;
        let mut targets = Vec::new();
        for _ in 0..3 {
            targets.push(present(&mut pacer, &mut swapchain).unwrap().target);
            swapchain.now += 25 * MS;
        }
        assert_eq!(
            targets,
            [7, 32, 57].map(|ms| ms * MS),
            "Scenario \"A time of zero\""
        );
        assert_eq!(
            swapchain.calls[4..8],
            ["feedback", "calibrated", "choose", "calibrated"]
        );
        // Read so far: 1000 and 1025 ms, then 7 and 32 ms; no interval spans the two clocks.
        let summary = pacer.summary().unwrap();
        assert_eq!(
            (summary.count, summary.smallest, summary.largest),
            (2, 25 * MS, 25 * MS)
        );
        assert_eq!(pacer.take_report(), None);

        // Chosen again, the swapchain may have no clock left that can be read.
        assert!(pacer.paces(Some(&swapchain)));
        swapchain.now = 0;
        swapchain.calibrateable.clear();
        assert_eq!(present(&mut pacer, &mut swapchain), None);
        assert!(pacer.wants_timing());
        assert!(!pacer.paces(Some(&swapchain)));
    }

    #[test]
    fn paced_swapchain_is_created_with_the_flags_of_what_it_uses() {
        let timing = SurfacePacing {
            stage_queries: 0xF,
            wait: false,
        };
        assert_eq!(timing.swapchain_flags().as_raw(), 0x200);
        let both = SurfacePacing {
            wait: true,
            ..timing
        };
        assert_eq!(both.swapchain_flags().as_raw(), 0x200 | 0x40 | 0x80);
    }
}
