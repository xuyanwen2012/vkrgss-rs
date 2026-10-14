//! The Vulkan instance, and what it tells about the physical devices.

use std::ffi::{CStr, c_char};

use ash::vk;

use super::{
    DeviceCaps, DeviceInfo, DeviceKind, FamilyCaps, PacingCaps, Surface, lacks, presenting_family,
};
use crate::present_timing as timing;
use crate::validation::{self, ValidationLog};
use crate::{DeviceError, VkError};

/// A Vulkan instance, with the validation layer and its messenger when a log was given and the
/// layer is installed.
pub(super) struct Instance {
    // Keeps the Vulkan library loaded for as long as the instance exists.
    pub(super) _entry: ash::Entry,
    pub(super) raw: ash::Instance,
    messenger: Option<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
    /// Whether the instance was created with `VK_KHR_get_surface_capabilities2`, which an
    /// instance for a surface gets when the installation offers it.
    pub(super) surface_capabilities2: bool,
    /// Whether the instance was created with `VK_EXT_surface_maintenance1`, which
    /// `VK_EXT_swapchain_maintenance1` on a device needs.
    surface_maintenance1: bool,
    /// Whether the instance was created with `VK_EXT_debug_utils`: whenever the installation
    /// lists it, and with the validation layer, which brings it.
    pub(super) debug_utils: bool,
    // The messenger's callback points into the log; holding it here keeps that target alive
    // until `drop` has destroyed the instance.
    pub(super) _log: Option<ValidationLog>,
}

/// A physical device with what `Gpu` needs to open it.
pub(super) struct Candidate {
    pub(super) physical: vk::PhysicalDevice,
    pub(super) info: DeviceInfo,
    pub(super) compute_family: Option<u32>,
    /// What each queue family can do; empty flags for a family without queues.
    pub(super) families: Vec<vk::QueueFlags>,
    pub(super) max_image_dimension: u32,
    pub(super) max_storage_buffer_range: u32,
    /// `driverVersion` of the device, in the driver's own encoding.
    pub(super) driver_version: u32,
    /// `timestampValidBits` of each queue family.
    pub(super) timestamp_bits: Vec<u32>,
    /// `timestampPeriod` of the device: the nanoseconds of one tick of a timestamp.
    pub(super) timestamp_period: f32,
}

impl Instance {
    /// Creates the instance. `surface_extensions` are the instance extensions a surface needs;
    /// a headless instance passes none.
    pub(super) fn new(
        log: Option<&ValidationLog>,
        surface_extensions: &[&'static CStr],
    ) -> Result<Instance, VkError> {
        // SAFETY: loading the system's Vulkan library runs its initialisers; that is the
        // documented way to obtain the entry points and has no other precondition.
        let entry = unsafe { ash::Entry::load() }
            .map_err(|error| DeviceError::Loader(error.to_string()))?;

        let validate = match log {
            Some(_) => {
                // SAFETY: the entry points are loaded and the call takes no handle.
                let layers = unsafe { entry.enumerate_instance_layer_properties() }
                    .map_err(VkError::vulkan("vkEnumerateInstanceLayerProperties"))?;
                layers
                    .iter()
                    .any(|layer| layer.layer_name_as_c_str().ok() == Some(validation::LAYER_NAME))
            }
            None => false,
        };
        let log = log.filter(|_| validate).cloned();

        // SAFETY: the entry points are loaded and the call takes no handle.
        let offered = unsafe { entry.enumerate_instance_extension_properties(None) }
            .map_err(VkError::vulkan("vkEnumerateInstanceExtensionProperties"))?;
        // Optional: for the names and labels. The validation layer brings it when the
        // installation does not, and the messenger needs it.
        let debug_utils = validate
            || offered.iter().any(|extension| {
                extension.extension_name_as_c_str().ok() == Some(ash::ext::debug_utils::NAME)
            });
        let mut surface_capabilities2 = false;
        let mut surface_maintenance1 = false;
        if !surface_extensions.is_empty() {
            // Optional: only the present-timing path asks the surface through it.
            surface_capabilities2 = offered.iter().any(|extension| {
                extension.extension_name_as_c_str().ok()
                    == Some(ash::khr::get_surface_capabilities2::NAME)
            });
            // Optional: for swapchain maintenance on the device. It depends on the one above.
            surface_maintenance1 = surface_capabilities2
                && offered.iter().any(|extension| {
                    extension.extension_name_as_c_str().ok()
                        == Some(ash::ext::surface_maintenance1::NAME)
                });
            for wanted in surface_extensions {
                let found = offered
                    .iter()
                    .any(|extension| extension.extension_name_as_c_str().ok() == Some(*wanted));
                if !found {
                    return Err(DeviceError::MissingInstanceExtension {
                        name: wanted.to_string_lossy().into_owned(),
                    }
                    .into());
                }
            }
        }

        let application = vk::ApplicationInfo::default()
            .application_name(c"vkrgss")
            .api_version(vk::API_VERSION_1_3);
        let layers = [validation::LAYER_NAME.as_ptr()];
        let mut extensions: Vec<*const c_char> = surface_extensions
            .iter()
            .map(|extension| extension.as_ptr())
            .collect();
        if surface_capabilities2 {
            extensions.push(ash::khr::get_surface_capabilities2::NAME.as_ptr());
        }
        if surface_maintenance1 {
            extensions.push(ash::ext::surface_maintenance1::NAME.as_ptr());
        }
        let mut messenger_info = log.as_ref().map(ValidationLog::messenger_info);
        if debug_utils {
            extensions.push(ash::ext::debug_utils::NAME.as_ptr());
        }
        let mut info = vk::InstanceCreateInfo::default()
            .application_info(&application)
            .enabled_extension_names(&extensions);
        if let Some(messenger_info) = messenger_info.as_mut() {
            // Chaining the messenger also covers instance creation and destruction.
            info = info.enabled_layer_names(&layers).push_next(messenger_info);
        }
        // SAFETY: `info` and everything it points to are valid for the call, and the log the
        // chained messenger reports into is kept alive by the returned `Instance`.
        let raw = unsafe { entry.create_instance(&info, None) }
            .map_err(VkError::vulkan("vkCreateInstance"))?;

        let mut instance = Instance {
            _entry: entry,
            raw,
            messenger: None,
            surface_capabilities2,
            surface_maintenance1,
            debug_utils,
            _log: log.clone(),
        };
        if let Some(log) = &log {
            let loader = ash::ext::debug_utils::Instance::new(&instance._entry, &instance.raw);
            // SAFETY: the instance was created with the debug-utils extension, and the log
            // behind the callback outlives the messenger (see `_log`).
            let messenger =
                unsafe { loader.create_debug_utils_messenger(&log.messenger_info(), None) }
                    .map_err(VkError::vulkan("vkCreateDebugUtilsMessengerEXT"))?;
            instance.messenger = Some((loader, messenger));
            log.set_active();
        }
        Ok(instance)
    }

    /// Every physical device, in enumeration order.
    pub(super) fn physical_devices(&self) -> Result<Vec<Candidate>, VkError> {
        // SAFETY: the instance is live.
        let physicals = unsafe { self.raw.enumerate_physical_devices() }
            .map_err(VkError::vulkan("vkEnumeratePhysicalDevices"))?;
        Ok(physicals
            .into_iter()
            .enumerate()
            .map(|(index, physical)| self.describe(index, physical))
            .collect())
    }

    fn describe(&self, index: usize, physical: vk::PhysicalDevice) -> Candidate {
        // SAFETY: `physical` was enumerated from this live instance.
        let properties = unsafe { self.raw.get_physical_device_properties(physical) };
        // SAFETY: as above.
        let families = unsafe {
            self.raw
                .get_physical_device_queue_family_properties(physical)
        };
        let compute_family = families
            .iter()
            .position(|family| {
                family.queue_count > 0 && family.queue_flags.contains(vk::QueueFlags::COMPUTE)
            })
            .map(|family| family as u32);

        let api_version = (
            vk::api_version_major(properties.api_version),
            vk::api_version_minor(properties.api_version),
        );
        let mut caps = DeviceCaps {
            api_version,
            compute_queue: compute_family.is_some(),
            runtime_descriptor_array: false,
            sampled_image_array_non_uniform_indexing: false,
            descriptor_binding_partially_bound: false,
            descriptor_binding_sampled_image_update_after_bind: false,
            synchronization2: false,
            dynamic_rendering: false,
            max_update_after_bind_sampled_images: 0,
            max_work_group_invocations: properties.limits.max_compute_work_group_invocations,
            max_work_group_size: [
                properties.limits.max_compute_work_group_size[0],
                properties.limits.max_compute_work_group_size[1],
            ],
        };
        let mut driver = String::new();
        // The Vulkan 1.2 structures may only be chained for a device of at least that version.
        if api_version >= (1, 2) {
            let mut features12 = vk::PhysicalDeviceVulkan12Features::default();
            let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut features12);
            // SAFETY: `physical` is valid and the chain holds only structures that a device of
            // this version knows.
            unsafe {
                self.raw
                    .get_physical_device_features2(physical, &mut features)
            };

            let mut properties12 = vk::PhysicalDeviceVulkan12Properties::default();
            let mut properties2 =
                vk::PhysicalDeviceProperties2::default().push_next(&mut properties12);
            // SAFETY: as above.
            unsafe {
                self.raw
                    .get_physical_device_properties2(physical, &mut properties2)
            };

            caps.runtime_descriptor_array = features12.runtime_descriptor_array == vk::TRUE;
            caps.sampled_image_array_non_uniform_indexing =
                features12.shader_sampled_image_array_non_uniform_indexing == vk::TRUE;
            caps.descriptor_binding_partially_bound =
                features12.descriptor_binding_partially_bound == vk::TRUE;
            caps.descriptor_binding_sampled_image_update_after_bind =
                features12.descriptor_binding_sampled_image_update_after_bind == vk::TRUE;
            caps.max_update_after_bind_sampled_images = properties12
                .max_descriptor_set_update_after_bind_sampled_images
                .min(properties12.max_per_stage_descriptor_update_after_bind_sampled_images)
                .min(properties12.max_per_stage_update_after_bind_resources);
            driver = [
                c_chars(&properties12.driver_name),
                c_chars(&properties12.driver_info),
            ]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        }
        // Likewise the Vulkan 1.3 structure. Both features are mandatory in that version.
        if api_version >= (1, 3) {
            let mut features13 = vk::PhysicalDeviceVulkan13Features::default();
            let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut features13);
            // SAFETY: as above.
            unsafe {
                self.raw
                    .get_physical_device_features2(physical, &mut features)
            };
            caps.synchronization2 = features13.synchronization2 == vk::TRUE;
            caps.dynamic_rendering = features13.dynamic_rendering == vk::TRUE;
        }

        Candidate {
            physical,
            info: DeviceInfo {
                index,
                name: c_chars(&properties.device_name),
                kind: match properties.device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => DeviceKind::Discrete,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => DeviceKind::Integrated,
                    vk::PhysicalDeviceType::VIRTUAL_GPU => DeviceKind::Virtual,
                    vk::PhysicalDeviceType::CPU => DeviceKind::Cpu,
                    _ => DeviceKind::Other,
                },
                driver,
                lacks: lacks(&caps),
            },
            compute_family,
            families: families
                .iter()
                .map(|family| match family.queue_count {
                    0 => vk::QueueFlags::empty(),
                    _ => family.queue_flags,
                })
                .collect(),
            max_image_dimension: properties.limits.max_image_dimension2_d,
            driver_version: properties.driver_version,
            max_storage_buffer_range: properties.limits.max_storage_buffer_range,
            timestamp_bits: families
                .iter()
                .map(|family| family.timestamp_valid_bits)
                .collect(),
            timestamp_period: properties.limits.timestamp_period,
        }
    }
}

impl Instance {
    /// Asks the device whether it can present to `surface`: the queue family to use, or why
    /// it cannot.
    pub(super) fn presenting_family(
        &self,
        candidate: &Candidate,
        surface: &Surface,
    ) -> Result<Result<u32, DeviceError>, VkError> {
        // SAFETY: the physical device was enumerated from this live instance.
        let extensions = unsafe {
            self.raw
                .enumerate_device_extension_properties(candidate.physical)
        }
        .map_err(VkError::vulkan("vkEnumerateDeviceExtensionProperties"))?;
        let swapchain_extension = extensions.iter().any(|extension| {
            extension.extension_name_as_c_str().ok() == Some(ash::khr::swapchain::NAME)
        });

        let mut families = Vec::with_capacity(candidate.families.len());
        for (index, flags) in candidate.families.iter().enumerate() {
            let usable = flags.contains(vk::QueueFlags::COMPUTE | vk::QueueFlags::GRAPHICS);
            // Only a family that could be used is asked about the surface.
            let present = usable
                // SAFETY: the physical device and the surface belong to this live instance
                // and the index is one of the device's queue families.
                && unsafe {
                    surface.loader.get_physical_device_surface_support(
                        candidate.physical,
                        index as u32,
                        surface.handle,
                    )
                }
                .map_err(VkError::vulkan("vkGetPhysicalDeviceSurfaceSupportKHR"))?;
            families.push(FamilyCaps {
                compute: flags.contains(vk::QueueFlags::COMPUTE),
                graphics: flags.contains(vk::QueueFlags::GRAPHICS),
                present,
            });
        }
        Ok(presenting_family(
            &candidate.info,
            swapchain_extension,
            &families,
        ))
    }
}

impl Instance {
    /// What the device offers for pacing presents: the extensions it lists and the features
    /// it reports for those of them it lists.
    pub(super) fn pacing_caps(&self, physical: vk::PhysicalDevice) -> Result<PacingCaps, VkError> {
        let mut caps = PacingCaps {
            surface_capabilities2: self.surface_capabilities2,
            ..PacingCaps::default()
        };
        // SAFETY: the physical device was enumerated from this live instance.
        let extensions = unsafe { self.raw.enumerate_device_extension_properties(physical) }
            .map_err(VkError::vulkan("vkEnumerateDeviceExtensionProperties"))?;
        let listed = |name: &CStr| {
            extensions
                .iter()
                .any(|extension| extension.extension_name_as_c_str().ok() == Some(name))
        };
        caps.present_timing_extension = listed(timing::PRESENT_TIMING_NAME);
        caps.present_id2_extension = listed(timing::PRESENT_ID_2_NAME);
        caps.present_wait2_extension = listed(timing::PRESENT_WAIT_2_NAME);
        caps.calibrated_timestamps_extension = listed(ash::khr::calibrated_timestamps::NAME);

        let mut timing_features = timing::PhysicalDevicePresentTimingFeatures::default();
        let mut id_features = timing::PhysicalDevicePresentId2Features::default();
        let mut wait_features = timing::PhysicalDevicePresentWait2Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::default();
        // A structure is only chained for a device that lists its extension.
        if caps.present_timing_extension {
            timing_features.p_next = features.p_next;
            features.p_next = (&raw mut timing_features).cast();
        }
        if caps.present_id2_extension {
            id_features.p_next = features.p_next;
            features.p_next = (&raw mut id_features).cast();
        }
        if caps.present_wait2_extension {
            wait_features.p_next = features.p_next;
            features.p_next = (&raw mut wait_features).cast();
        }
        // SAFETY: `physical` is valid, a suitable device is of Vulkan 1.3, and the chain
        // holds only live structures of extensions the device lists, each with the layout
        // and the structure type of the header.
        unsafe {
            self.raw
                .get_physical_device_features2(physical, &mut features)
        };
        caps.present_timing = timing_features.present_timing == vk::TRUE;
        caps.present_at_absolute_time = timing_features.present_at_absolute_time == vk::TRUE;
        caps.present_id2 = id_features.present_id2 == vk::TRUE;
        caps.present_wait2 = wait_features.present_wait2 == vk::TRUE;
        Ok(caps)
    }
}

impl Instance {
    /// Whether the device can be opened with `VK_EXT_swapchain_maintenance1`: the instance
    /// has the surface extension it needs, and the device lists it and reports its feature.
    pub(super) fn swapchain_maintenance(
        &self,
        physical: vk::PhysicalDevice,
    ) -> Result<bool, VkError> {
        if !self.surface_maintenance1 {
            return Ok(false);
        }
        // SAFETY: the physical device was enumerated from this live instance.
        let extensions = unsafe { self.raw.enumerate_device_extension_properties(physical) }
            .map_err(VkError::vulkan("vkEnumerateDeviceExtensionProperties"))?;
        let listed = extensions.iter().any(|extension| {
            extension.extension_name_as_c_str().ok() == Some(ash::ext::swapchain_maintenance1::NAME)
        });
        if !listed {
            return Ok(false);
        }
        let mut maintenance = vk::PhysicalDeviceSwapchainMaintenance1FeaturesEXT::default();
        let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut maintenance);
        // SAFETY: `physical` is valid, a suitable device is of Vulkan 1.3, and the chained
        // structure is that of an extension the device lists.
        unsafe {
            self.raw
                .get_physical_device_features2(physical, &mut features)
        };
        Ok(maintenance.swapchain_maintenance1 == vk::TRUE)
    }

    /// Whether the device lists `VK_KHR_swapchain_mutable_format`. What the extension depends
    /// on is core in the Vulkan version a suitable device has.
    pub(super) fn swapchain_mutable_format(
        &self,
        physical: vk::PhysicalDevice,
    ) -> Result<bool, VkError> {
        // SAFETY: the physical device was enumerated from this live instance.
        let extensions = unsafe { self.raw.enumerate_device_extension_properties(physical) }
            .map_err(VkError::vulkan("vkEnumerateDeviceExtensionProperties"))?;
        Ok(extensions.iter().any(|extension| {
            extension.extension_name_as_c_str().ok()
                == Some(ash::khr::swapchain_mutable_format::NAME)
        }))
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        // SAFETY: every object created from the instance has been destroyed by its owner, the
        // messenger belongs to this instance, and neither handle is used again.
        unsafe {
            if let Some((loader, messenger)) = self.messenger.take() {
                loader.destroy_debug_utils_messenger(messenger, None);
            }
            self.raw.destroy_instance(None);
        }
    }
}

/// A fixed-size character array from a Vulkan structure, up to its terminator.
fn c_chars(chars: &[std::ffi::c_char]) -> String {
    // SAFETY: `c_char` and `u8` have the same size and alignment, and the slice stays borrowed.
    let bytes = unsafe { std::slice::from_raw_parts(chars.as_ptr().cast::<u8>(), chars.len()) };
    match CStr::from_bytes_until_nul(bytes) {
        Ok(text) => text.to_string_lossy().into_owned(),
        Err(_) => String::from_utf8_lossy(bytes).into_owned(),
    }
}
