//! The compositor: owns the device and everything created from it.

use std::mem::ManuallyDrop;

use ash::vk;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};

use crate::device::Gpu;
use crate::{DeviceChoice, DeviceInfo, ValidationLog, VkError};

/// The Vulkan compositor.
///
/// Rendering is synchronous: every call submits its work and waits for it, so no Vulkan object
/// is ever in use by the device when a method returns.
pub struct VkCompositor {
    allocator: ManuallyDrop<Allocator>,
    pool: vk::CommandPool,
    commands: vk::CommandBuffer,
    fence: vk::Fence,
    // Last, so that the device and the instance outlive everything above. `drop` destroys the
    // objects created from the device before this field is dropped.
    gpu: Gpu,
}

impl VkCompositor {
    /// Opens the chosen device, without validation.
    pub fn new(choice: DeviceChoice) -> Result<VkCompositor, VkError> {
        VkCompositor::create(&choice, None)
    }

    /// Opens the chosen device with the Khronos validation layer enabled when it is installed.
    /// Validation errors are recorded in `log`, including those reported while the compositor
    /// is dropped.
    pub fn with_validation(
        choice: DeviceChoice,
        log: &ValidationLog,
    ) -> Result<VkCompositor, VkError> {
        VkCompositor::create(&choice, Some(log))
    }

    fn create(choice: &DeviceChoice, log: Option<&ValidationLog>) -> Result<VkCompositor, VkError> {
        let gpu = Gpu::new(choice, log)?;
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: gpu.instance().clone(),
            device: gpu.device.clone(),
            physical_device: gpu.physical,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
        .map_err(|error| VkError::Memory {
            what: "the allocator",
            message: error.to_string(),
        })?;

        // Handles start out null so that `drop` can clean up after a failure part way through
        // `init`: destroying a null handle is a no-op.
        let mut compositor = VkCompositor {
            allocator: ManuallyDrop::new(allocator),
            pool: vk::CommandPool::null(),
            commands: vk::CommandBuffer::null(),
            fence: vk::Fence::null(),
            gpu,
        };
        compositor.init()?;
        Ok(compositor)
    }

    fn init(&mut self) -> Result<(), VkError> {
        let device = &self.gpu.device;
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(self.gpu.queue_family);
        // SAFETY: the device is live and the queue family is the one it was created with.
        self.pool = unsafe { device.create_command_pool(&pool_info, None) }
            .map_err(VkError::vulkan("vkCreateCommandPool"))?;
        let allocate = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        // SAFETY: the pool was just created from this device.
        self.commands = unsafe { device.allocate_command_buffers(&allocate) }
            .map_err(VkError::vulkan("vkAllocateCommandBuffers"))?[0];
        // SAFETY: the device is live.
        self.fence = unsafe { device.create_fence(&vk::FenceCreateInfo::default(), None) }
            .map_err(VkError::vulkan("vkCreateFence"))?;
        Ok(())
    }

    /// The device the compositor runs on.
    pub fn device(&self) -> &DeviceInfo {
        &self.gpu.info
    }
}

impl Drop for VkCompositor {
    fn drop(&mut self) {
        let device = &self.gpu.device;
        // SAFETY: every submission was waited for, so nothing below is in use; each handle was
        // created from this device or is null, and none is used again. The allocator is dropped
        // exactly once, here, after its allocations were freed and while the device is alive.
        unsafe {
            let _ = device.device_wait_idle();
            device.destroy_fence(self.fence, None);
            // Frees the command buffer with it.
            device.destroy_command_pool(self.pool, None);
            ManuallyDrop::drop(&mut self.allocator);
        }
    }
}
