//! What the compositor is told about presentation and what it tells: the mix, the filter,
//! the mask, the frame rate and the size of the window.

use ash::vk;
use vkrgss_proto::{BitmapId, Size};

use super::scaling::{Filter, MAX_VAGUENESS, Mask, Mix};
use crate::compositor::VkCompositor;
use crate::pacing::IntervalSummary;

impl VkCompositor {
    /// Sets what is shown of the held and the newest frame from now on: the newest frame
    /// alone, or a step of a transition, mixed per channel on the frames' 8-bit values before
    /// scaling. It starts as [`Mix::Newest`].
    pub fn set_mix(&mut self, mix: Mix) {
        self.mix = mix;
    }

    /// What is shown of the held and the newest frame.
    pub fn mix(&self) -> Mix {
        self.mix
    }

    /// Sets the filter that scales everything shown from now on. It starts as
    /// [`Filter::Auto`].
    pub fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
    }

    /// The current scaling filter.
    pub fn filter(&self) -> Filter {
        self.filter
    }

    /// Makes the mix of a transition step from now on a masked one, or with `None` a
    /// cross-fade again. Under a mask each pixel has its own weight: with `L` the luminance
    /// of the mask there, `T` the step's threshold and `v` the vagueness, the held frame
    /// weighs `w = 0` for `L <= T` and `min(255, (L - T) * 255 / v)` above it, or for a
    /// vagueness of 0 `255` for `L >= T` and 0 below, and the result is the newest frame for
    /// `w = 0` and `new + (((old - new) * (w + 1)) >> 8)` otherwise. So the newest frame
    /// appears first where the mask is dark, behind an edge that is the softer the larger the
    /// vagueness. A mask smaller than the newest frame is tiled from the frame's corner.
    ///
    /// The bitmap is looked up each time something is shown, so it may be created by a packet
    /// rendered after this call. While there is no such bitmap, or it has no pixels, the mix
    /// is the cross-fade.
    pub fn set_mask(&mut self, mask: Option<(BitmapId, u32)>) {
        self.mask = mask.map(|(bitmap, vagueness)| Mask {
            bitmap,
            vagueness: vagueness.min(MAX_VAGUENESS),
        });
    }

    /// Whether `id` names a live bitmap.
    pub fn has_bitmap(&self, id: BitmapId) -> bool {
        self.table.get(id).is_ok()
    }

    /// The view and the vagueness of the mask in effect: the one set, if its bitmap is live
    /// and has pixels.
    pub(super) fn mask_in_effect(&self) -> Option<(vk::ImageView, u32)> {
        let mask = self.mask?;
        self.table.get(mask.bitmap).ok()?;
        let image = self.images.get(&mask.bitmap.index)?;
        Some((image.view, mask.vagueness))
    }

    /// Whether the compositor was created for a surface and can [`present`].
    ///
    /// [`present`]: VkCompositor::present
    pub fn has_surface(&self) -> bool {
        self.gpu.surface.is_some()
    }

    /// Sets the frame rate that presents are paced at from now on: where the device and the
    /// surface support `VK_EXT_present_timing`, each present of a new frame is given a target
    /// time one period of this rate after the one before. It starts as 40. A rate of 0
    /// counts as 1.
    pub fn set_frame_rate(&mut self, rate: u32) {
        self.pacer.set_frame_rate(rate);
    }

    /// Makes the target time of the next frame start from the clock again instead of
    /// following the one before, as after anything that broke the rhythm of the frames.
    pub fn restart_pacing(&mut self) {
        self.pacer.restart();
    }

    /// The intervals between the times at which the frames presented last became visible:
    /// the newest [`STATS_INTERVALS`](crate::STATS_INTERVALS) of them, or `None` while there
    /// is none. The times are the ones the presentation engine reports where presents are
    /// paced with `VK_EXT_present_timing`, and otherwise the times at which the presents
    /// were handed over, by the compositor's own clock. A present that repeats a frame
    /// already shown is not counted.
    pub fn present_summary(&self) -> Option<IntervalSummary> {
        self.pacer.summary()
    }

    /// Whether the current swapchain's presents carry target times.
    pub fn paces_with_present_timing(&self) -> bool {
        let chain = self.presenter.as_ref().and_then(|p| p.swapchain.as_ref());
        self.pacer.wants_timing() && chain.is_some_and(|chain| chain.timing.is_some())
    }

    /// Why presents stopped carrying target times, once: a call of the extension failed.
    /// Presenting goes on without them.
    pub fn take_pacing_report(&mut self) -> Option<String> {
        self.pacer.take_report()
    }

    /// Tells the compositor the size of the window in pixels. The swapchain is made for that
    /// size at the next [`present`](VkCompositor::present), and replaced when it changes.
    /// Until a size is given, and while either dimension is 0, nothing is presented.
    pub fn resize_surface(&mut self, size: Size) {
        self.surface_size = size;
    }
}
