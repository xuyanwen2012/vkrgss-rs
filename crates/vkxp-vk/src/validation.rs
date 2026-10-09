//! Collects the errors reported by the Vulkan validation layer.

use std::ffi::{CStr, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use ash::vk;

/// Name of the Khronos validation layer.
pub(crate) const LAYER_NAME: &CStr = c"VK_LAYER_KHRONOS_validation";

/// A record of validation errors, shared between its owner and the backend it is given to.
///
/// A backend created with a log enables `VK_LAYER_KHRONOS_validation` when the layer is
/// installed and appends every message of error severity. The log outlives the backend, so
/// errors reported while the backend is dropped, such as leaked objects, can be read afterwards.
#[derive(Debug, Clone, Default)]
pub struct ValidationLog {
    inner: Arc<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    active: AtomicBool,
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

    /// The error messages recorded so far, oldest first.
    pub fn errors(&self) -> Vec<String> {
        self.lock().clone()
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
        vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR)
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(record))
            .user_data(Arc::as_ptr(&self.inner) as *mut c_void)
    }
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
    // SAFETY: the layer passes a valid callback-data structure whose message, when present, is
    // a NUL-terminated string that lives for the duration of the call.
    let message = unsafe {
        match data.as_ref() {
            Some(data) if !data.p_message.is_null() => CStr::from_ptr(data.p_message)
                .to_string_lossy()
                .into_owned(),
            _ => "(no message)".to_owned(),
        }
    };
    inner
        .errors
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(message);
    vk::FALSE
}
