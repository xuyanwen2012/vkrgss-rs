//! Hand-written bindings of `VK_EXT_present_timing`, `VK_KHR_present_id2` and
//! `VK_KHR_present_wait2`, which the `ash` release in use predates.
//!
//! Only what the presenter needs is here. The definitions are transcribed from
//! `vulkan_core.h` of the Vulkan headers that define revision 3 of `VK_EXT_present_timing`;
//! the layout test at the end checks every structure against the C layout of its fields.

use std::ffi::{CStr, c_void};
use std::ptr;

use ash::vk;

pub(crate) const PRESENT_TIMING_NAME: &CStr = c"VK_EXT_present_timing";
pub(crate) const PRESENT_ID_2_NAME: &CStr = c"VK_KHR_present_id2";
pub(crate) const PRESENT_WAIT_2_NAME: &CStr = c"VK_KHR_present_wait2";

const fn structure_type(value: i32) -> vk::StructureType {
    vk::StructureType::from_raw(value)
}

const PHYSICAL_DEVICE_PRESENT_TIMING_FEATURES: vk::StructureType = structure_type(1000208000);
const SWAPCHAIN_TIME_DOMAIN_PROPERTIES: vk::StructureType = structure_type(1000208002);
const PRESENT_TIMINGS_INFO: vk::StructureType = structure_type(1000208003);
const PRESENT_TIMING_INFO: vk::StructureType = structure_type(1000208004);
const PAST_PRESENTATION_TIMING_INFO: vk::StructureType = structure_type(1000208005);
const PAST_PRESENTATION_TIMING_PROPERTIES: vk::StructureType = structure_type(1000208006);
const PAST_PRESENTATION_TIMING: vk::StructureType = structure_type(1000208007);
const PRESENT_TIMING_SURFACE_CAPABILITIES: vk::StructureType = structure_type(1000208008);
const SWAPCHAIN_CALIBRATED_TIMESTAMP_INFO: vk::StructureType = structure_type(1000208009);
const SURFACE_CAPABILITIES_PRESENT_ID_2: vk::StructureType = structure_type(1000479000);
const PRESENT_ID_2: vk::StructureType = structure_type(1000479001);
const PHYSICAL_DEVICE_PRESENT_ID_2_FEATURES: vk::StructureType = structure_type(1000479002);
const SURFACE_CAPABILITIES_PRESENT_WAIT_2: vk::StructureType = structure_type(1000480000);
const PHYSICAL_DEVICE_PRESENT_WAIT_2_FEATURES: vk::StructureType = structure_type(1000480001);
const PRESENT_WAIT_2_INFO: vk::StructureType = structure_type(1000480002);

/// `VK_SWAPCHAIN_CREATE_PRESENT_TIMING_BIT_EXT`.
pub(crate) const SWAPCHAIN_CREATE_PRESENT_TIMING: vk::SwapchainCreateFlagsKHR =
    vk::SwapchainCreateFlagsKHR::from_raw(0x0000_0200);
/// `VK_SWAPCHAIN_CREATE_PRESENT_ID_2_BIT_KHR`.
pub(crate) const SWAPCHAIN_CREATE_PRESENT_ID_2: vk::SwapchainCreateFlagsKHR =
    vk::SwapchainCreateFlagsKHR::from_raw(0x0000_0040);
/// `VK_SWAPCHAIN_CREATE_PRESENT_WAIT_2_BIT_KHR`.
pub(crate) const SWAPCHAIN_CREATE_PRESENT_WAIT_2: vk::SwapchainCreateFlagsKHR =
    vk::SwapchainCreateFlagsKHR::from_raw(0x0000_0080);

/// `VK_TIME_DOMAIN_PRESENT_STAGE_LOCAL_EXT`.
pub(crate) const TIME_DOMAIN_PRESENT_STAGE_LOCAL: vk::TimeDomainKHR =
    vk::TimeDomainKHR::from_raw(1000208000);
/// `VK_TIME_DOMAIN_SWAPCHAIN_LOCAL_EXT`.
pub(crate) const TIME_DOMAIN_SWAPCHAIN_LOCAL: vk::TimeDomainKHR =
    vk::TimeDomainKHR::from_raw(1000208001);

/// `VkPresentStageFlagBitsEXT`, in the order a present goes through them.
pub(crate) const PRESENT_STAGES: [u32; 4] = [
    // VK_PRESENT_STAGE_QUEUE_OPERATIONS_END_BIT_EXT
    0x1, // VK_PRESENT_STAGE_REQUEST_DEQUEUED_BIT_EXT
    0x2, // VK_PRESENT_STAGE_IMAGE_FIRST_PIXEL_OUT_BIT_EXT
    0x4, // VK_PRESENT_STAGE_IMAGE_FIRST_PIXEL_VISIBLE_BIT_EXT
    0x8,
];

/// Declares a structure that starts with `sType` and `pNext`, with its `Default`: the
/// structure type set and everything else zero.
macro_rules! structure {
    (
        $(#[$meta:meta])*
        $name:ident = $s_type:expr, $next:ty { $(pub $field:ident: $ty:ty = $zero:expr,)* }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        pub(crate) struct $name {
            pub s_type: vk::StructureType,
            pub p_next: $next,
            $(pub $field: $ty,)*
        }

        impl Default for $name {
            fn default() -> $name {
                $name {
                    s_type: $s_type,
                    p_next: ptr::null_mut::<c_void>() as $next,
                    $($field: $zero,)*
                }
            }
        }
    };
}

structure! {
    /// `VkPhysicalDevicePresentTimingFeaturesEXT`.
    PhysicalDevicePresentTimingFeatures = PHYSICAL_DEVICE_PRESENT_TIMING_FEATURES, *mut c_void {
        pub present_timing: vk::Bool32 = vk::FALSE,
        pub present_at_absolute_time: vk::Bool32 = vk::FALSE,
        pub present_at_relative_time: vk::Bool32 = vk::FALSE,
    }
}

structure! {
    /// `VkPhysicalDevicePresentId2FeaturesKHR`.
    PhysicalDevicePresentId2Features = PHYSICAL_DEVICE_PRESENT_ID_2_FEATURES, *mut c_void {
        pub present_id2: vk::Bool32 = vk::FALSE,
    }
}

structure! {
    /// `VkPhysicalDevicePresentWait2FeaturesKHR`.
    PhysicalDevicePresentWait2Features = PHYSICAL_DEVICE_PRESENT_WAIT_2_FEATURES, *mut c_void {
        pub present_wait2: vk::Bool32 = vk::FALSE,
    }
}

structure! {
    /// `VkPresentTimingSurfaceCapabilitiesEXT`.
    PresentTimingSurfaceCapabilities = PRESENT_TIMING_SURFACE_CAPABILITIES, *mut c_void {
        pub present_timing_supported: vk::Bool32 = vk::FALSE,
        pub present_at_absolute_time_supported: vk::Bool32 = vk::FALSE,
        pub present_at_relative_time_supported: vk::Bool32 = vk::FALSE,
        pub present_stage_queries: u32 = 0,
    }
}

structure! {
    /// `VkSurfaceCapabilitiesPresentId2KHR`.
    SurfaceCapabilitiesPresentId2 = SURFACE_CAPABILITIES_PRESENT_ID_2, *mut c_void {
        pub present_id2_supported: vk::Bool32 = vk::FALSE,
    }
}

structure! {
    /// `VkSurfaceCapabilitiesPresentWait2KHR`.
    SurfaceCapabilitiesPresentWait2 = SURFACE_CAPABILITIES_PRESENT_WAIT_2, *mut c_void {
        pub present_wait2_supported: vk::Bool32 = vk::FALSE,
    }
}

structure! {
    /// `VkSwapchainCalibratedTimestampInfoEXT`.
    SwapchainCalibratedTimestampInfo = SWAPCHAIN_CALIBRATED_TIMESTAMP_INFO, *const c_void {
        pub swapchain: vk::SwapchainKHR = vk::SwapchainKHR::null(),
        pub present_stage: u32 = 0,
        pub time_domain_id: u64 = 0,
    }
}

structure! {
    /// `VkSwapchainTimeDomainPropertiesEXT`.
    SwapchainTimeDomainProperties = SWAPCHAIN_TIME_DOMAIN_PROPERTIES, *mut c_void {
        pub time_domain_count: u32 = 0,
        pub p_time_domains: *mut vk::TimeDomainKHR = ptr::null_mut(),
        pub p_time_domain_ids: *mut u64 = ptr::null_mut(),
    }
}

structure! {
    /// `VkPastPresentationTimingInfoEXT`.
    PastPresentationTimingInfo = PAST_PRESENTATION_TIMING_INFO, *const c_void {
        pub flags: u32 = 0,
        pub swapchain: vk::SwapchainKHR = vk::SwapchainKHR::null(),
    }
}

/// `VkPresentStageTimeEXT`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct PresentStageTime {
    pub stage: u32,
    pub time: u64,
}

structure! {
    /// `VkPastPresentationTimingEXT`.
    PastPresentationTiming = PAST_PRESENTATION_TIMING, *mut c_void {
        pub present_id: u64 = 0,
        pub target_time: u64 = 0,
        pub present_stage_count: u32 = 0,
        pub p_present_stages: *mut PresentStageTime = ptr::null_mut(),
        pub time_domain: vk::TimeDomainKHR = vk::TimeDomainKHR::DEVICE,
        pub time_domain_id: u64 = 0,
        pub report_complete: vk::Bool32 = vk::FALSE,
    }
}

structure! {
    /// `VkPastPresentationTimingPropertiesEXT`.
    PastPresentationTimingProperties = PAST_PRESENTATION_TIMING_PROPERTIES, *mut c_void {
        pub timing_properties_counter: u64 = 0,
        pub time_domains_counter: u64 = 0,
        pub presentation_timing_count: u32 = 0,
        pub p_presentation_timings: *mut PastPresentationTiming = ptr::null_mut(),
    }
}

structure! {
    /// `VkPresentTimingInfoEXT`.
    PresentTimingInfo = PRESENT_TIMING_INFO, *const c_void {
        pub flags: u32 = 0,
        pub target_time: u64 = 0,
        pub time_domain_id: u64 = 0,
        pub present_stage_queries: u32 = 0,
        pub target_time_domain_present_stage: u32 = 0,
    }
}

structure! {
    /// `VkPresentTimingsInfoEXT`.
    PresentTimingsInfo = PRESENT_TIMINGS_INFO, *const c_void {
        pub swapchain_count: u32 = 0,
        pub p_timing_infos: *const PresentTimingInfo = ptr::null(),
    }
}

structure! {
    /// `VkPresentId2KHR`.
    PresentId2 = PRESENT_ID_2, *const c_void {
        pub swapchain_count: u32 = 0,
        pub p_present_ids: *const u64 = ptr::null(),
    }
}

structure! {
    /// `VkPresentWait2InfoKHR`.
    PresentWait2Info = PRESENT_WAIT_2_INFO, *const c_void {
        pub present_id: u64 = 0,
        pub timeout: u64 = 0,
    }
}

type SetSwapchainPresentTimingQueueSize =
    unsafe extern "system" fn(vk::Device, vk::SwapchainKHR, u32) -> vk::Result;
type GetSwapchainTimeDomainProperties = unsafe extern "system" fn(
    vk::Device,
    vk::SwapchainKHR,
    *mut SwapchainTimeDomainProperties,
    *mut u64,
) -> vk::Result;
type GetPastPresentationTiming = unsafe extern "system" fn(
    vk::Device,
    *const PastPresentationTimingInfo,
    *mut PastPresentationTimingProperties,
) -> vk::Result;
type WaitForPresent2 =
    unsafe extern "system" fn(vk::Device, vk::SwapchainKHR, *const PresentWait2Info) -> vk::Result;

/// The device-level commands of the extensions, loaded with `vkGetDeviceProcAddr`.
#[derive(Clone, Copy)]
pub(crate) struct Commands {
    pub set_swapchain_present_timing_queue_size: SetSwapchainPresentTimingQueueSize,
    pub get_swapchain_time_domain_properties: GetSwapchainTimeDomainProperties,
    pub get_past_presentation_timing: GetPastPresentationTiming,
    /// `None` when `VK_KHR_present_wait2` is not enabled on the device.
    pub wait_for_present2: Option<WaitForPresent2>,
}

impl Commands {
    /// Loads the commands of a device that was created with `VK_EXT_present_timing`, and
    /// with `VK_KHR_present_wait2` when `wait` says so. `None` when the device does not
    /// give one of them.
    ///
    /// # Safety
    ///
    /// `device` must be a live device of `instance` that was created with those extensions.
    pub(crate) unsafe fn load(
        instance: &ash::Instance,
        device: &ash::Device,
        wait: bool,
    ) -> Option<Commands> {
        let load = |name: &CStr| {
            // SAFETY: the device is live and belongs to the instance, as the caller
            // guarantees, and the name is NUL-terminated.
            unsafe { instance.get_device_proc_addr(device.handle(), name.as_ptr()) }
        };
        // SAFETY: each pointer is the device's implementation of the command it was asked
        // for by name, whose signature, from the header, is the type it is turned into.
        // Function pointers of any signature have the same size.
        unsafe {
            Some(Commands {
                set_swapchain_present_timing_queue_size: std::mem::transmute::<
                    unsafe extern "system" fn(),
                    SetSwapchainPresentTimingQueueSize,
                >(load(
                    c"vkSetSwapchainPresentTimingQueueSizeEXT",
                )?),
                get_swapchain_time_domain_properties: std::mem::transmute::<
                    unsafe extern "system" fn(),
                    GetSwapchainTimeDomainProperties,
                >(load(
                    c"vkGetSwapchainTimeDomainPropertiesEXT",
                )?),
                get_past_presentation_timing: std::mem::transmute::<
                    unsafe extern "system" fn(),
                    GetPastPresentationTiming,
                >(load(
                    c"vkGetPastPresentationTimingEXT",
                )?),
                wait_for_present2: match wait {
                    true => Some(std::mem::transmute::<
                        unsafe extern "system" fn(),
                        WaitForPresent2,
                    >(load(c"vkWaitForPresent2KHR")?)),
                    false => None,
                },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{align_of, offset_of, size_of};

    use super::*;

    /// A C type of the header, as far as layout goes.
    #[derive(Clone, Copy)]
    enum C {
        /// `uint32_t`, and what is defined as one: `VkBool32`, `VkFlags`, and an enumeration
        /// such as `VkStructureType` or `VkTimeDomainKHR`.
        U32,
        /// `uint64_t`, and a non-dispatchable handle such as `VkSwapchainKHR`.
        U64,
        /// Any pointer.
        Pointer,
    }

    impl C {
        fn size_and_alignment(self) -> (usize, usize) {
            match self {
                C::U32 => (4, align_of::<u32>()),
                C::U64 => (8, align_of::<u64>()),
                C::Pointer => (size_of::<*const c_void>(), align_of::<*const c_void>()),
            }
        }
    }

    /// The offset of each field of a C structure with these fields, and its size.
    fn c_layout(fields: &[C]) -> (Vec<usize>, usize) {
        let mut offsets = Vec::new();
        let (mut end, mut alignment) = (0_usize, 1);
        for field in fields {
            let (size, align) = field.size_and_alignment();
            let offset = end.next_multiple_of(align);
            offsets.push(offset);
            end = offset + size;
            alignment = alignment.max(align);
        }
        (offsets, end.next_multiple_of(alignment))
    }

    /// Checks a binding against the fields of the header's definition, given in its order
    /// with their C types.
    macro_rules! assert_layout {
        ($name:ident { $($field:ident: $c:expr,)* }) => {{
            let (offsets, size) = c_layout(&[$($c),*]);
            let found = [$(offset_of!($name, $field)),*];
            assert_eq!(found.as_slice(), offsets.as_slice(), stringify!($name));
            assert_eq!(size_of::<$name>(), size, stringify!($name));
        }};
    }

    #[test]
    fn c_layout_pads_fields_and_the_end_as_a_c_compiler_does() {
        // struct { uint32_t; void*; uint32_t; }
        let (offsets, size) = c_layout(&[C::U32, C::Pointer, C::U32]);
        let pointer = size_of::<*const c_void>();
        assert_eq!(offsets, [0, pointer, 2 * pointer]);
        assert_eq!(size, 2 * pointer + 4_usize.next_multiple_of(pointer));
    }

    #[test]
    fn bindings_have_the_layout_of_the_header_definitions() {
        use C::{Pointer, U32, U64};
        // typedef struct VkPhysicalDevicePresentTimingFeaturesEXT {
        //     VkStructureType sType; void* pNext; VkBool32 presentTiming;
        //     VkBool32 presentAtAbsoluteTime; VkBool32 presentAtRelativeTime; }
        assert_layout!(PhysicalDevicePresentTimingFeatures {
            s_type: U32,
            p_next: Pointer,
            present_timing: U32,
            present_at_absolute_time: U32,
            present_at_relative_time: U32,
        });
        // { VkStructureType sType; void* pNext; VkBool32 presentId2; }
        assert_layout!(PhysicalDevicePresentId2Features {
            s_type: U32,
            p_next: Pointer,
            present_id2: U32,
        });
        // { VkStructureType sType; void* pNext; VkBool32 presentWait2; }
        assert_layout!(PhysicalDevicePresentWait2Features {
            s_type: U32,
            p_next: Pointer,
            present_wait2: U32,
        });
        // { VkStructureType sType; void* pNext; VkBool32 presentTimingSupported;
        //   VkBool32 presentAtAbsoluteTimeSupported; VkBool32 presentAtRelativeTimeSupported;
        //   VkPresentStageFlagsEXT presentStageQueries; }
        assert_layout!(PresentTimingSurfaceCapabilities {
            s_type: U32,
            p_next: Pointer,
            present_timing_supported: U32,
            present_at_absolute_time_supported: U32,
            present_at_relative_time_supported: U32,
            present_stage_queries: U32,
        });
        // { VkStructureType sType; void* pNext; VkBool32 presentId2Supported; }
        assert_layout!(SurfaceCapabilitiesPresentId2 {
            s_type: U32,
            p_next: Pointer,
            present_id2_supported: U32,
        });
        // { VkStructureType sType; void* pNext; VkBool32 presentWait2Supported; }
        assert_layout!(SurfaceCapabilitiesPresentWait2 {
            s_type: U32,
            p_next: Pointer,
            present_wait2_supported: U32,
        });
        // { VkStructureType sType; const void* pNext; VkSwapchainKHR swapchain;
        //   VkPresentStageFlagsEXT presentStage; uint64_t timeDomainId; }
        assert_layout!(SwapchainCalibratedTimestampInfo {
            s_type: U32,
            p_next: Pointer,
            swapchain: U64,
            present_stage: U32,
            time_domain_id: U64,
        });
        // { VkStructureType sType; void* pNext; uint32_t timeDomainCount;
        //   VkTimeDomainKHR* pTimeDomains; uint64_t* pTimeDomainIds; }
        assert_layout!(SwapchainTimeDomainProperties {
            s_type: U32,
            p_next: Pointer,
            time_domain_count: U32,
            p_time_domains: Pointer,
            p_time_domain_ids: Pointer,
        });
        // { VkStructureType sType; const void* pNext;
        //   VkPastPresentationTimingFlagsEXT flags; VkSwapchainKHR swapchain; }
        assert_layout!(PastPresentationTimingInfo {
            s_type: U32,
            p_next: Pointer,
            flags: U32,
            swapchain: U64,
        });
        // { VkPresentStageFlagsEXT stage; uint64_t time; }
        assert_layout!(PresentStageTime {
            stage: U32,
            time: U64,
        });
        // { VkStructureType sType; void* pNext; uint64_t presentId; uint64_t targetTime;
        //   uint32_t presentStageCount; VkPresentStageTimeEXT* pPresentStages;
        //   VkTimeDomainKHR timeDomain; uint64_t timeDomainId; VkBool32 reportComplete; }
        assert_layout!(PastPresentationTiming {
            s_type: U32,
            p_next: Pointer,
            present_id: U64,
            target_time: U64,
            present_stage_count: U32,
            p_present_stages: Pointer,
            time_domain: U32,
            time_domain_id: U64,
            report_complete: U32,
        });
        // { VkStructureType sType; void* pNext; uint64_t timingPropertiesCounter;
        //   uint64_t timeDomainsCounter; uint32_t presentationTimingCount;
        //   VkPastPresentationTimingEXT* pPresentationTimings; }
        assert_layout!(PastPresentationTimingProperties {
            s_type: U32,
            p_next: Pointer,
            timing_properties_counter: U64,
            time_domains_counter: U64,
            presentation_timing_count: U32,
            p_presentation_timings: Pointer,
        });
        // { VkStructureType sType; const void* pNext; VkPresentTimingInfoFlagsEXT flags;
        //   uint64_t targetTime; uint64_t timeDomainId;
        //   VkPresentStageFlagsEXT presentStageQueries;
        //   VkPresentStageFlagsEXT targetTimeDomainPresentStage; }
        assert_layout!(PresentTimingInfo {
            s_type: U32,
            p_next: Pointer,
            flags: U32,
            target_time: U64,
            time_domain_id: U64,
            present_stage_queries: U32,
            target_time_domain_present_stage: U32,
        });
        // { VkStructureType sType; const void* pNext; uint32_t swapchainCount;
        //   const VkPresentTimingInfoEXT* pTimingInfos; }
        assert_layout!(PresentTimingsInfo {
            s_type: U32,
            p_next: Pointer,
            swapchain_count: U32,
            p_timing_infos: Pointer,
        });
        // { VkStructureType sType; const void* pNext; uint32_t swapchainCount;
        //   const uint64_t* pPresentIds; }
        assert_layout!(PresentId2 {
            s_type: U32,
            p_next: Pointer,
            swapchain_count: U32,
            p_present_ids: Pointer,
        });
        // { VkStructureType sType; const void* pNext; uint64_t presentId; uint64_t timeout; }
        assert_layout!(PresentWait2Info {
            s_type: U32,
            p_next: Pointer,
            present_id: U64,
            timeout: U64,
        });
    }

    #[test]
    fn defaults_carry_the_structure_types_of_the_header() {
        let types = [
            (
                PhysicalDevicePresentTimingFeatures::default().s_type,
                1000208000,
            ),
            (SwapchainTimeDomainProperties::default().s_type, 1000208002),
            (PresentTimingsInfo::default().s_type, 1000208003),
            (PresentTimingInfo::default().s_type, 1000208004),
            (PastPresentationTimingInfo::default().s_type, 1000208005),
            (
                PastPresentationTimingProperties::default().s_type,
                1000208006,
            ),
            (PastPresentationTiming::default().s_type, 1000208007),
            (
                PresentTimingSurfaceCapabilities::default().s_type,
                1000208008,
            ),
            (
                SwapchainCalibratedTimestampInfo::default().s_type,
                1000208009,
            ),
            (SurfaceCapabilitiesPresentId2::default().s_type, 1000479000),
            (PresentId2::default().s_type, 1000479001),
            (
                PhysicalDevicePresentId2Features::default().s_type,
                1000479002,
            ),
            (
                SurfaceCapabilitiesPresentWait2::default().s_type,
                1000480000,
            ),
            (
                PhysicalDevicePresentWait2Features::default().s_type,
                1000480001,
            ),
            (PresentWait2Info::default().s_type, 1000480002),
        ];
        for (s_type, value) in types {
            assert_eq!(s_type.as_raw(), value);
        }
    }
}
