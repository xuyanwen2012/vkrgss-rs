//! The shared test harness: which device the tests run on, and the rule that no test may cause
//! a validation error.

use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use ash::vk;

use crate::device::Gpu;
use crate::{DeviceChoice, DeviceKind, ValidationLog, VkCompositor, available_devices};

/// Environment variable naming the device the tests run on: an index or part of a name.
const DEVICE_VARIABLE: &str = "VKXP_TEST_DEVICE";

/// The device every test runs on.
///
/// `VKXP_TEST_DEVICE` selects it; when the variable is unset it is the CPU device, so the
/// result does not depend on, or disturb, the hardware GPUs of the machine. There is no skip:
/// a machine without a usable device fails the tests.
pub(crate) fn test_device() -> DeviceChoice {
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

/// Panics with the recorded messages unless the log is empty.
pub(crate) fn assert_clean(log: &ValidationLog, when: &str) {
    let errors = log.errors();
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
        eprintln!("validation layer not installed; nothing to record");
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
