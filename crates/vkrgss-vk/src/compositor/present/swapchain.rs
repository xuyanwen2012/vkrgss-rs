//! The swapchain: what it owns, the fences of its presents, and how the compositor makes,
//! replaces and repairs it.

use ash::vk;
use vkrgss_proto::Size;

use super::format::{PresentFormat, choose_format, pre_transform, swapchain_extent};
use crate::VkError;
use crate::compositor::VkCompositor;
use crate::compositor::pacer::{SwapchainTiming, surface_pacing};
use crate::present_timing as ffi;
use crate::resources::COLOR_RANGE;

/// How long an acquire waits for a swapchain image, in nanoseconds: four frame periods at
/// 40 frames per second. A surface that hands out none in that time, as that of a hidden
/// window may, must not stop the game.
pub(super) const ACQUIRE_TIMEOUT: u64 = 100_000_000;

/// How long the replacement of a swapchain waits for the fences of its presents, in
/// nanoseconds, before it idles the device instead. A lost surface may never signal them.
const PRESENT_FENCE_TIMEOUT: u64 = ACQUIRE_TIMEOUT;

/// The pipeline that draws into a target of one format.
pub(super) struct FormatPipeline {
    pub(super) format: vk::Format,
    pub(super) pipeline: vk::Pipeline,
}

/// A swapchain with what is needed to draw into and present its images.
pub(super) struct Swapchain {
    pub(super) handle: vk::SwapchainKHR,
    pub(super) extent: vk::Extent2D,
    /// The window size the swapchain was made for.
    requested: Size,
    pub(super) format: PresentFormat,
    pub(super) pipeline: vk::Pipeline,
    pub(super) images: Vec<vk::Image>,
    /// Per image: the view the pass draws through, in the view format of `format`.
    pub(super) views: Vec<vk::ImageView>,
    /// Signalled when an image has been acquired; waited for by the draw.
    pub(super) acquired: vk::Semaphore,
    /// Per image: signalled by the draw, waited for by the presentation engine.
    pub(super) drawn: Vec<vk::Semaphore>,
    /// Whether the swapchain no longer matches the surface and has to be replaced.
    pub(super) stale: bool,
    /// The surface's transform when the swapchain was created with another one, and the
    /// window system turns its images: Android calls every present of such a swapchain
    /// suboptimal, so that alone does not make it stale.
    pub(super) turned: Option<vk::SurfaceTransformFlagsKHR>,
    /// The pacing extensions on this swapchain, when it was created for them.
    pub(super) timing: Option<SwapchainTiming>,
    /// With swapchain maintenance: the fences that the presents carry, one per present,
    /// each signalled once the presentation engine has let go of what its present used. A
    /// fence is used again by a later present once it is signalled, and one is added when
    /// none is. `None` on a swapchain that presents without the extension.
    present_fences: Option<Vec<PresentFence>>,
}

/// A fence for the presents of a swapchain.
struct PresentFence {
    fence: vk::Fence,
    /// Whether a present carried the fence and it has not been seen signalled since.
    outstanding: bool,
}

impl Swapchain {
    /// Waits, for a bounded time, until the presentation engine has let go of what the
    /// presents of this swapchain used. Returns whether it has, which only the fences of
    /// swapchain maintenance tell: `false` without them and when one is not signalled in
    /// time. Fails when the wait finds the device lost.
    pub(super) fn settle_presents(&self, device: &ash::Device) -> Result<bool, VkError> {
        let Some(fences) = &self.present_fences else {
            return Ok(false);
        };
        let outstanding: Vec<vk::Fence> = fences
            .iter()
            .filter(|present| present.outstanding)
            .map(|present| present.fence)
            .collect();
        if !outstanding.is_empty() {
            // SAFETY: the fences are live and belong to this device.
            let waited =
                unsafe { device.wait_for_fences(&outstanding, true, PRESENT_FENCE_TIMEOUT) };
            match waited.map_err(VkError::vulkan("vkWaitForFences")) {
                Ok(()) => {}
                Err(error) if error.is_device_lost() => return Err(error),
                Err(_) => return Ok(false),
            }
        }
        Ok(true)
    }

    /// Destroys the swapchain and everything made for it. Every submission must have been
    /// waited for, and the presentation engine must have let go of the presents: their
    /// fences were waited for, or the device is idle.
    pub(super) fn destroy(self, device: &ash::Device, loader: &ash::khr::swapchain::Device) {
        // SAFETY: the caller guarantees that nothing is in use; every handle was created from
        // this device for this swapchain and is consumed here. Destroying a swapchain also
        // releases an image that was acquired and not presented.
        unsafe {
            for view in self.views {
                device.destroy_image_view(view, None);
            }
            for semaphore in self.drawn {
                device.destroy_semaphore(semaphore, None);
            }
            device.destroy_semaphore(self.acquired, None);
            loader.destroy_swapchain(self.handle, None);
            for present in self.present_fences.into_iter().flatten() {
                device.destroy_fence(present.fence, None);
            }
        }
    }
}

impl VkCompositor {
    /// Whether a swapchain created now uses `VK_EXT_swapchain_maintenance1`.
    pub(crate) fn uses_swapchain_maintenance(&self) -> bool {
        if self.present_probe.ignores_maintenance() {
            return false;
        }
        self.gpu.swapchain_maintenance
    }

    /// The fence the next present carries: null on a swapchain without swapchain
    /// maintenance, else one of the swapchain's fences that no present holds, or a new one
    /// when every one is still held. The fence is unsignalled and counts as outstanding from
    /// here on, whatever the present returns: the presentation engine may have taken it even
    /// then.
    pub(super) fn take_present_fence(&mut self) -> Result<vk::Fence, VkError> {
        let device = &self.gpu.device;
        let presenter = self.presenter.as_mut().expect("made with the sources");
        let chain = presenter.swapchain.as_mut().expect("presenting into it");
        let Some(fences) = &mut chain.present_fences else {
            return Ok(vk::Fence::null());
        };
        for present in fences.iter_mut() {
            if present.outstanding {
                // SAFETY: the fence is live and belongs to this device.
                let signalled = unsafe { device.get_fence_status(present.fence) }
                    .map_err(VkError::vulkan("vkGetFenceStatus"))?;
                if !signalled {
                    continue;
                }
                // SAFETY: the fence is signalled, so the present that carried it is done
                // with it.
                unsafe { device.reset_fences(&[present.fence]) }
                    .map_err(VkError::vulkan("vkResetFences"))?;
            }
            present.outstanding = true;
            return Ok(present.fence);
        }
        // SAFETY: the device is live.
        let fence = unsafe { device.create_fence(&vk::FenceCreateInfo::default(), None) }
            .map_err(VkError::vulkan("vkCreateFence"))?;
        fences.push(PresentFence {
            fence,
            outstanding: true,
        });
        Ok(fence)
    }

    /// After a draw that failed: gives the acquired image `index` back to the swapchain and
    /// replaces the two semaphores of the draw, so that the swapchain can go on. Returns
    /// `false` when that is not possible, without swapchain maintenance or because a call
    /// failed, and the swapchain has to be replaced. A call that finds the device lost
    /// records that, and nothing more of the device is used.
    pub(super) fn release_undrawn(&mut self, index: u32) -> bool {
        let device = &self.gpu.device;
        let Some(presenter) = self.presenter.as_mut() else {
            return false;
        };
        let (Some(chain), Some(loader)) =
            (presenter.swapchain.as_mut(), &presenter.maintenance_loader)
        else {
            return false;
        };
        if chain.present_fences.is_none() {
            return false;
        }
        // Not asked of a driver that cannot do it for this surface.
        if !self.gpu.releases_undrawn_images() {
            return false;
        }
        let semaphore = || {
            // SAFETY: the device is live.
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }
        };
        let acquired = match semaphore() {
            Ok(semaphore) => semaphore,
            Err(result) => {
                if result == vk::Result::ERROR_DEVICE_LOST {
                    self.lost.get_or_insert("vkCreateSemaphore");
                }
                return false;
            }
        };
        let drawn = match semaphore() {
            Ok(semaphore) => semaphore,
            Err(result) => {
                if result == vk::Result::ERROR_DEVICE_LOST {
                    // Nothing of a lost device is used again: the first one is leaked.
                    self.lost.get_or_insert("vkCreateSemaphore");
                    return false;
                }
                // SAFETY: the semaphore was just created and is not used.
                unsafe { device.destroy_semaphore(acquired, None) };
                return false;
            }
        };
        let indices = [index];
        let info = vk::ReleaseSwapchainImagesInfoEXT::default()
            .swapchain(chain.handle)
            .image_indices(&indices);
        // SAFETY: the device has swapchain maintenance and the swapchain is live. The image
        // was acquired from it and not presented, and nothing uses it: the device was idled
        // after the failure. For the same reason the old semaphores, which belong to this
        // device and are not used again, can be destroyed.
        unsafe {
            if let Err(result) = loader.release_swapchain_images(&info) {
                if result == vk::Result::ERROR_DEVICE_LOST {
                    // Nothing of the device is used again; the semaphores are leaked.
                    self.lost.get_or_insert("vkReleaseSwapchainImagesEXT");
                    return false;
                }
                device.destroy_semaphore(acquired, None);
                device.destroy_semaphore(drawn, None);
                return false;
            }
            device.destroy_semaphore(chain.acquired, None);
            device.destroy_semaphore(chain.drawn[index as usize], None);
        }
        chain.acquired = acquired;
        chain.drawn[index as usize] = drawn;
        true
    }

    /// Whether the swapchain, which a present called suboptimal, was created for a surface
    /// the window system turns and still fits it: the surface has the transform and the
    /// extent it had then. A call that fails counts as a change.
    pub(super) fn still_fits_turned_surface(&self) -> bool {
        let Some(chain) = self.presenter.as_ref().and_then(|p| p.swapchain.as_ref()) else {
            return false;
        };
        let (Some(turned), Some(surface)) = (chain.turned, self.gpu.surface.as_ref()) else {
            return false;
        };
        // SAFETY: the physical device and the surface belong to the live instance.
        let capabilities = unsafe {
            surface
                .loader
                .get_physical_device_surface_capabilities(self.gpu.physical, surface.handle)
        };
        capabilities.is_ok_and(|capabilities| {
            capabilities.current_transform == turned
                && swapchain_extent(&capabilities, chain.requested) == chain.extent
        })
    }

    pub(super) fn mark_swapchain_stale(&mut self) {
        if let Some(chain) = self.presenter.as_mut().and_then(|p| p.swapchain.as_mut()) {
            chain.stale = true;
        }
    }

    /// Makes a swapchain that matches the surface and the window size. Returns `false` when
    /// the surface has no pixels now, in which case there is nothing to present into.
    pub(super) fn ensure_swapchain(&mut self) -> Result<bool, VkError> {
        let requested = self.surface_size;
        let maintenance = self.uses_swapchain_maintenance();
        let device = &self.gpu.device;
        let surface = self.gpu.surface.as_ref().expect("checked by the caller");
        let presenter = self.presenter.as_mut().expect("made by the caller");
        if let Some(chain) = &presenter.swapchain
            && !chain.stale
            && chain.requested == requested
        {
            return Ok(true);
        }

        // SAFETY: the physical device and the surface belong to the live instance.
        let capabilities = unsafe {
            surface
                .loader
                .get_physical_device_surface_capabilities(self.gpu.physical, surface.handle)
        }
        .map_err(VkError::vulkan("vkGetPhysicalDeviceSurfaceCapabilitiesKHR"))?;
        let extent = swapchain_extent(&capabilities, requested);
        if extent.width == 0 || extent.height == 0 {
            return Ok(false);
        }
        // SAFETY: as above.
        let formats = unsafe {
            surface
                .loader
                .get_physical_device_surface_formats(self.gpu.physical, surface.handle)
        }
        .map_err(VkError::vulkan("vkGetPhysicalDeviceSurfaceFormatsKHR"))?;
        let formats = self.present_probe.surface_formats(formats);
        let format =
            choose_format(&formats, self.gpu.swapchain_mutable_format).ok_or(VkError::Vulkan {
                call: "vkGetPhysicalDeviceSurfaceFormatsKHR",
                result: vk::Result::ERROR_FORMAT_NOT_SUPPORTED,
            })?;
        let pipeline = presenter.pipeline(&self.gpu, format.view)?;

        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 {
            image_count = image_count.min(capabilities.max_image_count);
        }
        let composite_alpha = [
            vk::CompositeAlphaFlagsKHR::OPAQUE,
            vk::CompositeAlphaFlagsKHR::INHERIT,
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
        ]
        .into_iter()
        .find(|mode| capabilities.supported_composite_alpha.contains(*mode))
        .unwrap_or(vk::CompositeAlphaFlagsKHR::OPAQUE);

        let loader = presenter
            .swapchain_loader
            .get_or_insert_with(|| ash::khr::swapchain::Device::new(self.gpu.instance(), device));
        let settled = match &presenter.swapchain {
            Some(old) => Some(old.settle_presents(device)?),
            None => None,
        };
        if settled == Some(false) {
            // The presentation engine may still hold the semaphore of the last present. The
            // wait for a submission does not cover that. The fences of the presents do, where
            // swapchain maintenance gives them; without them, and when they are not
            // signalled in time, idling the device does.
            // SAFETY: the device is live.
            unsafe { device.device_wait_idle() }.map_err(VkError::vulkan("vkDeviceWaitIdle"))?;
            self.present_probe.swapchain_idled();
        }
        if maintenance && presenter.maintenance_loader.is_none() {
            presenter.maintenance_loader = Some(ash::ext::swapchain_maintenance1::Device::new(
                self.gpu.instance(),
                device,
            ));
        }
        let old = presenter.swapchain.take();
        if old.is_some() {
            self.pacer.swapchain_replaced();
        }
        // Whether the swapchain is created for the pacing extensions, and with what.
        let mut pacing = None;
        if self.gpu.pacing.timing && self.pacer.wants_timing() {
            if presenter.timing_commands.is_none() {
                // SAFETY: the device is live, belongs to this instance and was created with
                // the extensions `pacing` names.
                let commands = unsafe {
                    ffi::Commands::load(self.gpu.instance(), device, self.gpu.pacing.wait)
                };
                match commands {
                    Some(commands) => {
                        let calibrated = ash::khr::calibrated_timestamps::Device::new(
                            self.gpu.instance(),
                            device,
                        );
                        presenter.timing_commands = Some((commands, calibrated));
                    }
                    None => self.pacer.fail((
                        "vkGetDeviceProcAddr",
                        vk::Result::ERROR_EXTENSION_NOT_PRESENT,
                    )),
                }
            }
            if let Some(commands) = &presenter.timing_commands {
                match surface_pacing(&self.gpu) {
                    Ok(supported) => pacing = supported.map(|pacing| (pacing, commands.clone())),
                    Err(failure) => self.pacer.fail(failure),
                }
            }
            // A lost device ends the replacement before the swapchain is made; the old one,
            // taken out above, is leaked.
            self.pacer.check_device()?;
        }
        let mut flags = pacing
            .as_ref()
            .map_or(vk::SwapchainCreateFlagsKHR::empty(), |(pacing, _)| {
                pacing.swapchain_flags()
            });
        // The formats the images are viewed in, when the pass draws through another one than
        // the swapchain's.
        let view_formats = [format.surface.format, format.view];
        let mut format_list = vk::ImageFormatListCreateInfo::default().view_formats(&view_formats);
        if format.is_mutable() {
            flags |= vk::SwapchainCreateFlagsKHR::MUTABLE_FORMAT;
        }
        let mut info = vk::SwapchainCreateInfoKHR::default()
            .flags(flags)
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_format(format.surface.format)
            .image_color_space(format.surface.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform(&capabilities))
            .composite_alpha(composite_alpha)
            // Every surface supports it, and it paces presentation to the display.
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .old_swapchain(
                old.as_ref()
                    .map_or(vk::SwapchainKHR::null(), |old| old.handle),
            );
        if format.is_mutable() {
            info = info.push_next(&mut format_list);
        }
        // SAFETY: the surface belongs to the instance of this device, which was created with
        // the swapchain extension and whose queue can present to it. The values come from the
        // surface's own capabilities and formats; the flags are those of extensions the
        // device was created with and the surface reports support for. The format list is
        // only chained, with the flag for it, on a device with the mutable format extension,
        // and names the swapchain's format and its `UNORM` twin. Every submission was
        // waited for, so the old swapchain, if any, is idle and is retired by this call.
        let created = unsafe { loader.create_swapchain(&info, None) }
            .map_err(VkError::vulkan("vkCreateSwapchainKHR"));
        if let Err(error) = &created
            && error.is_device_lost()
        {
            // Nothing of a lost device is used again: the old swapchain is leaked.
            return Err(error.clone());
        }
        if let Some(old) = old {
            old.destroy(device, loader);
        }
        let handle = created?;
        tracing::info!(
            "swapchain of {} by {} pixels, transform {:?} of the surface's {:?}",
            extent.width,
            extent.height,
            pre_transform(&capabilities),
            capabilities.current_transform
        );
        let timing = pacing.and_then(|(pacing, (commands, calibrated))| {
            // SAFETY: the swapchain was just created from this device with the flags of
            // `pacing`, and the value is kept in the `Swapchain` that owns the handle; the
            // commands were loaded for this device.
            let timing = unsafe {
                SwapchainTiming::new(
                    commands,
                    calibrated,
                    device.handle(),
                    handle,
                    pacing,
                    self.gpu.calibrateable_domains.clone(),
                )
            };
            // The swapchain presents without the extension like any other.
            timing.map_err(|failure| self.pacer.fail(failure)).ok()
        });
        // On a lost device the new swapchain is leaked.
        self.pacer.check_device()?;

        if maintenance {
            self.gpu.note_present_fences();
        }
        let mut chain = Swapchain {
            handle,
            extent,
            requested,
            format,
            pipeline,
            images: Vec::new(),
            views: Vec::new(),
            acquired: vk::Semaphore::null(),
            drawn: Vec::new(),
            stale: false,
            turned: (pre_transform(&capabilities) != capabilities.current_transform)
                .then_some(capabilities.current_transform),
            timing,
            present_fences: maintenance.then(Vec::new),
        };
        let filled = (|| -> Result<(), VkError> {
            // SAFETY: the swapchain was just created from this device.
            chain.images = unsafe { loader.get_swapchain_images(chain.handle) }
                .map_err(VkError::vulkan("vkGetSwapchainImagesKHR"))?;
            let semaphore = || {
                // SAFETY: the device is live.
                unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }
                    .map_err(VkError::vulkan("vkCreateSemaphore"))
            };
            chain.acquired = semaphore()?;
            for (index, &image) in chain.images.iter().enumerate() {
                self.gpu
                    .name_object(image, &format!("swapchain image {index}"));
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.view)
                    .subresource_range(COLOR_RANGE);
                // SAFETY: the image belongs to the live swapchain and the view matches its
                // single colour subresource and has its format, or one the swapchain was
                // created for views in.
                let view = unsafe { device.create_image_view(&view_info, None) }
                    .map_err(VkError::vulkan("vkCreateImageView"))?;
                chain.views.push(view);
                chain.drawn.push(semaphore()?);
            }
            Ok(())
        })();
        match filled {
            Ok(()) => {
                presenter.swapchain = Some(chain);
                Ok(true)
            }
            // Nothing of a lost device is used again: the swapchain is leaked.
            Err(error) if error.is_device_lost() => Err(error),
            Err(error) => {
                chain.destroy(device, loader);
                Err(error)
            }
        }
    }
}
