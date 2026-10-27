//! A lost device reported by calls other than the wait for a submission: the commands of the
//! device are replaced by ones that answer as a driver whose device is lost, and that note
//! every call, so that a test sees which calls were made after the loss. No device is lost.

use std::cell::RefCell;

use ash::vk;
use vkrgss_compose_ref::fixtures::{self, bitmap};
use vkrgss_proto::{BitmapDelta, BitmapId, Color, FramePacket, FrameRate, Op, Rect, Size};

use super::harness::{TestCompositor, assert_clean, test_device};
use crate::{ValidationLog, VkCompositor, VkError};

thread_local! {
    /// The replaced commands called on this thread, in order. The compositor calls them on
    /// the thread of the test.
    static CALLS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn note(call: &'static str) {
    CALLS.with(|calls| calls.borrow_mut().push(call));
}

/// The replaced commands called so far, which are then forgotten.
fn take_calls() -> Vec<&'static str> {
    CALLS.with(|calls| std::mem::take(&mut *calls.borrow_mut()))
}

unsafe extern "system" fn lost_submit(
    _: vk::Queue,
    _: u32,
    _: *const vk::SubmitInfo2<'_>,
    _: vk::Fence,
) -> vk::Result {
    note("vkQueueSubmit2");
    vk::Result::ERROR_DEVICE_LOST
}

unsafe extern "system" fn failing_submit(
    _: vk::Queue,
    _: u32,
    _: *const vk::SubmitInfo2<'_>,
    _: vk::Fence,
) -> vk::Result {
    note("vkQueueSubmit2");
    vk::Result::ERROR_OUT_OF_HOST_MEMORY
}

unsafe extern "system" fn idle(_: vk::Device) -> vk::Result {
    note("vkDeviceWaitIdle");
    vk::Result::SUCCESS
}

unsafe extern "system" fn lost_idle(_: vk::Device) -> vk::Result {
    note("vkDeviceWaitIdle");
    vk::Result::ERROR_DEVICE_LOST
}

/// Notes the call and destroys nothing: it is only replaced where nothing may be destroyed.
unsafe extern "system" fn destroy_image(
    _: vk::Device,
    _: vk::Image,
    _: *const vk::AllocationCallbacks<'_>,
) {
    note("vkDestroyImage");
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn lost_query_results(
    _: vk::Device,
    _: vk::QueryPool,
    _: u32,
    _: u32,
    _: usize,
    _: *mut std::ffi::c_void,
    _: vk::DeviceSize,
    _: vk::QueryResultFlags,
) -> vk::Result {
    note("vkGetQueryPoolResults");
    vk::Result::ERROR_DEVICE_LOST
}

unsafe extern "system" fn lost_fences(
    _: vk::Device,
    _: u32,
    _: *const vk::Fence,
    _: vk::Bool32,
    _: u64,
) -> vk::Result {
    note("vkWaitForFences");
    vk::Result::ERROR_DEVICE_LOST
}

const SIZE: Size = Size::new(20, 18);
const BITMAP: BitmapId = BitmapId::new(0, 0);

fn packet(bitmaps: Vec<BitmapDelta>, red: u8) -> FramePacket {
    FramePacket {
        frame_index: 0,
        logical_size: SIZE,
        frame_rate: FrameRate::DEFAULT,
        brightness: 255,
        tables: vec![],
        bitmaps,
        ops: vec![Op::Clear {
            clip: Rect::from_size(SIZE),
            color: Color::new(red, 2, 3, 255),
        }],
    }
}

/// A compositor with a frame and a bitmap, which the next packet can destroy: its image is
/// then released once the submission of that packet is done.
fn compositor_with_a_bitmap() -> TestCompositor {
    let mut compositor = TestCompositor::new();
    let deltas = bitmap(BITMAP, 8, 8, fixtures::gradient(8, 8)).into();
    compositor.render(&packet(deltas, 1)).unwrap();
    take_calls();
    compositor
}

/// Asserts that every call that would use the device now fails with `lost` and calls none of
/// the replaced commands, and that nothing was submitted.
fn assert_refused(compositor: &mut VkCompositor, lost: &Result<(), VkError>) {
    let submissions = compositor.submissions();
    assert_eq!(compositor.render(&packet(vec![], 7)), *lost);
    assert_eq!(compositor.read_frame().map(|_| ()), *lost);
    assert_eq!(compositor.forget_resources(), *lost);
    assert_eq!(compositor.freeze(), *lost);
    assert_eq!(compositor.scale_frame(SIZE).map(|_| ()), *lost);
    assert_eq!(take_calls(), Vec::<&str>::new(), "the device was used");
    assert_eq!(compositor.submissions(), submissions);
}

/// Scenario "Lost during a frame", with the loss reported by the submission itself: it is
/// recorded before the failure is cleaned up, so the device is not idled and the image the
/// packet destroyed is not released.
#[test]
fn lost_submission_is_recorded_before_the_cleanup() {
    let mut compositor = compositor_with_a_bitmap();
    compositor.replace_device_commands(|v1_0, v1_3| {
        v1_3.queue_submit2 = lost_submit;
        v1_0.device_wait_idle = idle;
        v1_0.destroy_image = destroy_image;
    });
    let lost = Err(VkError::DeviceLost {
        call: "vkQueueSubmit2",
    });
    let destroy = vec![BitmapDelta::Destroy { id: BITMAP }];
    assert_eq!(compositor.render(&packet(destroy, 2)), lost);
    assert_eq!(take_calls(), ["vkQueueSubmit2"]);
    assert_refused(&mut compositor, &lost);
}

/// The idle that cleans up after a failed submission finds the device lost: the call fails
/// with that, and nothing is released or used after it.
#[test]
fn lost_idle_after_a_failed_submission_ends_the_call() {
    let mut compositor = compositor_with_a_bitmap();
    compositor.replace_device_commands(|v1_0, v1_3| {
        v1_3.queue_submit2 = failing_submit;
        v1_0.device_wait_idle = lost_idle;
        v1_0.destroy_image = destroy_image;
    });
    let lost = Err(VkError::DeviceLost {
        call: "vkDeviceWaitIdle",
    });
    let destroy = vec![BitmapDelta::Destroy { id: BITMAP }];
    assert_eq!(compositor.render(&packet(destroy, 2)), lost);
    assert_eq!(take_calls(), ["vkQueueSubmit2", "vkDeviceWaitIdle"]);
    assert_refused(&mut compositor, &lost);
}

/// Reading the timestamps of a measured frame finds the device lost: the frame fails.
#[test]
fn lost_device_in_the_timestamps_fails_the_frame() {
    let mut compositor = compositor_with_a_bitmap();
    if !compositor.has_timestamps() {
        eprintln!("the queue of the test device has no timestamps");
        return;
    }
    compositor.set_frame_timing(true);
    compositor.replace_device_commands(|v1_0, _| {
        v1_0.get_query_pool_results = lost_query_results;
    });
    let lost = Err(VkError::DeviceLost {
        call: "vkGetQueryPoolResults",
    });
    assert_eq!(compositor.render(&packet(vec![], 2)), lost);
    assert_eq!(take_calls(), ["vkGetQueryPoolResults"]);
    assert_refused(&mut compositor, &lost);
}

/// The wait for the fences of the presents, when a swapchain is replaced, finds the device
/// lost: the present fails, and neither the old swapchain nor anything else is used after.
#[test]
fn lost_device_in_the_wait_for_present_fences_fails_the_present() {
    let log = ValidationLog::new();
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(&log)).unwrap();
    compositor.resize_surface(Size::new(64, 48));
    compositor.render(&packet(vec![], 1)).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    if !compositor.uses_swapchain_maintenance() {
        eprintln!("{}: presents carry no fences", compositor.device());
        return;
    }
    compositor.replace_device_commands(|v1_0, _| {
        v1_0.wait_for_fences = lost_fences;
        v1_0.device_wait_idle = idle;
    });
    compositor.invalidate_swapchain();
    let lost = Err(VkError::DeviceLost {
        call: "vkWaitForFences",
    });
    assert_eq!(compositor.present().map(|_| ()), lost);
    assert_eq!(take_calls(), ["vkWaitForFences"]);
    assert_eq!(compositor.present().map(|_| ()), lost);
    assert_eq!(compositor.release_surface(), lost);
    assert_refused(&mut compositor, &lost);
    assert_clean(&log, "around the lost device");
}
