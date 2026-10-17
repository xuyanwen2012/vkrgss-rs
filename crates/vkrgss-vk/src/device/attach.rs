//! A surface that goes away and comes back while the device stays: the window of an Android
//! activity is destroyed when the activity is paused and made anew when it resumes, and its
//! surface has to go and come with it.
//!
//! A released surface keeps its place in [`Gpu`] with a null handle, so that the device
//! still counts as one opened for a surface; a new one is made from the same instance, which
//! has the extensions of that kind of surface, and has to be one the device's queue can
//! present to.

use ash::vk;

use super::{Gpu, SurfaceSource};
use crate::{DeviceError, VkError};

impl Gpu {
    /// Destroys the surface the device presents to and lets go of its window, keeping the
    /// device and everything made from it. Every swapchain of the surface must have been
    /// destroyed. Does nothing on a device without a surface or whose surface was released.
    pub(crate) fn release_surface(&mut self) {
        let Some(surface) = &mut self.surface else {
            return;
        };
        if surface.handle == vk::SurfaceKHR::null() {
            return;
        }
        // SAFETY: the surface belongs to the live instance, the caller has destroyed every
        // swapchain made for it, and the handle is replaced by null here, so it is not used
        // again; `Surface::drop` destroys a null handle, which does nothing.
        unsafe { surface.loader.destroy_surface(surface.handle, None) };
        surface.handle = vk::SurfaceKHR::null();
        // After the surface, as the field order of `Surface` has it.
        surface._window = None;
    }

    /// Whether the device was opened for a surface that has been released since and not
    /// replaced.
    pub(crate) fn surface_released(&self) -> bool {
        self.surface
            .as_ref()
            .is_some_and(|surface| surface.handle == vk::SurfaceKHR::null())
    }

    /// Makes a surface from `source`, which has to be of the kind the device was opened for,
    /// the one the device presents to, in place of the one that was released. Fails with
    /// [`VkError::NoSurface`] on a device opened without a surface, and with
    /// [`DeviceError::CannotPresent`] when the device's queue cannot present to the new
    /// surface, which is then destroyed again.
    pub(crate) fn attach_surface(&mut self, source: SurfaceSource) -> Result<(), VkError> {
        let surface = self.surface.as_ref().ok_or(VkError::NoSurface)?;
        debug_assert_eq!(surface.handle, vk::SurfaceKHR::null(), "released first");
        let entry = &self.instance._entry;
        let instance = &self.instance.raw;
        let handle = match &source {
            SurfaceSource::Headless => {
                let loader = ash::ext::headless_surface::Instance::new(entry, instance);
                // SAFETY: the instance is live and, the device having been opened for a
                // headless surface, was created with the headless-surface extension; the
                // description has no parameters.
                unsafe {
                    loader
                        .create_headless_surface(&vk::HeadlessSurfaceCreateInfoEXT::default(), None)
                }
                .map_err(VkError::vulkan("vkCreateHeadlessSurfaceEXT"))?
            }
            SurfaceSource::Window(window) => {
                let handles = window.display_handle().and_then(|display| {
                    window
                        .window_handle()
                        .map(|window| (display.as_raw(), window.as_raw()))
                });
                let (display, window) =
                    handles.map_err(|error| DeviceError::Window(error.to_string()))?;
                // SAFETY: the instance is live and, the device having been opened for a
                // window of this window system, was created with the extensions its display
                // needs. The handles were just obtained from the window, which is kept below
                // until after the surface is destroyed.
                unsafe { ash_window::create_surface(entry, instance, display, window, None) }
                    .map_err(VkError::vulkan("vkCreateSurface"))?
            }
        };
        // SAFETY: the physical device and the new surface belong to this live instance, and
        // the family is the one the device's queue was created in.
        let supported = unsafe {
            surface.loader.get_physical_device_surface_support(
                self.physical,
                self.queue_family,
                handle,
            )
        };
        let refusal = match supported {
            Ok(true) => None,
            Ok(false) => Some(VkError::from(DeviceError::CannotPresent {
                device: self.info.name.clone(),
                reason: "its queue cannot present to the new surface of the window".to_owned(),
            })),
            Err(result) => Some(VkError::Vulkan {
                call: "vkGetPhysicalDeviceSurfaceSupportKHR",
                result,
            }),
        };
        if let Some(error) = refusal {
            // SAFETY: the surface was just created from this instance and nothing uses it.
            unsafe { surface.loader.destroy_surface(handle, None) };
            return Err(error);
        }
        let surface = self.surface.as_mut().expect("checked above");
        surface.handle = handle;
        surface._window = match source {
            SurfaceSource::Headless => None,
            SurfaceSource::Window(window) => Some(window),
        };
        Ok(())
    }
}
