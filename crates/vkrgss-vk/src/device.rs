//! Instance and device creation, and the choice of the physical device.
//!
//! [`Gpu::new`] opens a headless device: no surface, swapchain or window-system extension is
//! involved, and the device only needs a queue that runs compute work. [`Gpu::with_surface`]
//! opens a device for a surface: the instance gets the surface extensions, the device gets
//! `VK_KHR_swapchain`, and its queue also draws and presents to that surface. Such a device
//! also gets `VK_EXT_present_timing` and `VK_KHR_present_wait2` with what they depend on,
//! when it has them; see [`pacing_support`]. It gets `VK_EXT_swapchain_maintenance1` too
//! where it and the instance offer it, and `VK_KHR_swapchain_mutable_format` where it lists
//! it.
//!
//! On macOS the instance gets `VK_KHR_portability_enumeration` when the loader offers it, so
//! that MoltenVK is enumerated, and a device that lists `VK_KHR_portability_subset` is created
//! with it.
//!
//! Every instance gets `VK_EXT_debug_utils` when the installation lists it, so that the
//! objects of the backend have names and its command buffers have labels in a capture; see
//! [`Gpu::name_object`].

use std::any::Any;
use std::ffi::{CStr, CString, c_char};

use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::present_timing as timing;
use crate::validation::ValidationLog;
use crate::{DeviceError, VkError};

mod attach;
mod instance;

use instance::{Candidate, Instance};

/// Number of bitmap slots, and so the length of the sampled-image array the shader indexes.
pub(crate) const BITMAP_SLOTS: u32 = 65536;

/// The descriptors of the composition set, which the compute stage reads: the bitmap slots,
/// the frame target and five storage buffers. Its layout is created for update after bind, so
/// each of them counts against `maxPerStageUpdateAfterBindResources`.
pub(crate) const COMPOSE_SET_DESCRIPTORS: u32 = BITMAP_SLOTS + 6;

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
    /// `maxPerStageUpdateAfterBindResources`: every descriptor of a stage, of any type.
    pub max_update_after_bind_resources: u32,
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
    if caps.max_update_after_bind_resources < COMPOSE_SET_DESCRIPTORS {
        lacks.push(format!(
            "{COMPOSE_SET_DESCRIPTORS} update-after-bind resources per stage, \
             maxPerStageUpdateAfterBindResources (supports {})",
            caps.max_update_after_bind_resources
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

/// How the timestamps of the device's queue are read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Timestamps {
    /// The bits of a timestamp that count.
    pub mask: u64,
    /// The nanoseconds of one tick.
    pub period: f32,
}

impl Timestamps {
    /// The nanoseconds from the timestamp `start` to the timestamp `end`.
    pub(crate) fn nanos(&self, start: u64, end: u64) -> u64 {
        let ticks = end.wrapping_sub(start) & self.mask;
        (ticks as f64 * f64::from(self.period)) as u64
    }
}

/// Whether a driver that offers swapchain maintenance can release the images of a swapchain
/// of a surface. `driver` is the name and information the driver reports and
/// `driver_version` its `driverVersion`.
///
/// Mesa before 25.3.4 advertises swapchain maintenance for headless surfaces and has no
/// `release_images` for them, so `vkReleaseSwapchainImagesEXT` calls a null pointer there
/// (fixed by Mesa commit c3763b12). Its other surfaces implement the call. Mesa names itself
/// in the driver information and packs the version as `VK_MAKE_VERSION(major, minor, patch)`.
pub(crate) fn releases_undrawn_images(headless: bool, driver: &str, driver_version: u32) -> bool {
    const FIXED_IN: u32 = vk::make_api_version(0, 25, 3, 4);
    !(headless && driver.contains("Mesa") && driver_version < FIXED_IN)
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
    /// Whether the surface was made for `VK_EXT_headless_surface`.
    pub headless: bool,
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
    /// `driverVersion` of the device, in the driver's own encoding.
    pub driver_version: u32,
    /// The surface the device presents to, when it was opened for one.
    pub surface: Option<Surface>,
    /// The pacing extensions the device was created with. None without a surface.
    pub pacing: PacingSupport,
    /// The time domains `vkGetPhysicalDeviceCalibrateableTimeDomainsKHR` lists, when the
    /// device was created with present timing; empty otherwise.
    pub calibrateable_domains: Vec<vk::TimeDomainKHR>,
    /// Whether the device was created with `VK_EXT_swapchain_maintenance1`. Never without a
    /// surface.
    pub swapchain_maintenance: bool,
    /// Whether the device was created with `VK_KHR_swapchain_mutable_format`, so that the
    /// images of a swapchain can be viewed in another format. Never without a surface.
    pub swapchain_mutable_format: bool,
    /// How the timestamps of the queue are read, or `None` for a queue family that has none
    /// (`timestampValidBits` of 0).
    pub timestamps: Option<Timestamps>,
    /// The commands of `VK_EXT_debug_utils` on this device, when the instance has the
    /// extension.
    debug: Option<ash::ext::debug_utils::Device>,
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
            headless: matches!(source, SurfaceSource::Headless),
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
    /// extension when there is a surface, together with the pacing extensions and the
    /// swapchain maintenance extension where the device supports them.
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
            // On by default; `VKRGSS_PRESENT_TIMING=0` switches it off. Checked live on RADV in a
            // headless mutter at 60, 120 and 144 Hz (see docs/decisions.md, milestone M5).
            Some(_) if present_timing_variable().is_some_and(|v| v == "0") => {
                PacingSupport::default()
            }
            Some(_) => pacing_support(&instance.pacing_caps(candidate.physical)?),
            None => PacingSupport::default(),
        };
        let calibrateable_domains = match pacing.timing {
            true => instance.calibrateable_domains(candidate.physical)?,
            false => Vec::new(),
        };
        let swapchain_maintenance = match surface {
            Some(_) => instance.swapchain_maintenance(candidate.physical)?,
            None => false,
        };
        let swapchain_mutable_format = match surface {
            Some(_) => instance.swapchain_mutable_format(candidate.physical)?,
            None => false,
        };
        let portability_subset =
            cfg!(target_os = "macos") && instance.portability_subset(candidate.physical)?;
        let mut extensions: Vec<*const c_char> = Vec::new();
        if surface.is_some() {
            extensions.push(ash::khr::swapchain::NAME.as_ptr());
        }
        let mut maintenance_features =
            vk::PhysicalDeviceSwapchainMaintenance1FeaturesEXT::default()
                .swapchain_maintenance1(true);
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
        // Exactly the features the requirement check looked for, and the timeline semaphores
        // of the submissions, which every device of Vulkan 1.2 or later supports.
        let mut features12 = vk::PhysicalDeviceVulkan12Features::default()
            .timeline_semaphore(true)
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
        if swapchain_maintenance {
            extensions.push(ash::ext::swapchain_maintenance1::NAME.as_ptr());
        }
        if swapchain_mutable_format {
            extensions.push(ash::khr::swapchain_mutable_format::NAME.as_ptr());
        }
        if portability_subset {
            extensions.push(ash::khr::portability_subset::NAME.as_ptr());
        }
        let mut info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queues)
            .enabled_extension_names(&extensions)
            .push_next(&mut features12)
            .push_next(&mut features13);
        if swapchain_maintenance {
            info = info.push_next(&mut maintenance_features);
        }
        // SAFETY: the physical device belongs to the live instance, the requested features
        // were reported as supported, the swapchain extension is only requested for a device
        // that lists it, as are the pacing extensions, whose feature structures behind
        // `features12` are live and have the layout of the header, and the swapchain
        // maintenance extension, whose instance extension the instance has, and the mutable
        // format and portability subset extensions, and `info` is valid for the call.
        let created = unsafe { instance.raw.create_device(candidate.physical, &info, None) };
        let device = match created {
            Ok(device) => device,
            Err(result) => {
                // Explicitly before the instance, which goes when this function returns.
                drop(surface);
                return Err(VkError::from_result("vkCreateDevice", result));
            }
        };
        // SAFETY: the device was created with one queue in this family.
        let queue = unsafe { device.get_device_queue(queue_family, 0) };
        let timestamps = match candidate.timestamp_bits[queue_family as usize] {
            0 => None,
            bits => Some(Timestamps {
                mask: u64::MAX >> (64 - bits.min(64)),
                period: candidate.timestamp_period,
            }),
        };
        let debug = instance
            .debug_utils
            .then(|| ash::ext::debug_utils::Device::new(&instance.raw, &device));

        Ok(Gpu {
            device,
            physical: candidate.physical,
            queue,
            queue_family,
            can_draw: candidate.families[queue_family as usize].contains(vk::QueueFlags::GRAPHICS),
            info: candidate.info,
            max_image_dimension: candidate.max_image_dimension,
            max_storage_buffer_range: candidate.max_storage_buffer_range,
            driver_version: candidate.driver_version,
            surface,
            pacing,
            calibrateable_domains,
            swapchain_maintenance,
            swapchain_mutable_format,
            timestamps,
            debug,
            instance,
        })
    }

    /// Whether an acquired image that was not drawn can be given back to a swapchain of the
    /// device's surface with swapchain maintenance (see [`releases_undrawn_images`]).
    pub(crate) fn releases_undrawn_images(&self) -> bool {
        let headless = self
            .surface
            .as_ref()
            .is_some_and(|surface| surface.headless);
        releases_undrawn_images(headless, &self.info.driver, self.driver_version)
    }

    /// Tells the validation log, when there is one, that presents carry fences.
    pub(crate) fn note_present_fences(&self) {
        if let Some(log) = &self.instance._log {
            log.set_present_fences();
        }
    }

    /// Gives `handle`, an object of this device, a name for captures and validation
    /// messages. Without `VK_EXT_debug_utils` nothing is named, and a failure to name is
    /// ignored.
    pub(crate) fn name_object<T: vk::Handle>(&self, handle: T, name: &str) {
        let Some(debug) = &self.debug else {
            return;
        };
        let Ok(name) = CString::new(name) else {
            return;
        };
        let info = vk::DebugUtilsObjectNameInfoEXT::default()
            .object_handle(handle)
            .object_name(&name);
        // SAFETY: the instance has the extension, every caller passes a live object of this
        // device that no other thread uses, and the name lives for the call.
        let _ = unsafe { debug.set_debug_utils_object_name(&info) };
    }

    /// Opens a labelled part of `commands`, which [`end_label`](Gpu::end_label) closes.
    /// Without `VK_EXT_debug_utils` nothing is recorded.
    ///
    /// # Safety
    ///
    /// `commands` must be a command buffer of this device in the recording state.
    pub(crate) unsafe fn begin_label(&self, commands: vk::CommandBuffer, name: &CStr) {
        if let Some(debug) = &self.debug {
            let label = vk::DebugUtilsLabelEXT::default().label_name(name);
            // SAFETY: the instance has the extension; the rest is guaranteed by the caller.
            unsafe { debug.cmd_begin_debug_utils_label(commands, &label) };
        }
    }

    /// Closes the part that the last [`begin_label`](Gpu::begin_label) opened.
    ///
    /// # Safety
    ///
    /// `commands` must be a command buffer of this device in the recording state with a
    /// label open that was begun in it.
    pub(crate) unsafe fn end_label(&self, commands: vk::CommandBuffer) {
        if let Some(debug) = &self.debug {
            // SAFETY: the instance has the extension; the rest is guaranteed by the caller.
            unsafe { debug.cmd_end_debug_utils_label(commands) };
        }
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

/// The value of `VKRGSS_PRESENT_TIMING`, if it is set. Up to version 0.3.0 the program was
/// named vkxp: for one version, `VKXP_PRESENT_TIMING` is read when the variable is not set,
/// with a warning that names both.
fn present_timing_variable() -> Option<std::ffi::OsString> {
    if let Some(value) = std::env::var_os("VKRGSS_PRESENT_TIMING") {
        return Some(value);
    }
    let value = std::env::var_os("VKXP_PRESENT_TIMING")?;
    tracing::warn!(
        "read VKXP_PRESENT_TIMING under the former name of the program; the variable is named \
         VKRGSS_PRESENT_TIMING now"
    );
    Some(value)
}

#[cfg(test)]
mod tests;
