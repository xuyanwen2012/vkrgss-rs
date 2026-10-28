//! A lost device reported by calls other than the wait for a submission: the commands of the
//! device are replaced by ones that answer as a driver whose device is lost, and that note
//! every call, so that a test sees which calls were made after the loss. No device is lost.

use std::cell::{Cell, RefCell};

use ash::vk;
use vkrgss_compose_ref::fixtures::{self, bitmap};
use vkrgss_proto::{BitmapDelta, BitmapId, Color, FramePacket, FrameRate, Op, Rect, Size};

use crate::Mix;

use super::harness::{TestCompositor, assert_clean, test_device};
use crate::{OverlayImage, ValidationLog, VkCompositor, VkError};

thread_local! {
    /// The replaced commands called on this thread, in order. The compositor calls them on
    /// the thread of the test.
    static CALLS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    /// The real `vkCreateSemaphore`, for a replacement that creates the first semaphore.
    static CREATE_SEMAPHORE: Cell<Option<vk::PFN_vkCreateSemaphore>> = const { Cell::new(None) };
    /// The real `vkBindImageMemory`, for a replacement that binds the first image.
    static BIND_IMAGE: Cell<Option<vk::PFN_vkBindImageMemory>> = const { Cell::new(None) };
    /// The idles a replacement has answered.
    static IDLES: Cell<u32> = const { Cell::new(0) };
    /// The real `vkDestroyDevice`, which a replacement calls after noting the call.
    static DESTROY_DEVICE: Cell<Option<vk::PFN_vkDestroyDevice>> = const { Cell::new(None) };
    /// The real `vkDeviceWaitIdle`, which a replacement calls after noting the call.
    static DEVICE_WAIT_IDLE: Cell<Option<vk::PFN_vkDeviceWaitIdle>> = const { Cell::new(None) };
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

/// Notes the call and idles the device with the real command.
unsafe extern "system" fn noted_idle(device: vk::Device) -> vk::Result {
    note("vkDeviceWaitIdle");
    let idle = DEVICE_WAIT_IDLE
        .with(Cell::get)
        .expect("set with the replacement");
    // SAFETY: the real command of the device the compositor calls this for.
    unsafe { idle(device) }
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

unsafe extern "system" fn lost_begin(
    _: vk::CommandBuffer,
    _: *const vk::CommandBufferBeginInfo<'_>,
) -> vk::Result {
    note("vkBeginCommandBuffer");
    vk::Result::ERROR_DEVICE_LOST
}

unsafe extern "system" fn lost_image_view(
    _: vk::Device,
    _: *const vk::ImageViewCreateInfo<'_>,
    _: *const vk::AllocationCallbacks<'_>,
    _: *mut vk::ImageView,
) -> vk::Result {
    note("vkCreateImageView");
    vk::Result::ERROR_DEVICE_LOST
}

/// Notes the call and destroys nothing, like the other replaced destroys.
unsafe extern "system" fn destroy_image_view(
    _: vk::Device,
    _: vk::ImageView,
    _: *const vk::AllocationCallbacks<'_>,
) {
    note("vkDestroyImageView");
}

unsafe extern "system" fn lost_bind_buffer(
    _: vk::Device,
    _: vk::Buffer,
    _: vk::DeviceMemory,
    _: vk::DeviceSize,
) -> vk::Result {
    note("vkBindBufferMemory");
    vk::Result::ERROR_DEVICE_LOST
}

unsafe extern "system" fn destroy_buffer(
    _: vk::Device,
    _: vk::Buffer,
    _: *const vk::AllocationCallbacks<'_>,
) {
    note("vkDestroyBuffer");
}

unsafe extern "system" fn lost_semaphore(
    _: vk::Device,
    _: *const vk::SemaphoreCreateInfo<'_>,
    _: *const vk::AllocationCallbacks<'_>,
    _: *mut vk::Semaphore,
) -> vk::Result {
    note("vkCreateSemaphore");
    vk::Result::ERROR_DEVICE_LOST
}

/// Creates the first semaphore with the real command and finds the device lost at the next.
unsafe extern "system" fn second_semaphore_lost(
    device: vk::Device,
    info: *const vk::SemaphoreCreateInfo<'_>,
    allocator: *const vk::AllocationCallbacks<'_>,
    semaphore: *mut vk::Semaphore,
) -> vk::Result {
    note("vkCreateSemaphore");
    match CREATE_SEMAPHORE.with(Cell::take) {
        // SAFETY: the arguments are those the compositor passed to the real command.
        Some(create) => unsafe { create(device, info, allocator, semaphore) },
        None => vk::Result::ERROR_DEVICE_LOST,
    }
}

unsafe extern "system" fn destroy_semaphore(
    _: vk::Device,
    _: vk::Semaphore,
    _: *const vk::AllocationCallbacks<'_>,
) {
    note("vkDestroySemaphore");
}

unsafe extern "system" fn lost_pipeline_layout(
    _: vk::Device,
    _: *const vk::PipelineLayoutCreateInfo<'_>,
    _: *const vk::AllocationCallbacks<'_>,
    _: *mut vk::PipelineLayout,
) -> vk::Result {
    note("vkCreatePipelineLayout");
    vk::Result::ERROR_DEVICE_LOST
}

unsafe extern "system" fn destroy_set_layout(
    _: vk::Device,
    _: vk::DescriptorSetLayout,
    _: *const vk::AllocationCallbacks<'_>,
) {
    note("vkDestroyDescriptorSetLayout");
}

/// Binds the first image with the real command and finds the device lost at the next.
unsafe extern "system" fn second_image_bind_lost(
    device: vk::Device,
    image: vk::Image,
    memory: vk::DeviceMemory,
    offset: vk::DeviceSize,
) -> vk::Result {
    note("vkBindImageMemory");
    match BIND_IMAGE.with(Cell::take) {
        // SAFETY: the arguments are those the compositor passed to the real command.
        Some(bind) => unsafe { bind(device, image, memory, offset) },
        None => vk::Result::ERROR_DEVICE_LOST,
    }
}

/// Answers the first idle as done and finds the device lost at the next.
unsafe extern "system" fn second_idle_lost(_: vk::Device) -> vk::Result {
    note("vkDeviceWaitIdle");
    let idles = IDLES.with(|idles| idles.replace(idles.get() + 1));
    match idles {
        0 => vk::Result::SUCCESS,
        _ => vk::Result::ERROR_DEVICE_LOST,
    }
}

/// Notes the call and destroys the device with the real command.
unsafe extern "system" fn destroy_device(
    device: vk::Device,
    allocator: *const vk::AllocationCallbacks<'_>,
) {
    note("vkDestroyDevice");
    if let Some(destroy) = DESTROY_DEVICE.with(Cell::take) {
        // SAFETY: the arguments are those the compositor passed to the real command.
        unsafe { destroy(device, allocator) };
    }
}

#[allow(clippy::too_many_arguments)]
unsafe extern "system" fn lost_compute_pipelines(
    _: vk::Device,
    _: vk::PipelineCache,
    _: u32,
    _: *const vk::ComputePipelineCreateInfo<'_>,
    _: *const vk::AllocationCallbacks<'_>,
    _: *mut vk::Pipeline,
) -> vk::Result {
    note("vkCreateComputePipelines");
    vk::Result::ERROR_DEVICE_LOST
}

unsafe extern "system" fn destroy_shader_module(
    _: vk::Device,
    _: vk::ShaderModule,
    _: *const vk::AllocationCallbacks<'_>,
) {
    note("vkDestroyShaderModule");
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

/// Reading the timestamps of a measured frame finds the device lost: the call that waits for
/// the frame fails, which is the one after its render, since the render does not wait.
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
    assert_eq!(compositor.render(&packet(vec![], 2)), Ok(()));
    assert_eq!(take_calls(), Vec::<&str>::new());
    assert_eq!(compositor.read_frame().map(|_| ()), lost);
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

/// Starting to record finds the device lost: the device is not idled and the image the
/// packet destroyed is not released.
#[test]
fn lost_begin_is_recorded_before_the_cleanup() {
    let mut compositor = compositor_with_a_bitmap();
    compositor.replace_device_commands(|v1_0, _| {
        v1_0.begin_command_buffer = lost_begin;
        v1_0.device_wait_idle = idle;
        v1_0.destroy_image = destroy_image;
    });
    let lost = Err(VkError::DeviceLost {
        call: "vkBeginCommandBuffer",
    });
    let destroy = vec![BitmapDelta::Destroy { id: BITMAP }];
    assert_eq!(compositor.render(&packet(destroy, 2)), lost);
    assert_eq!(take_calls(), ["vkBeginCommandBuffer"]);
    assert_refused(&mut compositor, &lost);
}

/// The view of the image of a new bitmap finds the device lost: the image is not destroyed
/// and nothing of the packet is submitted.
#[test]
fn lost_view_of_a_new_bitmap_destroys_nothing_and_submits_nothing() {
    let mut compositor = compositor_with_a_bitmap();
    compositor.replace_device_commands(|v1_0, _| {
        v1_0.create_image_view = lost_image_view;
        v1_0.destroy_image_view = destroy_image_view;
        v1_0.destroy_image = destroy_image;
        v1_0.begin_command_buffer = lost_begin;
        v1_0.device_wait_idle = idle;
    });
    let submissions = compositor.submissions();
    let lost = Err(VkError::DeviceLost {
        call: "vkCreateImageView",
    });
    let deltas = bitmap(BitmapId::new(1, 0), 8, 8, fixtures::gradient(8, 8)).into();
    assert_eq!(compositor.render(&packet(deltas, 2)), lost);
    assert_eq!(take_calls(), ["vkCreateImageView"]);
    assert_eq!(compositor.submissions(), submissions);
    assert_refused(&mut compositor, &lost);
}

/// Binding the memory of a larger staging buffer finds the device lost: the buffer is not
/// destroyed, and the packet is neither recorded nor submitted.
#[test]
fn lost_bind_of_a_frame_buffer_destroys_nothing_and_submits_nothing() {
    let mut compositor = compositor_with_a_bitmap();
    compositor.replace_device_commands(|v1_0, _| {
        v1_0.bind_buffer_memory = lost_bind_buffer;
        v1_0.destroy_buffer = destroy_buffer;
        v1_0.begin_command_buffer = lost_begin;
        v1_0.device_wait_idle = idle;
    });
    let submissions = compositor.submissions();
    let lost = Err(VkError::DeviceLost {
        call: "vkBindBufferMemory",
    });
    // Four times the upload of the bitmap there is, so the staging buffer has to grow.
    let deltas = bitmap(BitmapId::new(1, 0), 16, 16, fixtures::gradient(16, 16)).into();
    assert_eq!(compositor.render(&packet(deltas, 2)), lost);
    assert_eq!(take_calls(), ["vkBindBufferMemory"]);
    assert_eq!(compositor.submissions(), submissions);
    assert_refused(&mut compositor, &lost);
}

/// A compositor for a headless surface that has presented once, or `None` where an undrawn
/// image is not given back, so that a failed draw replaces the swapchain instead.
fn presenting_compositor(log: &ValidationLog) -> Option<VkCompositor> {
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(log)).unwrap();
    compositor.resize_surface(Size::new(64, 48));
    compositor.render(&packet(vec![], 1)).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    if !compositor.releases_undrawn_images() {
        eprintln!("{}: undrawn images are not given back", compositor.device());
        return None;
    }
    take_calls();
    Some(compositor)
}

/// After a failed draw the semaphores that replace those of the undrawn image cannot be
/// made, the first or the second, because the device is lost: the present fails with that,
/// nothing is destroyed, and nothing of the device is used after it.
#[test]
fn lost_semaphore_after_a_failed_draw_fails_the_present() {
    let log = ValidationLog::new();
    for second in [false, true] {
        let Some(mut compositor) = presenting_compositor(&log) else {
            return;
        };
        compositor.replace_device_commands(|v1_0, _| {
            if second {
                CREATE_SEMAPHORE.with(|create| create.set(Some(v1_0.create_semaphore)));
                v1_0.create_semaphore = second_semaphore_lost;
            } else {
                v1_0.create_semaphore = lost_semaphore;
            }
            v1_0.destroy_semaphore = destroy_semaphore;
        });
        compositor.fail_next_draw();
        let lost = Err(VkError::DeviceLost {
            call: "vkCreateSemaphore",
        });
        assert_eq!(compositor.present().map(|_| ()), lost, "second: {second}");
        let expected: &[&str] = match second {
            true => &["vkCreateSemaphore", "vkCreateSemaphore"],
            false => &["vkCreateSemaphore"],
        };
        assert_eq!(take_calls(), expected, "second: {second}");
        assert_eq!(compositor.present().map(|_| ()), lost);
        assert_refused(&mut compositor, &lost);
    }
    assert_clean(&log, "around the lost device");
}

/// A view of the images of a new swapchain finds the device lost: the new swapchain is not
/// destroyed, and nothing of the device is used after it.
#[test]
fn lost_view_of_a_new_swapchain_fails_the_present() {
    let log = ValidationLog::new();
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(&log)).unwrap();
    compositor.resize_surface(Size::new(64, 48));
    compositor.render(&packet(vec![], 1)).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    compositor.replace_device_commands(|v1_0, _| {
        v1_0.create_image_view = lost_image_view;
        v1_0.destroy_image_view = destroy_image_view;
        v1_0.destroy_semaphore = destroy_semaphore;
    });
    take_calls();
    compositor.invalidate_swapchain();
    let lost = Err(VkError::DeviceLost {
        call: "vkCreateImageView",
    });
    assert_eq!(compositor.present().map(|_| ()), lost);
    // The old swapchain is retired before the new one is filled; nothing follows the view.
    let calls = take_calls();
    assert_eq!(calls.last(), Some(&"vkCreateImageView"), "{calls:?}");
    assert_eq!(
        calls
            .iter()
            .filter(|call| **call == "vkCreateImageView")
            .count(),
        1
    );
    assert_eq!(compositor.present().map(|_| ()), lost);
    assert_refused(&mut compositor, &lost);
    assert_clean(&log, "around the lost device");
}

/// What a draw needs, or the overlay, cannot be made because the device is lost: what was
/// made of it is not destroyed, and nothing of the device is used after it.
#[test]
fn lost_device_while_the_presenter_or_the_overlay_is_made_destroys_nothing() {
    let log = ValidationLog::new();
    let lost = Err(VkError::DeviceLost {
        call: "vkCreatePipelineLayout",
    });
    let replace = |v1_0: &mut ash::DeviceFnV1_0, _: &mut ash::DeviceFnV1_3| {
        v1_0.create_pipeline_layout = lost_pipeline_layout;
        v1_0.destroy_descriptor_set_layout = destroy_set_layout;
    };

    // The presenter, made by the first scaled frame.
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(&log)).unwrap();
    compositor.render(&packet(vec![], 1)).unwrap();
    take_calls();
    compositor.replace_device_commands(replace);
    assert_eq!(compositor.scale_frame(SIZE).map(|_| ()), lost);
    assert_eq!(take_calls(), ["vkCreatePipelineLayout"]);
    assert_refused(&mut compositor, &lost);

    // The overlay, made by the first picture.
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(&log)).unwrap();
    compositor.resize_surface(Size::new(64, 48));
    compositor.render(&packet(vec![], 1)).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    take_calls();
    compositor.replace_device_commands(replace);
    let picture = OverlayImage {
        size: Size::new(4, 4),
        pixels: vec![255; 4 * 4 * 4],
    };
    assert_eq!(compositor.set_overlay(Some(&picture)), lost);
    assert_eq!(take_calls(), ["vkCreatePipelineLayout"]);
    assert_eq!(compositor.set_overlay(Some(&picture)), lost);
    assert_refused(&mut compositor, &lost);
    assert_clean(&log, "around the lost device");
}

/// Allocating the buffer that `scale_frame` reads the off-screen target back into, in the
/// step that runs once the target is drawn, finds the device lost: the target is not
/// destroyed, and nothing of the device is used after it.
#[test]
fn lost_readback_buffer_leaves_the_off_screen_target_alone() {
    let mut compositor = compositor_with_a_bitmap();
    compositor.scale_frame(SIZE).unwrap();
    compositor.replace_device_commands(|v1_0, _| {
        v1_0.bind_buffer_memory = lost_bind_buffer;
        v1_0.destroy_image_view = destroy_image_view;
        v1_0.destroy_image = destroy_image;
    });
    let lost = Err(VkError::DeviceLost {
        call: "vkBindBufferMemory",
    });
    assert_eq!(compositor.scale_frame(SIZE).map(|_| ()), lost);
    assert_eq!(take_calls(), ["vkBindBufferMemory"]);
    assert_refused(&mut compositor, &lost);
}

/// A freeze in the middle of a mix draws what is shown into an off-screen target and then
/// makes a held frame of a new size from it, whose memory finds the device lost: neither
/// image is destroyed, and nothing of the device is used after it.
#[test]
fn lost_held_frame_of_a_freeze_leaves_the_off_screen_target_alone() {
    let mut compositor = compositor_with_a_bitmap();
    compositor.freeze().unwrap();
    // The state is shown at 40 by 30 from now on, so the freeze below captures that size.
    compositor.scale_frame(Size::new(40, 30)).unwrap();
    compositor.set_mix(Mix::Step {
        weight: 128,
        threshold: 128,
    });
    compositor.replace_device_commands(|v1_0, _| {
        BIND_IMAGE.with(|bind| bind.set(Some(v1_0.bind_image_memory)));
        v1_0.bind_image_memory = second_image_bind_lost;
        v1_0.destroy_image_view = destroy_image_view;
        v1_0.destroy_image = destroy_image;
    });
    let lost = Err(VkError::DeviceLost {
        call: "vkBindImageMemory",
    });
    assert_eq!(compositor.freeze(), lost);
    // The off-screen target, then the held frame.
    assert_eq!(take_calls(), ["vkBindImageMemory", "vkBindImageMemory"]);
    assert_refused(&mut compositor, &lost);
}

/// Allocating the staging buffer of an overlay picture finds the device lost: the image of
/// the overlay is not destroyed, and nothing of the device is used after it.
#[test]
fn lost_staging_buffer_of_an_overlay_leaves_its_image_alone() {
    let mut compositor = compositor_with_a_bitmap();
    compositor.scale_frame(SIZE).unwrap();
    compositor.replace_device_commands(|v1_0, _| {
        v1_0.bind_buffer_memory = lost_bind_buffer;
        v1_0.destroy_image_view = destroy_image_view;
        v1_0.destroy_image = destroy_image;
    });
    let lost = Err(VkError::DeviceLost {
        call: "vkBindBufferMemory",
    });
    let picture = OverlayImage {
        size: Size::new(4, 4),
        pixels: vec![255; 4 * 4 * 4],
    };
    assert_eq!(compositor.set_overlay(Some(&picture)), lost);
    assert_eq!(take_calls(), ["vkBindBufferMemory"]);
    assert_eq!(compositor.set_overlay(Some(&picture)), lost);
    assert_refused(&mut compositor, &lost);
}

/// Making the compute pipeline finds the device lost while the compositor is set up: the
/// shader module, and everything made before it, is not destroyed, and the device is not
/// idled.
#[test]
fn lost_device_while_the_compositor_is_set_up_destroys_nothing() {
    let made = VkCompositor::with_device_commands(|v1_0, _| {
        v1_0.create_compute_pipelines = lost_compute_pipelines;
        v1_0.destroy_shader_module = destroy_shader_module;
        v1_0.device_wait_idle = idle;
    });
    assert_eq!(
        made.err(),
        Some(VkError::DeviceLost {
            call: "vkCreateComputePipelines"
        })
    );
    assert_eq!(take_calls(), ["vkCreateComputePipelines"]);
}

/// The wait for the fences of the presents, when a compositor that presented is dropped,
/// finds the device lost: nothing more is destroyed.
#[test]
fn lost_device_in_the_wait_for_present_fences_stops_the_drop() {
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
        v1_0.destroy_image_view = destroy_image_view;
        v1_0.destroy_image = destroy_image;
        v1_0.destroy_semaphore = destroy_semaphore;
        v1_0.destroy_descriptor_set_layout = destroy_set_layout;
    });
    take_calls();
    drop(compositor);
    assert_eq!(take_calls(), ["vkWaitForFences"]);
    assert_clean(&log, "around the lost device");
}

/// The idle of the device itself, once the compositor has destroyed what it made, finds the
/// device lost: the device is not destroyed.
#[test]
fn lost_device_in_the_last_idle_leaves_the_device_alone() {
    let mut compositor = VkCompositor::new(test_device()).unwrap();
    compositor.render(&packet(vec![], 1)).unwrap();
    IDLES.with(|idles| idles.set(0));
    compositor.replace_device_commands(|v1_0, _| {
        DESTROY_DEVICE.with(|destroy| destroy.set(Some(v1_0.destroy_device)));
        v1_0.device_wait_idle = second_idle_lost;
        v1_0.destroy_device = destroy_device;
    });
    take_calls();
    drop(compositor);
    // That of the compositor, then that of the device.
    assert_eq!(take_calls(), ["vkDeviceWaitIdle", "vkDeviceWaitIdle"]);
}

// The compose of the last render in flight when the device is found lost.

/// A frame that destroys the bitmap is still in flight when a freeze finds the device lost
/// while it makes the presenter: the freeze does not wait for the compose, and the image of
/// the destroyed bitmap, which the compose may still use, is not released.
#[test]
fn lost_device_found_with_a_compose_in_flight_releases_nothing_it_uses() {
    let mut compositor = compositor_with_a_bitmap();
    let destroy = vec![BitmapDelta::Destroy { id: BITMAP }];
    compositor.render(&packet(destroy, 2)).unwrap();
    assert!(compositor.has_compose_in_flight());
    compositor.replace_device_commands(|v1_0, _| {
        v1_0.create_pipeline_layout = lost_pipeline_layout;
        v1_0.destroy_image = destroy_image;
    });
    let lost = Err(VkError::DeviceLost {
        call: "vkCreatePipelineLayout",
    });
    assert_eq!(compositor.freeze(), lost);
    assert_eq!(take_calls(), ["vkCreatePipelineLayout"]);
    assert_refused(&mut compositor, &lost);
}

/// A compositor for a headless surface that has presented once and has the compose of a
/// second frame in flight, whose bounded wait is made to run out, and whose idles are noted.
fn compositor_with_a_stalled_compose(log: &ValidationLog, maintenance: bool) -> VkCompositor {
    let mut compositor = VkCompositor::with_headless_surface(test_device(), Some(log)).unwrap();
    if !maintenance {
        compositor.ignore_swapchain_maintenance();
    }
    compositor.resize_surface(Size::new(64, 48));
    compositor.render(&packet(vec![], 1)).unwrap();
    assert_eq!(compositor.present(), Ok(true));
    compositor.render(&packet(vec![], 2)).unwrap();
    compositor.inject_wait_result(vk::Result::TIMEOUT);
    compositor.replace_device_commands(|v1_0, _| {
        DEVICE_WAIT_IDLE.with(|idle| idle.set(Some(v1_0.device_wait_idle)));
        v1_0.device_wait_idle = noted_idle;
    });
    take_calls();
    compositor
}

/// A draw that fails is cleaned up by idling the device, which is not bounded: the compose
/// in flight is waited for first with the bounded wait, which runs out, and the device is
/// not idled.
#[test]
fn failed_draw_waits_for_the_compose_with_the_bounded_wait_before_any_idle() {
    let log = ValidationLog::new();
    let mut compositor = compositor_with_a_stalled_compose(&log, true);
    compositor.fail_next_draw();
    let lost = Err(VkError::DeviceLost {
        call: "vkWaitSemaphores",
    });
    assert_eq!(compositor.present().map(|_| ()), lost);
    assert_eq!(take_calls(), Vec::<&str>::new(), "the device was idled");
    assert_refused(&mut compositor, &lost);
}

/// A swapchain whose presents carry no fences is replaced after idling the device, which is
/// not bounded: the compose in flight is waited for first with the bounded wait, which runs
/// out, and the device is not idled.
#[test]
fn replaced_swapchain_waits_for_the_compose_with_the_bounded_wait_before_any_idle() {
    let log = ValidationLog::new();
    let mut compositor = compositor_with_a_stalled_compose(&log, false);
    compositor.invalidate_swapchain();
    let lost = Err(VkError::DeviceLost {
        call: "vkWaitSemaphores",
    });
    assert_eq!(compositor.present().map(|_| ()), lost);
    assert_eq!(take_calls(), Vec::<&str>::new(), "the device was idled");
    assert_refused(&mut compositor, &lost);
}
