//! The compositor: owns the device and everything created from it.

use std::collections::{BTreeSet, HashMap};
use std::mem::ManuallyDrop;

use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use vkxp_proto::{BitmapDelta, Rect};

use crate::bitmaps::{BitmapTable, Change};
use crate::device::Gpu;
use crate::resources::{COLOR_LAYERS, COLOR_RANGE, GpuBuffer, GpuImage, transition};
use crate::{DeviceChoice, DeviceInfo, ValidationLog, VkError};

#[cfg(test)]
mod test_compute;

/// The Vulkan compositor.
///
/// Rendering is synchronous: every call submits its work and waits for it, so no Vulkan object
/// is ever in use by the device when a method returns.
pub struct VkCompositor {
    /// Which slot holds which bitmap. The images below follow it change by change.
    table: BitmapTable,
    /// The image of every live bitmap that has pixels, by slot. Between submissions each is in
    /// the `SHADER_READ_ONLY_OPTIMAL` layout.
    images: HashMap<u32, GpuImage>,
    allocator: ManuallyDrop<Allocator>,
    pool: vk::CommandPool,
    commands: vk::CommandBuffer,
    fence: vk::Fence,
    // Last, so that the device and the instance outlive everything above. `drop` destroys the
    // objects created from the device before this field is dropped.
    gpu: Gpu,
}

/// The GPU work that the deltas of one packet have queued. It is carried out even when a later
/// delta or an op turns out to be invalid, because the deltas before the error stay applied.
#[derive(Default)]
struct Pending<'a> {
    /// What to record, in delta order.
    steps: Vec<Step<'a>>,
    /// Total bytes of all uploads, the size of the staging buffer.
    staging_len: u64,
    /// Slots whose occupant changed, so their descriptor has to be rewritten.
    touched: BTreeSet<u32>,
    /// Images of destroyed bitmaps, freed once the packet's work has completed.
    retired: Vec<GpuImage>,
}

enum Step<'a> {
    /// Fill a new image with transparent black.
    Clear { image: vk::Image },
    /// Replace the pixels inside `rect` with `rgba`, staged at `offset`.
    Upload {
        image: vk::Image,
        rect: Rect,
        offset: u64,
        rgba: &'a [u8],
    },
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
            table: BitmapTable::new(gpu.max_image_dimension),
            images: HashMap::new(),
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

    /// Largest width or height of an image on the device.
    #[cfg(test)]
    pub(crate) fn max_image_dimension(&self) -> u32 {
        self.gpu.max_image_dimension
    }

    /// Applies bitmap deltas in order and carries out their GPU work. The first invalid delta
    /// stops the sequence; the deltas before it stay applied.
    pub(crate) fn apply_bitmaps(&mut self, deltas: &[BitmapDelta]) -> Result<(), VkError> {
        let mut pending = Pending::default();
        let checked = self.queue_deltas(deltas, &mut pending);
        // A failure of the device outranks an invalid delta.
        self.execute(pending)?;
        checked
    }

    /// Checks each delta against the table, applies it there and queues its GPU work.
    fn queue_deltas<'a>(
        &mut self,
        deltas: &'a [BitmapDelta],
        pending: &mut Pending<'a>,
    ) -> Result<(), VkError> {
        for (index, delta) in deltas.iter().enumerate() {
            match self.table.apply(delta).map_err(|error| error.at(index))? {
                Change::Created {
                    slot,
                    width,
                    height,
                } => {
                    pending.touched.insert(slot);
                    if width == 0 || height == 0 {
                        // No pixels, so no image; nothing can sample this bitmap.
                        continue;
                    }
                    let image = GpuImage::new(
                        &self.gpu.device,
                        &mut self.allocator,
                        "bitmap",
                        (width, height),
                        // Transfer source only for the readback the tests use.
                        vk::ImageUsageFlags::SAMPLED
                            | vk::ImageUsageFlags::TRANSFER_DST
                            | vk::ImageUsageFlags::TRANSFER_SRC,
                        vk::ImageCreateFlags::empty(),
                    );
                    match image {
                        Ok(image) => {
                            pending.steps.push(Step::Clear { image: image.image });
                            self.images.insert(slot, image);
                        }
                        Err(error) => {
                            self.table.forget(slot);
                            return Err(error);
                        }
                    }
                }
                Change::Updated { slot, rect } => {
                    let BitmapDelta::Update { rgba, .. } = delta else {
                        unreachable!("only an update delta yields an update");
                    };
                    let image = self.images[&slot].image;
                    pending.steps.push(Step::Upload {
                        image,
                        rect,
                        offset: pending.staging_len,
                        rgba,
                    });
                    pending.staging_len += rgba.len() as u64;
                }
                Change::Destroyed { slot } => {
                    pending.touched.insert(slot);
                    // The image may still be the target of steps queued by this packet.
                    pending.retired.extend(self.images.remove(&slot));
                }
                Change::Nothing => {}
            }
        }
        Ok(())
    }

    /// Carries out queued work: one submission, waited for, then the release of everything the
    /// work was the last user of.
    fn execute(&mut self, pending: Pending<'_>) -> Result<(), VkError> {
        let mut staging = None;
        let result = self.submit_pending(&pending, &mut staging);
        if result.is_err() {
            // SAFETY: the device is live. Whatever was submitted has to finish before the
            // objects below are destroyed; if this fails too the device is lost.
            let _ = unsafe { self.gpu.device.device_wait_idle() };
        }
        if let Some(staging) = staging {
            staging.destroy(&self.gpu.device, &mut self.allocator);
        }
        for image in pending.retired {
            image.destroy(&self.gpu.device, &mut self.allocator);
        }
        result
    }

    fn submit_pending(
        &mut self,
        pending: &Pending<'_>,
        staging: &mut Option<GpuBuffer>,
    ) -> Result<(), VkError> {
        if pending.steps.is_empty() {
            return Ok(());
        }
        let device = &self.gpu.device;
        let mut staging_buffer = vk::Buffer::null();
        if pending.staging_len > 0 {
            let buffer = staging.insert(GpuBuffer::new(
                device,
                &mut self.allocator,
                "bitmap staging",
                pending.staging_len,
                vk::BufferUsageFlags::TRANSFER_SRC,
                MemoryLocation::CpuToGpu,
            )?);
            let bytes = buffer.bytes_mut();
            for step in &pending.steps {
                if let Step::Upload { offset, rgba, .. } = step {
                    let start = *offset as usize;
                    bytes[start..start + rgba.len()].copy_from_slice(rgba);
                }
            }
            staging_buffer = buffer.buffer;
        }

        self.begin_commands()?;
        let (device, commands) = (&self.gpu.device, self.commands);
        for step in &pending.steps {
            // SAFETY: the command buffer is recording. Every image named by a step is alive
            // until `execute` has waited for this submission: live ones are in `self.images`
            // and destroyed ones in `pending.retired`. A new image starts in `UNDEFINED`; any
            // other is in `SHADER_READ_ONLY_OPTIMAL`, where the previous step or submission
            // left it. Upload rectangles were checked against the image size and their bytes
            // lie inside the staging buffer.
            unsafe {
                match step {
                    Step::Clear { image } => {
                        transition(
                            device,
                            commands,
                            *image,
                            vk::ImageLayout::UNDEFINED,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        );
                        device.cmd_clear_color_image(
                            commands,
                            *image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            &vk::ClearColorValue { uint32: [0; 4] },
                            &[COLOR_RANGE],
                        );
                    }
                    Step::Upload {
                        image,
                        rect,
                        offset,
                        ..
                    } => {
                        transition(
                            device,
                            commands,
                            *image,
                            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        );
                        let region = vk::BufferImageCopy::default()
                            .buffer_offset(*offset)
                            .image_subresource(COLOR_LAYERS)
                            .image_offset(vk::Offset3D {
                                x: rect.x,
                                y: rect.y,
                                z: 0,
                            })
                            .image_extent(vk::Extent3D {
                                width: rect.width,
                                height: rect.height,
                                depth: 1,
                            });
                        device.cmd_copy_buffer_to_image(
                            commands,
                            staging_buffer,
                            *image,
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                            &[region],
                        );
                    }
                }
                let (Step::Clear { image } | Step::Upload { image, .. }) = step;
                transition(
                    device,
                    commands,
                    *image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                );
            }
        }
        self.submit_and_wait()
    }

    /// Resets the command buffer and starts recording into it.
    fn begin_commands(&self) -> Result<(), VkError> {
        let info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        // SAFETY: the command buffer belongs to this device, its pool allows resetting single
        // buffers, and it is not pending: every submission is waited for.
        unsafe {
            self.gpu
                .device
                .begin_command_buffer(self.commands, &info)
                .map_err(VkError::vulkan("vkBeginCommandBuffer"))
        }
    }

    /// Ends recording, submits the command buffer and blocks until the device has finished it.
    fn submit_and_wait(&self) -> Result<(), VkError> {
        let device = &self.gpu.device;
        let buffers = [self.commands];
        let submit = vk::SubmitInfo::default().command_buffers(&buffers);
        // SAFETY: the command buffer is recording and everything it references stays alive
        // until the wait below returns; the queue and fence belong to this device, the fence
        // is unsignalled, and the queue is only used from here.
        unsafe {
            device
                .end_command_buffer(self.commands)
                .map_err(VkError::vulkan("vkEndCommandBuffer"))?;
            device
                .queue_submit(self.gpu.queue, &[submit], self.fence)
                .map_err(VkError::vulkan("vkQueueSubmit"))?;
            device
                .wait_for_fences(&[self.fence], true, u64::MAX)
                .map_err(VkError::vulkan("vkWaitForFences"))?;
            device
                .reset_fences(&[self.fence])
                .map_err(VkError::vulkan("vkResetFences"))
        }
    }

    /// Copies the pixels of an image to host memory: RGBA8 in row order.
    ///
    /// `layout` is the layout the image is in, which is restored; an image in `GENERAL` is
    /// copied from as it is.
    fn read_image(
        &mut self,
        image: vk::Image,
        (width, height): (u32, u32),
        layout: vk::ImageLayout,
    ) -> Result<Vec<u8>, VkError> {
        let size = u64::from(width) * u64::from(height) * 4;
        let buffer = GpuBuffer::new(
            &self.gpu.device,
            &mut self.allocator,
            "readback",
            size,
            vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuToCpu,
        )?;
        let result = self.copy_image_to_buffer(image, (width, height), layout, buffer.buffer);
        if result.is_err() {
            // SAFETY: the device is live; see `execute`.
            let _ = unsafe { self.gpu.device.device_wait_idle() };
        }
        let pixels = result.map(|()| buffer.bytes()[..size as usize].to_vec());
        buffer.destroy(&self.gpu.device, &mut self.allocator);
        pixels
    }

    fn copy_image_to_buffer(
        &self,
        image: vk::Image,
        (width, height): (u32, u32),
        layout: vk::ImageLayout,
        buffer: vk::Buffer,
    ) -> Result<(), VkError> {
        self.begin_commands()?;
        let (device, commands) = (&self.gpu.device, self.commands);
        let copy_layout = match layout {
            vk::ImageLayout::GENERAL => layout,
            _ => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        };
        let region = vk::BufferImageCopy::default()
            .image_subresource(COLOR_LAYERS)
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });
        // SAFETY: the command buffer is recording; the caller passes a live image of this size
        // in `layout`, created with transfer-source usage, and a live buffer that holds the
        // whole image; both outlive the wait in `submit_and_wait`.
        unsafe {
            if copy_layout != layout {
                transition(device, commands, image, layout, copy_layout);
            }
            device.cmd_copy_image_to_buffer(commands, image, copy_layout, buffer, &[region]);
            if copy_layout != layout {
                transition(device, commands, image, copy_layout, layout);
            }
        }
        self.submit_and_wait()
    }

    /// The pixels of a live bitmap, read back from its image.
    #[cfg(test)]
    pub(crate) fn read_bitmap(&mut self, id: vkxp_proto::BitmapId) -> Result<Vec<u8>, VkError> {
        self.table
            .get(id)
            .map_err(|source| VkError::InvalidOpBitmap { index: 0, source })?;
        match self.images.get(&id.index) {
            Some(image) => self.read_image(
                image.image,
                (image.width, image.height),
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ),
            None => Ok(Vec::new()),
        }
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
            for (_, image) in self.images.drain() {
                image.destroy(device, &mut self.allocator);
            }
            device.destroy_fence(self.fence, None);
            // Frees the command buffer with it.
            device.destroy_command_pool(self.pool, None);
            ManuallyDrop::drop(&mut self.allocator);
        }
    }
}
