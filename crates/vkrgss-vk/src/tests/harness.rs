//! The shared test harness: which device the tests run on, and the rule that no test may cause
//! a validation error.

use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use ash::vk;

use crate::device::Gpu;
use crate::{DeviceChoice, DeviceKind, ValidationLog, VkCompositor, available_devices};

/// Environment variable naming the device the tests run on: an index or part of a name.
const DEVICE_VARIABLE: &str = "VKRGSS_TEST_DEVICE";

/// Environment variables that restrict which drivers the Vulkan loader loads. Naming a
/// device does not do that: the loader loads every installed driver to enumerate the devices,
/// including the driver of a GPU that must be left alone.
const ISOLATION_VARIABLES: [&str; 3] = [
    "VK_LOADER_DRIVERS_SELECT",
    "VK_DRIVER_FILES",
    "VK_ICD_FILENAMES",
];

/// Why the tests may not touch Vulkan in an environment where `variable` reads the loader's
/// variables, or `None` when one of them restricts the loader.
pub(crate) fn isolation_error(variable: impl Fn(&str) -> Option<String>) -> Option<String> {
    let restricted = ISOLATION_VARIABLES
        .iter()
        .any(|name| variable(name).is_some_and(|value| !value.trim().is_empty()));
    (!restricted).then(|| {
        format!(
            "the Vulkan loader is not restricted to chosen drivers, so these tests would load \
             the driver of every GPU in the machine just to enumerate the devices. Set \
             VK_LOADER_DRIVERS_SELECT before running them: '*lvp*' for the CPU device \
             (lavapipe), which is what the tests use by default, or the pattern of the driver \
             that {DEVICE_VARIABLE} names, for example '*radeon*' with {DEVICE_VARIABLE}=RADV."
        )
    })
}

/// Fails the test before anything is asked of the Vulkan loader unless the loader has been
/// restricted. Every test that reaches Vulkan goes through this, most of them by way of
/// [`test_device`].
pub(crate) fn require_isolation() {
    if let Some(error) = isolation_error(|name| std::env::var(name).ok()) {
        panic!("{error}");
    }
}

/// The device every test runs on.
///
/// `VKRGSS_TEST_DEVICE` selects it; when the variable is unset it is the CPU device, so the
/// result does not depend on the hardware GPUs of the machine. There is no skip: a machine
/// without a usable device fails the tests, and so does a loader that is not restricted to
/// chosen drivers (see [`require_isolation`]).
pub(crate) fn test_device() -> DeviceChoice {
    require_isolation();
    static CHOICE: OnceLock<DeviceChoice> = OnceLock::new();
    CHOICE
        .get_or_init(|| match std::env::var(DEVICE_VARIABLE) {
            Ok(value) if !value.is_empty() => match value.parse() {
                Ok(index) => DeviceChoice::Index(index),
                Err(_) => DeviceChoice::Name(value),
            },
            _ => {
                let devices = available_devices().expect("cannot enumerate Vulkan devices");
                let cpu = devices
                    .iter()
                    .find(|device| device.kind == DeviceKind::Cpu && device.is_suitable())
                    .unwrap_or_else(|| {
                        panic!(
                            "the tests need a CPU Vulkan 1.3 device (Mesa lavapipe) or \
                             {DEVICE_VARIABLE} naming another device; found: {devices:#?}"
                        )
                    });
                // By name rather than by index: a name cannot come to mean another device.
                DeviceChoice::Name(cpu.name.clone())
            }
        })
        .clone()
}

/// A small deterministic generator (xorshift64*), for test data that needs no shrinking.
pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub(crate) fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    /// A value in `0..bound`.
    pub(crate) fn below(&mut self, bound: u32) -> u32 {
        self.next_u32() % bound
    }
}

/// The report that validation layers without tracking of present fences make when the wait
/// semaphores of a present are destroyed after its present fence has signalled, which the
/// specification allows (issues 8578 and 9587 of Vulkan-ValidationLayers).
const PRESENT_FENCE_REPORT: &str = "VUID-vkDestroySemaphore-semaphore-05149";

/// The first specification version of the validation layer that is known to track present
/// fences.
const LAYER_TRACKING_PRESENT_FENCES: u32 = vk::make_api_version(0, 1, 4, 313);

/// Whether `message` is the false report of a validation layer of specification version
/// `layer` about a backend whose presents do or do not carry fences.
fn is_false_report(message: &str, layer: Option<u32>, present_fences: bool) -> bool {
    present_fences
        && message.contains(PRESENT_FENCE_REPORT)
        && layer.is_some_and(|version| version < LAYER_TRACKING_PRESENT_FENCES)
}

/// The specification version of the installed validation layer, or `None` without one.
fn layer_version() -> Option<u32> {
    static VERSION: OnceLock<Option<u32>> = OnceLock::new();
    *VERSION.get_or_init(|| {
        require_isolation();
        // SAFETY: loading the system's Vulkan library runs its initialisers and has no
        // other precondition; the call after it takes no handle.
        let layers = unsafe {
            ash::Entry::load()
                .ok()?
                .enumerate_instance_layer_properties()
        };
        layers
            .ok()?
            .iter()
            .find(|layer| layer.layer_name_as_c_str().ok() == Some(crate::validation::LAYER_NAME))
            .map(|layer| layer.spec_version)
    })
}

/// Panics with the recorded messages unless the log is empty, apart from the false report
/// of an old validation layer (see [`is_false_report`]), which is dropped with a note.
pub(crate) fn assert_clean(log: &ValidationLog, when: &str) {
    let mut errors = log.errors();
    // The layer is only asked for when the report is there.
    if log.uses_present_fences() && errors.iter().any(|e| e.contains(PRESENT_FENCE_REPORT)) {
        let layer = layer_version();
        let before = errors.len();
        errors.retain(|error| !is_false_report(error, layer, true));
        if errors.len() < before {
            eprintln!(
                "dropped {} report(s) of {PRESENT_FENCE_REPORT} {when}: this validation layer \
                 ({}) does not track present fences and takes the semaphores of a present \
                 whose fence has signalled for still in use",
                before - errors.len(),
                layer.map_or_else(String::new, version_text),
            );
        }
    }
    assert!(
        errors.is_empty(),
        "{} validation error(s) {when}:\n{}",
        errors.len(),
        errors.join("\n")
    );
}

/// A compositor on the test device with validation enabled. Dropping it asserts that no
/// validation error was recorded, both before and after the compositor itself is dropped.
pub(crate) struct TestCompositor {
    compositor: Option<VkCompositor>,
    log: ValidationLog,
}

impl TestCompositor {
    pub(crate) fn new() -> TestCompositor {
        let log = ValidationLog::new();
        let compositor = VkCompositor::with_validation(test_device(), &log)
            .unwrap_or_else(|error| panic!("cannot create the test compositor: {error}"));
        TestCompositor {
            compositor: Some(compositor),
            log,
        }
    }
}

impl Deref for TestCompositor {
    type Target = VkCompositor;

    fn deref(&self) -> &VkCompositor {
        self.compositor.as_ref().expect("present until drop")
    }
}

impl DerefMut for TestCompositor {
    fn deref_mut(&mut self) -> &mut VkCompositor {
        self.compositor.as_mut().expect("present until drop")
    }
}

impl Drop for TestCompositor {
    fn drop(&mut self) {
        if std::thread::panicking() {
            // The test already failed; a second panic would abort. Show what was recorded.
            self.compositor = None;
            for error in self.log.errors() {
                eprintln!("validation error: {error}");
            }
            return;
        }
        assert_clean(&self.log, "during the test");
        self.compositor = None;
        assert_clean(&self.log, "while the compositor was dropped");
    }
}

fn version_text(version: u32) -> String {
    format!(
        "{}.{}.{}",
        vk::api_version_major(version),
        vk::api_version_minor(version),
        vk::api_version_patch(version)
    )
}

#[test]
fn only_the_false_report_of_a_layer_without_present_fence_tracking_is_dropped() {
    let old = Some(vk::make_api_version(0, 1, 3, 275));
    let report = format!(
        "Validation Error: [ {PRESENT_FENCE_REPORT} ] Object 0: handle = 0x1 | \
         vkDestroySemaphore(): can't be called on VkSemaphore 0x1 that is currently in use"
    );
    assert!(is_false_report(&report, old, true));
    assert!(is_false_report(
        &report,
        Some(vk::make_api_version(0, 1, 4, 312)),
        true
    ));
    // A layer that tracks present fences means it, and so does any layer about a backend
    // that presents without fences.
    assert!(!is_false_report(
        &report,
        Some(LAYER_TRACKING_PRESENT_FENCES),
        true
    ));
    assert!(!is_false_report(
        &report,
        Some(vk::make_api_version(0, 1, 4, 341)),
        true
    ));
    assert!(!is_false_report(&report, old, false));
    assert!(!is_false_report(&report, None, true));
    // Every other report counts.
    let other = "Validation Error: [ VUID-vkDestroySemaphore-semaphore-parameter ] invalid";
    assert!(!is_false_report(other, old, true));
    assert!(!is_false_report(
        "vkCreateBuffer(): pCreateInfo->usage is zero.",
        old,
        true
    ));
}

#[test]
fn unrestricted_loader_is_refused_before_any_vulkan_call() {
    let with = |set: &'static [(&'static str, &'static str)]| {
        isolation_error(move |name| {
            set.iter()
                .find(|(variable, _)| *variable == name)
                .map(|(_, value)| (*value).to_owned())
        })
    };
    let error = with(&[]).expect("nothing restricts the loader");
    assert!(error.contains("VK_LOADER_DRIVERS_SELECT"), "{error}");
    assert!(error.contains("'*lvp*'"), "{error}");
    // Naming the device is not a restriction, and neither is an empty value.
    assert!(with(&[("VKRGSS_TEST_DEVICE", "llvmpipe")]).is_some());
    assert!(with(&[("VK_LOADER_DRIVERS_SELECT", " ")]).is_some());

    assert_eq!(with(&[("VK_LOADER_DRIVERS_SELECT", "*lvp*")]), None);
    assert_eq!(
        with(&[(
            "VK_DRIVER_FILES",
            "/usr/share/vulkan/icd.d/lvp_icd.x86_64.json"
        )]),
        None
    );
    assert_eq!(with(&[("VK_ICD_FILENAMES", "lvp_icd.json")]), None);
}

#[test]
fn test_device_is_the_cpu_device_unless_overridden() {
    let compositor = TestCompositor::new();
    let device = compositor.device();
    assert!(device.is_suitable());
    if std::env::var_os(DEVICE_VARIABLE).is_none() {
        assert_eq!(device.kind, DeviceKind::Cpu, "{device}");
    }
    eprintln!("test device: {device}");
}

#[test]
fn device_is_created_and_dropped_without_validation_errors() {
    let log = ValidationLog::new();
    let gpu = Gpu::new(&test_device(), Some(&log)).unwrap();
    assert_clean(&log, "after creating the device");
    drop(gpu);
    assert_clean(&log, "after dropping the device");
}

/// The harness has to notice a validation error, or a clean run would prove nothing. A buffer
/// without any usage flag is invalid and harmless to the driver.
#[test]
fn deliberate_validation_error_is_recorded() {
    let log = ValidationLog::new();
    let gpu = Gpu::new(&test_device(), Some(&log)).unwrap();
    let info = vk::BufferCreateInfo::default()
        .size(16)
        .usage(vk::BufferUsageFlags::empty());
    // SAFETY: deliberately invalid in one respect only, the empty usage, which drivers do not
    // look at for a buffer that is never bound or used; the buffer is destroyed right away.
    unsafe {
        let buffer = gpu.device.create_buffer(&info, None).unwrap();
        gpu.device.destroy_buffer(buffer, None);
    }
    drop(gpu);

    if log.is_active() {
        let errors = log.errors();
        assert!(!errors.is_empty(), "the validation layer reported nothing");
        assert!(errors[0].contains("VkBufferCreateInfo"), "{errors:?}");
        let caught = std::panic::catch_unwind(|| assert_clean(&log, "in a deliberate error"));
        assert!(caught.is_err(), "the harness must fail on a recorded error");
    } else {
        eprintln!("validation layer not installed or disabled; nothing to record");
    }
}

/// Whether the installed validation layer offers `VK_EXT_layer_settings`; false without the
/// layer.
fn layer_offers_settings() -> bool {
    require_isolation();
    // SAFETY: loading the system's Vulkan library runs its initialisers and has no other
    // precondition; the call after it takes no handle and names a layer only when it is
    // installed.
    let extensions = unsafe {
        let Ok(entry) = ash::Entry::load() else {
            return false;
        };
        if layer_version().is_none() {
            return false;
        }
        entry.enumerate_instance_extension_properties(Some(crate::validation::LAYER_NAME))
    };
    extensions.is_ok_and(|extensions| {
        extensions.iter().any(|extension| {
            extension.extension_name_as_c_str().ok() == Some(crate::validation::LAYER_SETTINGS_NAME)
        })
    })
}

#[test]
fn synchronization_validation_is_on_where_the_layer_offers_settings() {
    let log = ValidationLog::new();
    let gpu = Gpu::new(&test_device(), Some(&log)).unwrap();
    drop(gpu);
    assert_clean(&log, "while creating and dropping the device");
    let offered = log.is_active() && layer_offers_settings();
    assert_eq!(log.validates_sync(), offered);
    if !offered {
        eprintln!("the validation layer does not offer VK_EXT_layer_settings");
    }
}

/// Synchronization validation has to notice a missing barrier, or a clean run would prove
/// nothing about the barriers.
#[test]
fn deliberate_hazard_is_reported() {
    let log = ValidationLog::new();
    let mut compositor = VkCompositor::with_validation(test_device(), &log).unwrap();
    compositor.record_unguarded_read();
    drop(compositor);

    if log.validates_sync() {
        let errors = log.errors();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("SYNC-HAZARD-READ-AFTER-WRITE"),
            "the validation layer reported no hazard: {errors:?}"
        );
        let caught = std::panic::catch_unwind(|| assert_clean(&log, "in a deliberate hazard"));
        assert!(
            caught.is_err(),
            "the harness must fail on a reported hazard"
        );
    } else {
        eprintln!("synchronization validation is not on; nothing to report");
    }
}

/// Without a log the layer is not requested, whether or not it is installed.
#[test]
fn compositor_without_a_log_runs_unvalidated() {
    let compositor = VkCompositor::new(test_device()).unwrap();
    assert!(compositor.device().is_suitable());
}

#[test]
fn compositor_is_created_and_dropped_a_hundred_times() {
    let log = ValidationLog::new();
    for round in 0..100 {
        let compositor = VkCompositor::with_validation(test_device(), &log).unwrap();
        drop(compositor);
        assert_clean(&log, &format!("in round {round}"));
    }
}
