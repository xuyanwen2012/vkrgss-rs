//! A window whose surface goes away and comes back: what Android does to the window of an
//! activity that is paused and resumed. The swapchain and the surface go; the device, the
//! bitmaps, the frame and the held frame stay, so the game goes on where it was.

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::VkError;
use crate::compositor::VkCompositor;
use crate::device::SurfaceSource;

impl VkCompositor {
    /// Destroys the swapchain and the surface of the window and lets go of the window, for a
    /// window system that takes the window away. Until [`attach_window`] gives the compositor
    /// a new surface, frames are rendered as before and [`present`] shows nothing and returns
    /// `Ok(false)`, as it does for a window without pixels. Does nothing when the surface was
    /// released already.
    ///
    /// Fails with [`VkError::NoSurface`] on a compositor that was created without a surface.
    /// When idling the device before the swapchain is destroyed fails, the swapchain and the
    /// surface are let go all the same and that error is returned; unless the device is lost,
    /// when nothing of it is used again and both are leaked.
    ///
    /// [`attach_window`]: VkCompositor::attach_window
    /// [`present`]: VkCompositor::present
    pub fn release_surface(&mut self) -> Result<(), VkError> {
        self.unless_lost(|compositor| compositor.release_surface_now())
    }

    fn release_surface_now(&mut self) -> Result<(), VkError> {
        if self.gpu.surface.is_none() {
            return Err(VkError::NoSurface);
        }
        if self.gpu.surface_released() {
            return Ok(());
        }
        let device = &self.gpu.device;
        let mut idled = Ok(());
        if let Some(presenter) = &mut self.presenter
            && let Some(chain) = presenter.swapchain.take()
        {
            // Where the presents carry fences, these say when the presentation engine has let
            // go of them; idling the device covers the rest, as for a replaced swapchain. A
            // lost device ends the release: the swapchain is leaked and the surface stays.
            chain.settle_presents(device)?;
            // SAFETY: the device is live.
            let idle = unsafe { device.device_wait_idle() };
            idled = self
                .present_probe
                .idle_result(idle)
                .map_err(VkError::vulkan("vkDeviceWaitIdle"));
            if let Err(error) = &idled
                && error.is_device_lost()
            {
                return idled;
            }
            let loader = presenter
                .swapchain_loader
                .as_ref()
                .expect("made with the swapchain");
            // Also when idling failed: the surface goes whatever happens, and a swapchain
            // must not outlive it.
            chain.destroy(device, loader);
            self.pacer.swapchain_replaced();
        }
        self.starved = false;
        self.gpu.release_surface();
        idled
    }

    /// Gives the compositor the surface of `window` after [`release_surface`]: the window the
    /// window system made anew, or the same window once it can be drawn to again. The device
    /// has to be able to present to it, or this fails with [`DeviceError::CannotPresent`]
    /// and the compositor stays without a surface. The next [`present`] makes a swapchain
    /// for it and shows the newest frame. The window is kept alive until the surface is
    /// released or the compositor dropped.
    ///
    /// [`release_surface`]: VkCompositor::release_surface
    /// [`present`]: VkCompositor::present
    /// [`DeviceError::CannotPresent`]: crate::DeviceError::CannotPresent
    pub fn attach_window<W>(&mut self, window: W) -> Result<(), VkError>
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        self.attach(SurfaceSource::Window(Box::new(window)))
    }

    /// Gives a compositor made with a headless surface a new one, after
    /// [`release_surface`](VkCompositor::release_surface).
    #[cfg(test)]
    pub(crate) fn attach_headless_surface(&mut self) -> Result<(), VkError> {
        self.attach(SurfaceSource::Headless)
    }

    fn attach(&mut self, source: SurfaceSource) -> Result<(), VkError> {
        self.unless_lost(|compositor| compositor.attach_now(source))
    }

    fn attach_now(&mut self, source: SurfaceSource) -> Result<(), VkError> {
        if !self.gpu.surface_released() {
            self.release_surface()?;
        }
        self.gpu.attach_surface(source)
    }
}
