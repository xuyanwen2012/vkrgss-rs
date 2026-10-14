//! What the tests record of presentation and inject into it.
//!
//! [`Probe`] exists in every build. Outside the tests it has no field and its methods do
//! nothing, so the compositor calls them without an attribute.

use ash::vk;

#[cfg(test)]
use super::format::SRGB_FORMATS;
use crate::VkError;
#[cfg(test)]
use crate::compositor::VkCompositor;
use crate::compositor::pacer::TimingCalls;

/// The records and the injected failures of presentation.
#[derive(Default)]
pub(in crate::compositor) struct Probe {
    /// Failure injection: the number of acquires that still report a timeout.
    #[cfg(test)]
    acquire_timeouts: u32,
    /// Makes swapchains present as on a device without swapchain maintenance.
    #[cfg(test)]
    ignore_maintenance: bool,
    /// Makes swapchains take an sRGB format of the surface when it has one.
    #[cfg(test)]
    force_srgb: bool,
    /// Failure injection: whether the draw of the next present fails.
    #[cfg(test)]
    fail_draw: bool,
    /// The number of presents handed over with a present fence.
    #[cfg(test)]
    fenced_presents: u32,
    /// The number of times the device was idled to replace a swapchain.
    #[cfg(test)]
    swapchain_idles: u32,
    /// Stands in for the pacing extensions of the swapchain when the presents are planned.
    #[cfg(test)]
    pub(in crate::compositor) timing_stand_in: Option<Box<dyn TimingCalls>>,
}

#[cfg(test)]
impl Probe {
    /// Whether the acquire about to be made reports a timeout without asking the surface,
    /// which uses up one of the injected timeouts.
    pub(super) fn acquire_times_out(&mut self) -> bool {
        let injected = self.acquire_timeouts > 0;
        if injected {
            self.acquire_timeouts -= 1;
        }
        injected
    }

    /// Whether something stands in for the pacing extensions of the swapchain.
    pub(super) fn has_timing_stand_in(&self) -> bool {
        self.timing_stand_in.is_some()
    }

    /// The calls a present is planned with: the stand-in when there is one, else `calls`.
    pub(super) fn timing_calls<'a>(
        &'a mut self,
        calls: Option<&'a mut dyn TimingCalls>,
    ) -> Option<&'a mut dyn TimingCalls> {
        match &mut self.timing_stand_in {
            Some(calls) => Some(&mut **calls as &mut dyn TimingCalls),
            None => calls,
        }
    }

    /// Before the draw of a present is recorded: fails when that is the injected failure.
    pub(super) fn before_draw(&mut self) -> Result<(), VkError> {
        if std::mem::take(&mut self.fail_draw) {
            return Err(VkError::Memory {
                what: "the draw",
                message: "injected failure".to_owned(),
            });
        }
        Ok(())
    }

    /// Counts a present handed over with a fence.
    pub(super) fn present_fenced(&mut self) {
        self.fenced_presents += 1;
    }

    /// Whether swapchains are created as on a device without swapchain maintenance.
    pub(super) fn ignores_maintenance(&self) -> bool {
        self.ignore_maintenance
    }

    /// The formats a swapchain chooses from, of those the surface offers.
    pub(super) fn surface_formats(
        &self,
        formats: Vec<vk::SurfaceFormatKHR>,
    ) -> Vec<vk::SurfaceFormatKHR> {
        match self.force_srgb {
            true => srgb_formats_only(formats),
            false => formats,
        }
    }

    /// Counts an idling of the device to replace a swapchain.
    pub(super) fn swapchain_idled(&mut self) {
        self.swapchain_idles += 1;
    }
}

#[cfg(not(test))]
impl Probe {
    #[inline(always)]
    pub(super) fn acquire_times_out(&mut self) -> bool {
        false
    }

    #[inline(always)]
    pub(super) fn has_timing_stand_in(&self) -> bool {
        false
    }

    #[inline(always)]
    pub(super) fn timing_calls<'a>(
        &'a mut self,
        calls: Option<&'a mut dyn TimingCalls>,
    ) -> Option<&'a mut dyn TimingCalls> {
        calls
    }

    #[inline(always)]
    pub(super) fn before_draw(&mut self) -> Result<(), VkError> {
        Ok(())
    }

    #[inline(always)]
    pub(super) fn present_fenced(&mut self) {}

    #[inline(always)]
    pub(super) fn ignores_maintenance(&self) -> bool {
        false
    }

    #[inline(always)]
    pub(super) fn surface_formats(
        &self,
        formats: Vec<vk::SurfaceFormatKHR>,
    ) -> Vec<vk::SurfaceFormatKHR> {
        formats
    }

    #[inline(always)]
    pub(super) fn swapchain_idled(&mut self) {}
}

/// The sRGB formats among `formats` that [`choose_format`] knows, or all of `formats` when
/// there is none: what a surface without a `UNORM` format would offer.
#[cfg(test)]
fn srgb_formats_only(formats: Vec<vk::SurfaceFormatKHR>) -> Vec<vk::SurfaceFormatKHR> {
    let srgb: Vec<_> = formats
        .iter()
        .copied()
        .filter(|offered| {
            SRGB_FORMATS.contains(&offered.format)
                && offered.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .collect();
    match srgb.is_empty() {
        true => formats,
        false => srgb,
    }
}

#[cfg(test)]
impl VkCompositor {
    /// The extent of the current swapchain, if there is one.
    pub(crate) fn swapchain_extent(&self) -> Option<(u32, u32)> {
        let chain = self.presenter.as_ref()?.swapchain.as_ref()?;
        Some((chain.extent.width, chain.extent.height))
    }

    /// Makes the next `count` acquires of a swapchain image report that none arrived in
    /// time, without asking the surface.
    pub(crate) fn fail_acquires(&mut self, count: u32) {
        self.present_probe.acquire_timeouts = count;
    }

    /// Whether the last acquire gave no image, so that the next one does not wait.
    pub(crate) fn is_starved(&self) -> bool {
        self.starved
    }

    /// Makes the next present replace the swapchain, as an out-of-date report does.
    pub(crate) fn invalidate_swapchain(&mut self) {
        self.mark_swapchain_stale();
    }

    /// Makes the swapchains created from now on present without swapchain maintenance, as on
    /// a device that lacks the extension.
    pub(crate) fn ignore_swapchain_maintenance(&mut self) {
        self.present_probe.ignore_maintenance = true;
    }

    /// Makes the swapchains created from now on take an sRGB format of the surface when it
    /// has one, as on a surface that offers no `UNORM` format.
    pub(crate) fn force_srgb_surface_format(&mut self) {
        self.present_probe.force_srgb = true;
    }

    /// The format of the swapchain's images and that of the views the pass draws through.
    pub(crate) fn swapchain_formats(&self) -> Option<(vk::Format, vk::Format)> {
        let chain = self.presenter.as_ref()?.swapchain.as_ref()?;
        Some((chain.format.surface.format, chain.format.view))
    }

    /// The number of presents handed over with a fence so far.
    pub(crate) fn fenced_presents(&self) -> u32 {
        self.present_probe.fenced_presents
    }

    /// The number of times the device was idled to replace a swapchain so far.
    pub(crate) fn swapchain_idles(&self) -> u32 {
        self.present_probe.swapchain_idles
    }

    /// Makes the draw of the next present fail before anything is submitted, with an image
    /// acquired.
    pub(crate) fn fail_next_draw(&mut self) {
        self.present_probe.fail_draw = true;
    }

    /// Whether an image left undrawn by a failed draw is given back to a swapchain created
    /// now, and the swapchain is not replaced.
    pub(crate) fn releases_undrawn_images(&self) -> bool {
        self.uses_swapchain_maintenance() && self.gpu.releases_undrawn_images()
    }
}
