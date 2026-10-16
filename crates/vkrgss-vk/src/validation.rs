//! Collects the errors reported by the Vulkan validation layer.

use std::ffi::{CStr, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use ash::vk;

/// Name of the Khronos validation layer.
pub(crate) const LAYER_NAME: &CStr = c"VK_LAYER_KHRONOS_validation";

/// Name of `VK_EXT_layer_settings`, through which the layer's settings are given to the
/// instance. `ash` 0.38 has its structures but no constant for its name.
pub(crate) const LAYER_SETTINGS_NAME: &CStr = c"VK_EXT_layer_settings";

/// The setting of the validation layer that turns its synchronization validation on.
const VALIDATE_SYNC: &CStr = c"validate_sync";

/// The value the settings point to: true.
static TRUE: vk::Bool32 = vk::TRUE;

/// A record of validation errors, shared between its owner and the backend it is given to.
///
/// A backend created with a log enables `VK_LAYER_KHRONOS_validation` when the layer is
/// installed and appends every message of error severity. When the layer offers
/// `VK_EXT_layer_settings`, its synchronization validation is on as well, so a missing or
/// wrong barrier is reported as an error too. The log outlives the backend, so
/// errors reported while the backend is dropped, such as leaked objects, can be read afterwards.
#[derive(Debug, Clone, Default)]
pub struct ValidationLog {
    inner: Arc<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    active: AtomicBool,
    sync_validation: AtomicBool,
    present_fences: AtomicBool,
    errors: Mutex<Vec<String>>,
}

impl ValidationLog {
    pub fn new() -> ValidationLog {
        ValidationLog::default()
    }

    /// Whether a backend found the validation layer and is reporting into this log.
    pub fn is_active(&self) -> bool {
        self.inner.active.load(Ordering::Relaxed)
    }

    /// Whether a backend reporting into this log has the layer's synchronization validation
    /// on.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by the tests"))]
    pub(crate) fn validates_sync(&self) -> bool {
        self.inner.sync_validation.load(Ordering::Relaxed)
    }

    pub(crate) fn set_sync_validation(&self) {
        self.inner.sync_validation.store(true, Ordering::Relaxed);
    }

    /// The error messages recorded so far, oldest first.
    pub fn errors(&self) -> Vec<String> {
        self.lock().clone()
    }

    /// Whether a backend reporting into this log made a swapchain whose presents carry
    /// fences.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by the test harness"))]
    pub(crate) fn uses_present_fences(&self) -> bool {
        self.inner.present_fences.load(Ordering::Relaxed)
    }

    pub(crate) fn set_present_fences(&self) {
        self.inner.present_fences.store(true, Ordering::Relaxed);
    }

    pub(crate) fn set_active(&self) {
        self.inner.active.store(true, Ordering::Relaxed);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<String>> {
        self.inner
            .errors
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// The messenger description that reports into this log. The caller must keep a clone of
    /// the log alive for as long as an instance or messenger created from it exists.
    pub(crate) fn messenger_info(&self) -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
        // General messages are left out: they carry loader and driver chatter, such as an
        // installed driver for absent hardware failing to load, not misuse of the API.
        // Performance messages are left out too: they come as warnings, which this messenger
        // does not ask for, and whether one should fail a test is not decided yet.
        vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR)
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION)
            .pfn_user_callback(Some(record))
            .user_data(Arc::as_ptr(&self.inner) as *mut c_void)
    }
}

/// The settings of the validation layer that an instance with `VK_EXT_layer_settings` is
/// created with: synchronization validation on, everything else at the layer's defaults.
pub(crate) fn layer_settings() -> [vk::LayerSettingEXT<'static>; 1] {
    // Built by hand: the builder of the values takes bytes and would count them, where the
    // count is one of the type.
    let mut validate_sync = vk::LayerSettingEXT::default()
        .layer_name(LAYER_NAME)
        .setting_name(VALIDATE_SYNC)
        .ty(vk::LayerSettingTypeEXT::BOOL32);
    validate_sync.value_count = 1;
    validate_sync.p_values = (&raw const TRUE).cast();
    [validate_sync]
}

/// Debug-utils callback: appends the message to the log behind `user_data`.
unsafe extern "system" fn record(
    _severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _types: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    user_data: *mut c_void,
) -> vk::Bool32 {
    // SAFETY: `user_data` is the pointer `messenger_info` took from an `Arc<Inner>` that the
    // owner of the instance keeps alive while the callback can run.
    let inner = unsafe { &*(user_data as *const Inner) };
    // SAFETY: the layer passes a valid callback-data structure whose message and message id
    // name, when present, are NUL-terminated strings that live for the duration of the call.
    let (id, message) = unsafe {
        let text = |text: *const std::ffi::c_char| {
            (!text.is_null()).then(|| CStr::from_ptr(text).to_string_lossy().into_owned())
        };
        match data.as_ref() {
            Some(data) => (text(data.p_message_id_name), text(data.p_message)),
            None => (None, None),
        }
    };
    let message = message.unwrap_or_else(|| "(no message)".to_owned());
    // Newer layers leave the id, such as `SYNC-HAZARD-READ-AFTER-WRITE` or a VUID, out of the
    // message; it is put in front so that the record names what was violated.
    let message = match id {
        Some(id) if !message.contains(&id) => format!("[ {id} ] {message}"),
        _ => message,
    };
    inner
        .errors
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(message);
    vk::FALSE
}
