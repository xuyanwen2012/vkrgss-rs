//! Instance and device creation, and the choice of the physical device.
//!
//! [`Gpu::new`] opens a headless device: no surface, swapchain or window-system extension is
//! involved, and the device only needs a queue that runs compute work. [`Gpu::with_surface`]
//! opens a device for a surface: the instance gets the surface extensions, the device gets
//! `VK_KHR_swapchain`, and its queue also draws and presents to that surface. Such a device
//! also gets `VK_EXT_present_timing` and `VK_KHR_present_wait2` with what they depend on,
//! when it has them; see [`pacing_support`].

use std::any::Any;
use std::ffi::{CStr, c_char};

use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::present_timing as timing;
use crate::validation::{self, ValidationLog};
use crate::{DeviceError, VkError};

/// Number of bitmap slots, and so the length of the sampled-image array the shader indexes.
pub(crate) const BITMAP_SLOTS: u32 = 65536;

/// Edge of a tile in pixels, and of the compute workgroup that evaluates it.
pub(crate) const TILE_SIZE: u32 = 16;

/// Which physical device the backend runs on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DeviceChoice {
    /// The first suitable discrete GPU, else integrated GPU, else any other suitable device.
    #[default]
    Auto,
    /// The device at this position in the enumeration order.
    Index(usize),
    /// The first suitable device whose name contains this text, ignoring ASCII case.
    Name(String),
}

impl std::fmt::Display for DeviceChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceChoice::Auto => f.write_str("automatic selection"),
            DeviceChoice::Index(index) => write!(f, "index {index}"),
            DeviceChoice::Name(name) => write!(f, "name `{name}`"),
        }
    }
}

/// The kind of a physical device, as far as the preference order cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    Discrete,
    Integrated,
    Virtual,
    /// A CPU implementation such as llvmpipe.
    Cpu,
    Other,
}

impl DeviceKind {
    /// Position in the automatic preference order; lower is preferred.
    const fn rank(self) -> u8 {
        match self {
            DeviceKind::Discrete => 0,
            DeviceKind::Integrated => 1,
            DeviceKind::Virtual | DeviceKind::Cpu | DeviceKind::Other => 2,
        }
    }
}

impl std::fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            DeviceKind::Discrete => "discrete GPU",
            DeviceKind::Integrated => "integrated GPU",
            DeviceKind::Virtual => "virtual GPU",
            DeviceKind::Cpu => "CPU",
            DeviceKind::Other => "other",
        })
    }
}

/// One physical device and whether the backend can run on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Position in the enumeration order, as `DeviceChoice::Index` counts.
    pub index: usize,
    pub name: String,
    pub kind: DeviceKind,
    /// Driver name and version as the driver reports them; empty when it does not.
    pub driver: String,
    /// Each requirement the device does not meet. Empty for a suitable device.
    pub lacks: Vec<String>,
}

impl DeviceInfo {
    /// Whether the backend can run on the device.
    pub fn is_suitable(&self) -> bool {
        self.lacks.is_empty()
    }
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} ({}", self.index, self.name, self.kind)?;
        if !self.driver.is_empty() {
            write!(f, ", {}", self.driver)?;
        }
        if self.is_suitable() {
            f.write_str("): suitable")
        } else {
            write!(f, "): lacks {}", self.lacks.join("; "))
        }
    }
}

/// What the requirement check looks at, separated from Vulkan so it can be tested on synthetic
/// descriptions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DeviceCaps {
    /// Vulkan version of the device as (major, minor).
    pub api_version: (u32, u32),
    /// Whether a queue family supports compute work.
    pub compute_queue: bool,
    pub runtime_descriptor_array: bool,
    pub sampled_image_array_non_uniform_indexing: bool,
    pub descriptor_binding_partially_bound: bool,
    pub descriptor_binding_sampled_image_update_after_bind: bool,
    pub synchronization2: bool,
    pub dynamic_rendering: bool,
    /// The smaller of the per-set and per-stage update-after-bind sampled-image limits.
    pub max_update_after_bind_sampled_images: u32,
    /// `maxComputeWorkGroupInvocations`.
    pub max_work_group_invocations: u32,
    /// The x and y entries of `maxComputeWorkGroupSize`.
    pub max_work_group_size: [u32; 2],
}

/// Each requirement of the backend that `caps` does not meet.
pub(crate) fn lacks(caps: &DeviceCaps) -> Vec<String> {
    let mut lacks = Vec::new();
    if caps.api_version < (1, 3) {
        let (major, minor) = caps.api_version;
        lacks.push(format!("Vulkan 1.3 (supports {major}.{minor})"));
        // The descriptor indexing features and limits cannot be queried reliably below 1.2,
        // so the version is the whole report.
        return lacks;
    }
    if !caps.compute_queue {
        lacks.push("a compute queue".to_owned());
    }
    let features = [
        ("runtimeDescriptorArray", caps.runtime_descriptor_array),
        (
            "shaderSampledImageArrayNonUniformIndexing",
            caps.sampled_image_array_non_uniform_indexing,
        ),
        (
            "descriptorBindingPartiallyBound",
            caps.descriptor_binding_partially_bound,
        ),
        (
            "descriptorBindingSampledImageUpdateAfterBind",
            caps.descriptor_binding_sampled_image_update_after_bind,
        ),
        ("synchronization2", caps.synchronization2),
        ("dynamicRendering", caps.dynamic_rendering),
    ];
    for (name, supported) in features {
        if !supported {
            lacks.push(format!("feature {name}"));
        }
    }
    if caps.max_update_after_bind_sampled_images < BITMAP_SLOTS {
        lacks.push(format!(
            "{BITMAP_SLOTS} update-after-bind sampled images (supports {})",
            caps.max_update_after_bind_sampled_images
        ));
    }
    let invocations = TILE_SIZE * TILE_SIZE;
    if caps.max_work_group_invocations < invocations
        || caps
            .max_work_group_size
            .iter()
            .any(|&size| size < TILE_SIZE)
    {
        lacks.push(format!(
            "{TILE_SIZE}x{TILE_SIZE} compute workgroups (supports {}x{}, {} invocations)",
            caps.max_work_group_size[0],
            caps.max_work_group_size[1],
            caps.max_work_group_invocations
        ));
    }
    lacks
}

/// What one queue family of a device can do, as far as presenting cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FamilyCaps {
    pub compute: bool,
    pub graphics: bool,
    /// Whether the family can present to the surface in question.
    pub present: bool,
}

/// The queue family a device presents with: the first that computes, draws and can present to
/// the surface. Fails, naming the device, when it has no swapchain extension or no such family.
pub(crate) fn presenting_family(
    device: &DeviceInfo,
    swapchain_extension: bool,
    families: &[FamilyCaps],
) -> Result<u32, DeviceError> {
    let cannot = |reason: &str| DeviceError::CannotPresent {
        device: device.name.clone(),
        reason: reason.to_owned(),
    };
    if !swapchain_extension {
        return Err(cannot("it lacks the VK_KHR_swapchain extension"));
    }
    families
        .iter()
        .position(|family| family.compute && family.graphics && family.present)
        .map(|family| family as u32)
        .ok_or_else(|| {
            cannot("none of its queue families can compute, draw and present to that surface")
        })
}

/// What decides whether presents can be paced with `VK_EXT_present_timing`: the extensions
/// the device lists and the features it reports for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PacingCaps {
    /// Whether the instance has `VK_KHR_get_surface_capabilities2`.
    pub surface_capabilities2: bool,
    pub present_timing_extension: bool,
    pub present_id2_extension: bool,
    pub present_wait2_extension: bool,
    pub calibrated_timestamps_extension: bool,
    /// The `presentTiming` feature.
    pub present_timing: bool,
    /// The `presentAtAbsoluteTime` feature.
    pub present_at_absolute_time: bool,
    /// The `presentId2` feature.
    pub present_id2: bool,
    /// The `presentWait2` feature.
    pub present_wait2: bool,
}

/// Which of the optional pacing extensions a device is opened with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PacingSupport {
    /// `VK_EXT_present_timing` with presentation at an absolute time, together with the
    /// extensions it depends on.
    pub timing: bool,
    /// Also `VK_KHR_present_wait2`, with present ids.
    pub wait: bool,
}

/// What a device with `caps` is opened with. Present timing needs its extension, the ones
/// it depends on and its two features; the present wait is only used on top of it.
pub(crate) fn pacing_support(caps: &PacingCaps) -> PacingSupport {
    let timing = caps.surface_capabilities2
        && caps.present_timing_extension
        && caps.present_id2_extension
        && caps.calibrated_timestamps_extension
        && caps.present_timing
        && caps.present_at_absolute_time;
    PacingSupport {
        timing,
        wait: timing && caps.present_wait2_extension && caps.present_id2 && caps.present_wait2,
    }
}

/// The positions in `devices` to try for a surface, most preferred first: every suitable
/// device in the automatic preference order, or the one device an explicit choice selects.
pub(crate) fn surface_candidates(
    choice: &DeviceChoice,
    devices: &[DeviceInfo],
) -> Result<Vec<usize>, DeviceError> {
    match choice {
        DeviceChoice::Auto => {
            let mut positions: Vec<usize> = (0..devices.len())
                .filter(|position| devices[*position].is_suitable())
                .collect();
            if positions.is_empty() {
                return Err(DeviceError::NoSuitableDevice {
                    devices: devices.to_vec(),
                });
            }
            // Stable, so equally ranked devices keep their enumeration order.
            positions.sort_by_key(|position| devices[*position].kind.rank());
            Ok(positions)
        }
        _ => Ok(vec![select(choice, devices)?]),
    }
}

/// The position in `devices` of the device `choice` selects.
///
/// An explicit choice never falls back to another device: when it names no device, or only
/// unsuitable ones, the error lists what is available.
pub(crate) fn select(choice: &DeviceChoice, devices: &[DeviceInfo]) -> Result<usize, DeviceError> {
    match choice {
        DeviceChoice::Auto => devices
            .iter()
            .enumerate()
            .filter(|(_, device)| device.is_suitable())
            // `min_by_key` keeps the first of equally ranked devices.
            .min_by_key(|(_, device)| device.kind.rank())
            .map(|(position, _)| position)
            .ok_or_else(|| DeviceError::NoSuitableDevice {
                devices: devices.to_vec(),
            }),
        DeviceChoice::Index(index) => {
            let found = devices.iter().position(|device| device.index == *index);
            explicit(choice, devices, found.into_iter())
        }
        DeviceChoice::Name(name) => {
            let wanted = name.to_ascii_lowercase();
            let matches = devices
                .iter()
                .enumerate()
                .filter(|(_, device)| device.name.to_ascii_lowercase().contains(&wanted))
                .map(|(position, _)| position);
            explicit(choice, devices, matches)
        }
    }
}

/// Resolves an explicit choice from the positions it matches.
fn explicit(
    choice: &DeviceChoice,
    devices: &[DeviceInfo],
    matches: impl Iterator<Item = usize>,
) -> Result<usize, DeviceError> {
    let mut any = false;
    for position in matches {
        any = true;
        if devices[position].is_suitable() {
            return Ok(position);
        }
    }
    let (choice, devices) = (choice.to_string(), devices.to_vec());
    Err(if any {
        DeviceError::Unsuitable { choice, devices }
    } else {
        DeviceError::NotFound { choice, devices }
    })
}

/// The Vulkan devices of this machine, in enumeration order, each with what it lacks.
pub fn available_devices() -> Result<Vec<DeviceInfo>, VkError> {
    let instance = Instance::new(None, &[])?;
    Ok(instance
        .physical_devices()?
        .into_iter()
        .map(|candidate| candidate.info)
        .collect())
}

/// A Vulkan instance, with the validation layer and its messenger when a log was given and the
/// layer is installed.
struct Instance {
    // Keeps the Vulkan library loaded for as long as the instance exists.
    _entry: ash::Entry,
    raw: ash::Instance,
    messenger: Option<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
    /// Whether the instance was created with `VK_KHR_get_surface_capabilities2`, which an
    /// instance for a surface gets when the installation offers it.
    surface_capabilities2: bool,
    // The messenger's callback points into the log; holding it here keeps that target alive
    // until `drop` has destroyed the instance.
    _log: Option<ValidationLog>,
}

/// A physical device with what `Gpu` needs to open it.
struct Candidate {
    physical: vk::PhysicalDevice,
    info: DeviceInfo,
    compute_family: Option<u32>,
    /// What each queue family can do; empty flags for a family without queues.
    families: Vec<vk::QueueFlags>,
    max_image_dimension: u32,
    max_storage_buffer_range: u32,
}

impl Instance {
    /// Creates the instance. `surface_extensions` are the instance extensions a surface needs;
    /// a headless instance passes none.
    fn new(
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

        let mut surface_capabilities2 = false;
        if !surface_extensions.is_empty() {
            // SAFETY: the entry points are loaded and the call takes no handle.
            let offered = unsafe { entry.enumerate_instance_extension_properties(None) }
                .map_err(VkError::vulkan("vkEnumerateInstanceExtensionProperties"))?;
            // Optional: only the present-timing path asks the surface through it.
            surface_capabilities2 = offered.iter().any(|extension| {
                extension.extension_name_as_c_str().ok()
                    == Some(ash::khr::get_surface_capabilities2::NAME)
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
            .application_name(c"vkxp")
            .api_version(vk::API_VERSION_1_3);
        let layers = [validation::LAYER_NAME.as_ptr()];
        let mut extensions: Vec<*const c_char> = surface_extensions
            .iter()
            .map(|extension| extension.as_ptr())
            .collect();
        if surface_capabilities2 {
            extensions.push(ash::khr::get_surface_capabilities2::NAME.as_ptr());
        }
        let mut messenger_info = log.as_ref().map(ValidationLog::messenger_info);
        if messenger_info.is_some() {
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
    fn physical_devices(&self) -> Result<Vec<Candidate>, VkError> {
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
            max_storage_buffer_range: properties.limits.max_storage_buffer_range,
        }
    }
}

impl Instance {
    /// Asks the device whether it can present to `surface`: the queue family to use, or why
    /// it cannot.
    fn presenting_family(
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
    fn pacing_caps(&self, physical: vk::PhysicalDevice) -> Result<PacingCaps, VkError> {
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

/// Where a surface comes from.
pub(crate) enum SurfaceSource {
    /// A surface without any window system behind it (`VK_EXT_headless_surface`).
    Headless,
    /// The surface of a window, which is kept alive for as long as the surface exists.
    Window(Box<dyn SurfaceWindow>),
}

/// A window a surface can be created for and that can be kept alongside it.
pub(crate) trait SurfaceWindow:
    HasWindowHandle + HasDisplayHandle + Any + Send + Sync
{
}

impl<T: HasWindowHandle + HasDisplayHandle + Any + Send + Sync> SurfaceWindow for T {}

/// A surface and the device-level functions that present to it. Dropping it destroys the
/// surface, so it has to be dropped after the device and before the instance.
pub(crate) struct Surface {
    pub loader: ash::khr::surface::Instance,
    /// The functions of `VK_KHR_get_surface_capabilities2`, when the instance has it.
    pub capabilities2: Option<ash::khr::get_surface_capabilities2::Instance>,
    pub handle: vk::SurfaceKHR,
    // After the handle in declaration order: the window outlives its surface.
    _window: Option<Box<dyn SurfaceWindow>>,
}

impl Drop for Surface {
    fn drop(&mut self) {
        // SAFETY: the surface belongs to the instance behind `loader`, which is still alive
        // (see the field order of `Gpu`); every swapchain created for it has been destroyed by
        // its owner, and the handle is not used again.
        unsafe { self.loader.destroy_surface(self.handle, None) };
    }
}

/// An open logical device with its one queue. Dropping it destroys the device, then the
/// surface if there is one, and then the instance; whoever creates objects from the device
/// destroys them first.
pub(crate) struct Gpu {
    pub device: ash::Device,
    pub physical: vk::PhysicalDevice,
    pub queue: vk::Queue,
    pub queue_family: u32,
    /// Whether the queue can draw as well as compute. Always true for a device opened for a
    /// surface.
    pub can_draw: bool,
    pub info: DeviceInfo,
    /// `maxImageDimension2D` of the device.
    pub max_image_dimension: u32,
    /// `maxStorageBufferRange` of the device.
    pub max_storage_buffer_range: u32,
    /// The surface the device presents to, when it was opened for one.
    pub surface: Option<Surface>,
    /// The pacing extensions the device was created with. None without a surface.
    pub pacing: PacingSupport,
    // After `device` in declaration order only for clarity; `drop` destroys the device before
    // the fields are dropped, which then happens in this order: the surface, then the instance.
    instance: Instance,
}

impl Gpu {
    /// Opens the device `choice` selects. With a log, the validation layer is enabled when it
    /// is installed and its errors are recorded there.
    pub(crate) fn new(choice: &DeviceChoice, log: Option<&ValidationLog>) -> Result<Gpu, VkError> {
        let instance = Instance::new(log, &[])?;
        let mut candidates = instance.physical_devices()?;
        let infos: Vec<DeviceInfo> = candidates.iter().map(|c| c.info.clone()).collect();
        let candidate = candidates.swap_remove(select(choice, &infos)?);
        let queue_family = candidate
            .compute_family
            .expect("a suitable device has a compute queue");
        Gpu::open(instance, candidate, queue_family, None)
    }

    /// Opens a device that can present to a surface made from `source`.
    ///
    /// An explicit choice is checked against the surface and fails when that device cannot
    /// present to it; the automatic choice takes the most preferred device that can. Only the
    /// devices tried are asked about the surface.
    pub(crate) fn with_surface(
        choice: &DeviceChoice,
        log: Option<&ValidationLog>,
        source: SurfaceSource,
    ) -> Result<Gpu, VkError> {
        let extensions: Vec<&'static CStr> = match &source {
            SurfaceSource::Headless => {
                vec![ash::khr::surface::NAME, ash::ext::headless_surface::NAME]
            }
            SurfaceSource::Window(window) => {
                let display = window
                    .display_handle()
                    .map_err(|error| DeviceError::Window(error.to_string()))?;
                ash_window::enumerate_required_extensions(display.as_raw())
                    .map_err(VkError::vulkan("ash_window::enumerate_required_extensions"))?
                    .iter()
                    // SAFETY: `ash-window` returns pointers to the NUL-terminated extension
                    // names that `ash` defines as constants, which live for the program.
                    .map(|name| unsafe { CStr::from_ptr(*name) })
                    .collect()
            }
        };
        let instance = Instance::new(log, &extensions)?;

        let handle = match &source {
            SurfaceSource::Headless => {
                let loader =
                    ash::ext::headless_surface::Instance::new(&instance._entry, &instance.raw);
                // SAFETY: the instance is live and was created with the headless-surface
                // extension; the description has no parameters.
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
                // SAFETY: the instance is live and was created with the extensions this
                // display needs. The handles were just obtained from the window, which is
                // kept alive by the `Surface` below until after the surface is destroyed.
                unsafe {
                    ash_window::create_surface(
                        &instance._entry,
                        &instance.raw,
                        display,
                        window,
                        None,
                    )
                }
                .map_err(VkError::vulkan("vkCreateSurface"))?
            }
        };
        // Declared after `instance`, so on an early return the surface is destroyed first.
        let surface = Surface {
            loader: ash::khr::surface::Instance::new(&instance._entry, &instance.raw),
            capabilities2: instance.surface_capabilities2.then(|| {
                ash::khr::get_surface_capabilities2::Instance::new(&instance._entry, &instance.raw)
            }),
            handle,
            _window: match source {
                SurfaceSource::Headless => None,
                SurfaceSource::Window(window) => Some(window),
            },
        };

        let mut candidates = instance.physical_devices()?;
        let infos: Vec<DeviceInfo> = candidates.iter().map(|c| c.info.clone()).collect();
        let mut refusal = None;
        let mut chosen = None;
        for position in surface_candidates(choice, &infos)? {
            match instance.presenting_family(&candidates[position], &surface)? {
                Ok(family) => {
                    chosen = Some((position, family));
                    break;
                }
                // The most preferred device is the one to report.
                Err(error) => refusal = refusal.or(Some(error)),
            }
        }
        let Some((position, queue_family)) = chosen else {
            return Err(refusal.expect("at least one device was tried").into());
        };
        let candidate = candidates.swap_remove(position);
        Gpu::open(instance, candidate, queue_family, Some(surface))
    }

    /// Creates the logical device with one queue of `queue_family`, and with the swapchain
    /// extension when there is a surface, together with the pacing extensions the device
    /// supports.
    fn open(
        instance: Instance,
        candidate: Candidate,
        queue_family: u32,
        surface: Option<Surface>,
    ) -> Result<Gpu, VkError> {
        let priorities = [1.0];
        let queues = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family)
            .queue_priorities(&priorities)];
        let pacing = match surface {
            // On by default; `VKXP_PRESENT_TIMING=0` switches it off. Checked live on RADV in a
            // headless mutter at 60, 120 and 144 Hz (see README).
            Some(_) if std::env::var_os("VKXP_PRESENT_TIMING").is_some_and(|v| v == "0") => {
                PacingSupport::default()
            }
            Some(_) => pacing_support(&instance.pacing_caps(candidate.physical)?),
            None => PacingSupport::default(),
        };
        let mut extensions: Vec<*const c_char> = Vec::new();
        if surface.is_some() {
            extensions.push(ash::khr::swapchain::NAME.as_ptr());
        }
        let mut timing_features = timing::PhysicalDevicePresentTimingFeatures {
            present_timing: vk::TRUE,
            present_at_absolute_time: vk::TRUE,
            ..Default::default()
        };
        let mut id_features = timing::PhysicalDevicePresentId2Features {
            present_id2: vk::TRUE,
            ..Default::default()
        };
        let mut wait_features = timing::PhysicalDevicePresentWait2Features {
            present_wait2: vk::TRUE,
            ..Default::default()
        };
        // Exactly the features the requirement check looked for, and nothing else.
        let mut features12 = vk::PhysicalDeviceVulkan12Features::default()
            .runtime_descriptor_array(true)
            .shader_sampled_image_array_non_uniform_indexing(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_sampled_image_update_after_bind(true);
        let mut features13 = vk::PhysicalDeviceVulkan13Features::default()
            .synchronization2(true)
            .dynamic_rendering(true);
        if pacing.timing {
            extensions.extend([
                timing::PRESENT_TIMING_NAME.as_ptr(),
                timing::PRESENT_ID_2_NAME.as_ptr(),
                ash::khr::calibrated_timestamps::NAME.as_ptr(),
            ]);
            timing_features.p_next = features12.p_next;
            features12.p_next = (&raw mut timing_features).cast();
        }
        if pacing.wait {
            extensions.push(timing::PRESENT_WAIT_2_NAME.as_ptr());
            id_features.p_next = features12.p_next;
            features12.p_next = (&raw mut id_features).cast();
            wait_features.p_next = features12.p_next;
            features12.p_next = (&raw mut wait_features).cast();
        }
        let info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queues)
            .enabled_extension_names(&extensions)
            .push_next(&mut features12)
            .push_next(&mut features13);
        // SAFETY: the physical device belongs to the live instance, the requested features
        // were reported as supported, the swapchain extension is only requested for a device
        // that lists it, as are the pacing extensions, whose feature structures behind
        // `features12` are live and have the layout of the header, and `info` is valid for
        // the call.
        let created = unsafe { instance.raw.create_device(candidate.physical, &info, None) };
        let device = match created {
            Ok(device) => device,
            Err(result) => {
                // Explicitly before the instance, which goes when this function returns.
                drop(surface);
                return Err(VkError::Vulkan {
                    call: "vkCreateDevice",
                    result,
                });
            }
        };
        // SAFETY: the device was created with one queue in this family.
        let queue = unsafe { device.get_device_queue(queue_family, 0) };

        Ok(Gpu {
            device,
            physical: candidate.physical,
            queue,
            queue_family,
            can_draw: candidate.families[queue_family as usize].contains(vk::QueueFlags::GRAPHICS),
            info: candidate.info,
            max_image_dimension: candidate.max_image_dimension,
            max_storage_buffer_range: candidate.max_storage_buffer_range,
            surface,
            pacing,
            instance,
        })
    }

    /// The instance the device belongs to.
    pub(crate) fn instance(&self) -> &ash::Instance {
        &self.instance.raw
    }
}

impl Drop for Gpu {
    fn drop(&mut self) {
        // SAFETY: the owner has destroyed every object it created from the device, nothing is
        // submitted after this point, and the handle is not used again.
        unsafe {
            // A failure here means the device is lost; destroying it is still the right step.
            let _ = self.device.device_wait_idle();
            self.device.destroy_device(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device that meets every requirement.
    const CAPABLE: DeviceCaps = DeviceCaps {
        api_version: (1, 3),
        compute_queue: true,
        runtime_descriptor_array: true,
        sampled_image_array_non_uniform_indexing: true,
        descriptor_binding_partially_bound: true,
        descriptor_binding_sampled_image_update_after_bind: true,
        synchronization2: true,
        dynamic_rendering: true,
        max_update_after_bind_sampled_images: BITMAP_SLOTS,
        max_work_group_invocations: 256,
        max_work_group_size: [16, 16],
    };

    fn device(index: usize, name: &str, kind: DeviceKind, caps: DeviceCaps) -> DeviceInfo {
        DeviceInfo {
            index,
            name: name.to_owned(),
            kind,
            driver: String::new(),
            lacks: lacks(&caps),
        }
    }

    fn old() -> DeviceCaps {
        DeviceCaps {
            api_version: (1, 1),
            ..CAPABLE
        }
    }

    /// A CPU device, an integrated GPU and a discrete GPU, in the least preferred order.
    fn three() -> Vec<DeviceInfo> {
        vec![
            device(0, "llvmpipe (LLVM 22)", DeviceKind::Cpu, CAPABLE),
            device(
                1,
                "Integrated Graphics (RADV)",
                DeviceKind::Integrated,
                CAPABLE,
            ),
            device(2, "Discrete Graphics 9000", DeviceKind::Discrete, CAPABLE),
        ]
    }

    #[test]
    fn capable_device_lacks_nothing() {
        assert_eq!(lacks(&CAPABLE), Vec::<String>::new());
        let newer = DeviceCaps {
            api_version: (1, 4),
            max_update_after_bind_sampled_images: u32::MAX,
            max_work_group_invocations: 1024,
            max_work_group_size: [1024, 1024],
            ..CAPABLE
        };
        assert_eq!(lacks(&newer), Vec::<String>::new());
        // A later major version also counts as at least 1.3.
        let next = DeviceCaps {
            api_version: (2, 0),
            ..CAPABLE
        };
        assert_eq!(lacks(&next), Vec::<String>::new());
    }

    #[test]
    fn each_missing_requirement_is_named() {
        let cases: [(DeviceCaps, &str); 11] = [
            (old(), "Vulkan 1.3 (supports 1.1)"),
            (
                DeviceCaps {
                    compute_queue: false,
                    ..CAPABLE
                },
                "a compute queue",
            ),
            (
                DeviceCaps {
                    runtime_descriptor_array: false,
                    ..CAPABLE
                },
                "feature runtimeDescriptorArray",
            ),
            (
                DeviceCaps {
                    sampled_image_array_non_uniform_indexing: false,
                    ..CAPABLE
                },
                "feature shaderSampledImageArrayNonUniformIndexing",
            ),
            (
                DeviceCaps {
                    descriptor_binding_partially_bound: false,
                    ..CAPABLE
                },
                "feature descriptorBindingPartiallyBound",
            ),
            (
                DeviceCaps {
                    descriptor_binding_sampled_image_update_after_bind: false,
                    ..CAPABLE
                },
                "feature descriptorBindingSampledImageUpdateAfterBind",
            ),
            (
                DeviceCaps {
                    synchronization2: false,
                    ..CAPABLE
                },
                "feature synchronization2",
            ),
            (
                DeviceCaps {
                    dynamic_rendering: false,
                    ..CAPABLE
                },
                "feature dynamicRendering",
            ),
            (
                DeviceCaps {
                    max_update_after_bind_sampled_images: 65535,
                    ..CAPABLE
                },
                "65536 update-after-bind sampled images (supports 65535)",
            ),
            (
                DeviceCaps {
                    max_work_group_invocations: 128,
                    ..CAPABLE
                },
                "16x16 compute workgroups (supports 16x16, 128 invocations)",
            ),
            (
                DeviceCaps {
                    max_work_group_size: [128, 8],
                    ..CAPABLE
                },
                "16x16 compute workgroups (supports 128x8, 256 invocations)",
            ),
        ];
        for (caps, expected) in cases {
            assert_eq!(lacks(&caps), vec![expected.to_owned()]);
        }
    }

    #[test]
    fn several_missing_requirements_are_all_reported() {
        let caps = DeviceCaps {
            compute_queue: false,
            runtime_descriptor_array: false,
            max_update_after_bind_sampled_images: 1024,
            ..CAPABLE
        };
        assert_eq!(lacks(&caps).len(), 3);
    }

    #[test]
    fn auto_prefers_discrete_then_integrated_then_other() {
        let devices = three();
        assert_eq!(select(&DeviceChoice::Auto, &devices), Ok(2));
        assert_eq!(select(&DeviceChoice::Auto, &devices[..2]), Ok(1));
        assert_eq!(select(&DeviceChoice::Auto, &devices[..1]), Ok(0));
        let others = vec![
            device(0, "virtual", DeviceKind::Virtual, CAPABLE),
            device(1, "cpu", DeviceKind::Cpu, CAPABLE),
            device(2, "other", DeviceKind::Other, CAPABLE),
        ];
        assert_eq!(
            select(&DeviceChoice::Auto, &others),
            Ok(0),
            "first of equals"
        );
    }

    #[test]
    fn auto_skips_unsuitable_devices_whatever_their_kind() {
        let devices = vec![
            device(0, "old discrete", DeviceKind::Discrete, old()),
            device(1, "old integrated", DeviceKind::Integrated, old()),
            device(2, "llvmpipe", DeviceKind::Cpu, CAPABLE),
        ];
        assert_eq!(select(&DeviceChoice::Auto, &devices), Ok(2));
    }

    #[test]
    fn auto_takes_the_first_of_two_devices_of_the_preferred_kind() {
        let devices = vec![
            device(0, "integrated", DeviceKind::Integrated, CAPABLE),
            device(1, "discrete a", DeviceKind::Discrete, CAPABLE),
            device(2, "discrete b", DeviceKind::Discrete, CAPABLE),
        ];
        assert_eq!(select(&DeviceChoice::Auto, &devices), Ok(1));
    }

    #[test]
    fn explicit_index_wins_over_the_preference_order() {
        let devices = three();
        for index in 0..3 {
            assert_eq!(select(&DeviceChoice::Index(index), &devices), Ok(index));
        }
    }

    #[test]
    fn explicit_name_wins_over_the_preference_order() {
        let devices = three();
        let name = |text: &str| DeviceChoice::Name(text.to_owned());
        assert_eq!(select(&name("llvmpipe"), &devices), Ok(0));
        assert_eq!(select(&name("RADV"), &devices), Ok(1));
        assert_eq!(select(&name("radv"), &devices), Ok(1), "case is ignored");
        assert_eq!(select(&name("9000"), &devices), Ok(2));
        // Both GPUs match; the first one in enumeration order is taken.
        assert_eq!(select(&name("Graphics"), &devices), Ok(1));
    }

    #[test]
    fn name_matching_several_devices_takes_the_first_suitable_one() {
        let devices = vec![
            device(0, "Radeon old", DeviceKind::Discrete, old()),
            device(1, "Radeon new", DeviceKind::Integrated, CAPABLE),
        ];
        assert_eq!(
            select(&DeviceChoice::Name("radeon".to_owned()), &devices),
            Ok(1)
        );
    }

    #[test]
    fn missing_explicit_device_fails_without_falling_back() {
        let devices = three();
        for choice in [
            DeviceChoice::Index(3),
            DeviceChoice::Name("GeForce".to_owned()),
        ] {
            let error = select(&choice, &devices).unwrap_err();
            assert_eq!(
                error,
                DeviceError::NotFound {
                    choice: choice.to_string(),
                    devices: devices.clone(),
                }
            );
            let text = error.to_string();
            for device in &devices {
                assert!(text.contains(&device.name), "{text}");
            }
        }
    }

    #[test]
    fn unsuitable_explicit_device_fails_without_falling_back() {
        let devices = vec![
            device(0, "old discrete", DeviceKind::Discrete, old()),
            device(1, "llvmpipe", DeviceKind::Cpu, CAPABLE),
        ];
        for choice in [DeviceChoice::Index(0), DeviceChoice::Name("old".to_owned())] {
            let error = select(&choice, &devices).unwrap_err();
            assert_eq!(
                error,
                DeviceError::Unsuitable {
                    choice: choice.to_string(),
                    devices: devices.clone(),
                }
            );
            let text = error.to_string();
            assert!(text.contains("0: old discrete (discrete GPU): lacks Vulkan 1.3"));
            assert!(text.contains("1: llvmpipe (CPU): suitable"), "{text}");
        }
    }

    #[test]
    fn no_suitable_device_error_names_every_device_and_what_it_lacks() {
        let weak = DeviceCaps {
            runtime_descriptor_array: false,
            descriptor_binding_partially_bound: false,
            ..CAPABLE
        };
        let devices = vec![
            device(0, "Old Card", DeviceKind::Discrete, old()),
            device(1, "Weak Chip", DeviceKind::Integrated, weak),
        ];
        let error = select(&DeviceChoice::Auto, &devices).unwrap_err();
        assert_eq!(
            error,
            DeviceError::NoSuitableDevice {
                devices: devices.clone(),
            }
        );
        assert_eq!(
            error.to_string(),
            "no Vulkan device meets the requirements. Available devices:\n  \
             0: Old Card (discrete GPU): lacks Vulkan 1.3 (supports 1.1)\n  \
             1: Weak Chip (integrated GPU): lacks feature runtimeDescriptorArray; \
             feature descriptorBindingPartiallyBound"
        );
    }

    #[test]
    fn no_device_at_all_is_reported_as_such() {
        let error = select(&DeviceChoice::Auto, &[]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "no Vulkan device meets the requirements. No Vulkan device is available."
        );
        assert!(matches!(
            select(&DeviceChoice::Index(0), &[]),
            Err(DeviceError::NotFound { .. })
        ));
    }

    #[test]
    fn presenting_family_is_the_first_that_computes_draws_and_presents() {
        let device = device(1, "Chip", DeviceKind::Integrated, CAPABLE);
        let family = |compute, graphics, present| FamilyCaps {
            compute,
            graphics,
            present,
        };
        let families = [
            family(true, false, false),
            family(true, true, false),
            family(false, true, true),
            family(true, true, true),
            family(true, true, true),
        ];
        assert_eq!(presenting_family(&device, true, &families), Ok(3));
    }

    #[test]
    fn device_that_cannot_present_is_refused_by_name() {
        let device = device(2, "Compute Brick 3000", DeviceKind::Discrete, CAPABLE);
        let family = |compute, graphics, present| FamilyCaps {
            compute,
            graphics,
            present,
        };

        let error = presenting_family(
            &device,
            true,
            &[family(true, true, false), family(true, false, true)],
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Vulkan device `Compute Brick 3000` cannot present to the surface: none of its \
             queue families can compute, draw and present to that surface"
        );

        let error = presenting_family(&device, false, &[family(true, true, true)]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Vulkan device `Compute Brick 3000` cannot present to the surface: it lacks the \
             VK_KHR_swapchain extension"
        );
        assert!(matches!(error, DeviceError::CannotPresent { .. }));
    }

    #[test]
    fn pacing_extensions_need_their_dependencies_and_features() {
        let all = PacingCaps {
            surface_capabilities2: true,
            present_timing_extension: true,
            present_id2_extension: true,
            present_wait2_extension: true,
            calibrated_timestamps_extension: true,
            present_timing: true,
            present_at_absolute_time: true,
            present_id2: true,
            present_wait2: true,
        };
        let both = PacingSupport {
            timing: true,
            wait: true,
        };
        assert_eq!(pacing_support(&all), both);
        // What the CPU device of the tests offers.
        assert_eq!(
            pacing_support(&PacingCaps::default()),
            PacingSupport::default()
        );
        let neither: [fn(&mut PacingCaps); 6] = [
            |caps| caps.surface_capabilities2 = false,
            |caps| caps.present_timing_extension = false,
            |caps| caps.present_id2_extension = false,
            |caps| caps.calibrated_timestamps_extension = false,
            |caps| caps.present_timing = false,
            |caps| caps.present_at_absolute_time = false,
        ];
        for remove in neither {
            let mut caps = all;
            remove(&mut caps);
            assert_eq!(pacing_support(&caps), PacingSupport::default());
        }
        let timing_only: [fn(&mut PacingCaps); 3] = [
            |caps| caps.present_wait2_extension = false,
            |caps| caps.present_id2 = false,
            |caps| caps.present_wait2 = false,
        ];
        for remove in timing_only {
            let mut caps = all;
            remove(&mut caps);
            let expected = PacingSupport {
                timing: true,
                wait: false,
            };
            assert_eq!(pacing_support(&caps), expected);
        }
    }

    #[test]
    fn surface_candidates_follow_the_preference_order_or_the_explicit_choice() {
        let devices = three();
        assert_eq!(
            surface_candidates(&DeviceChoice::Auto, &devices),
            Ok(vec![2, 1, 0])
        );
        let mixed = vec![
            device(0, "old discrete", DeviceKind::Discrete, old()),
            device(1, "llvmpipe", DeviceKind::Cpu, CAPABLE),
            device(2, "integrated", DeviceKind::Integrated, CAPABLE),
            device(3, "virtual", DeviceKind::Virtual, CAPABLE),
        ];
        assert_eq!(
            surface_candidates(&DeviceChoice::Auto, &mixed),
            Ok(vec![2, 1, 3])
        );
        // An explicit choice is one device and never widens to another.
        assert_eq!(
            surface_candidates(&DeviceChoice::Name("llvmpipe".to_owned()), &devices),
            Ok(vec![0])
        );
        assert_eq!(
            surface_candidates(&DeviceChoice::Index(1), &devices),
            Ok(vec![1])
        );
        assert!(matches!(
            surface_candidates(&DeviceChoice::Index(0), &mixed),
            Err(DeviceError::Unsuitable { .. })
        ));
        assert!(matches!(
            surface_candidates(&DeviceChoice::Auto, &mixed[..1]),
            Err(DeviceError::NoSuitableDevice { .. })
        ));
    }

    #[test]
    fn device_description_includes_the_driver_when_known() {
        let mut info = device(1, "Chip", DeviceKind::Integrated, CAPABLE);
        info.driver = "radv Mesa 26".to_owned();
        assert_eq!(
            info.to_string(),
            "1: Chip (integrated GPU, radv Mesa 26): suitable"
        );
    }
}
