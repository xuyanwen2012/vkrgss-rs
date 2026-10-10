//! Where the time of a frame goes, as the compositor measures it on request.

use std::time::Duration;

/// The durations of the stages of one frame, measured when
/// [`set_frame_timing`](crate::VkCompositor::set_frame_timing) asked for it and handed out by
/// [`take_frame_timing`](crate::VkCompositor::take_frame_timing).
///
/// The processor durations are those of the calling thread's wall clock. A stage that did
/// not run for the frame is `None`: the stages of a render before the first one, the present
/// of a frame that was rendered and not shown. The device durations are the differences of
/// timestamps the device wrote, and are `None` on a queue without timestamps as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameTiming {
    /// Checking the ops of the packet and encoding them as the records the shader reads.
    pub encode: Option<Duration>,
    /// Assigning the records to the tiles they touch.
    pub bin: Option<Duration>,
    /// Copying the bitmap updates and the records into the buffers of the frame and
    /// recording the commands.
    pub upload: Option<Duration>,
    /// Submitting the commands and waiting until the device has finished them.
    pub submit: Option<Duration>,
    /// Recording the presentation pass, submitting it and waiting for it.
    pub present: Option<Duration>,
    /// Device time of the compose dispatch.
    pub compose_device: Option<Duration>,
    /// Device time of the presentation pass.
    pub present_device: Option<Duration>,
}

impl FrameTiming {
    /// Every stage with a short name: the processor stages in the order a frame goes through
    /// them, then the device durations.
    pub fn stages(&self) -> [(&'static str, Option<Duration>); 7] {
        [
            ("encode", self.encode),
            ("bin", self.bin),
            ("upload", self.upload),
            ("submit", self.submit),
            ("present", self.present),
            ("compose (device)", self.compose_device),
            ("present (device)", self.present_device),
        ]
    }
}
