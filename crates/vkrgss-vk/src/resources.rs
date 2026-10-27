//! Buffers and images with their memory, and the barriers between the uses of an image.

use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};

use crate::VkError;

/// Format of every bitmap and of the frame target: four unsigned 8-bit integers, read and
/// written by the shader as integers so that no value passes through a float.
pub(crate) const PIXEL_FORMAT: vk::Format = vk::Format::R8G8B8A8_UINT;

/// The single colour subresource of a 2D image.
pub(crate) const COLOR_RANGE: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};

/// The same subresource, as copies name it.
pub(crate) const COLOR_LAYERS: vk::ImageSubresourceLayers = vk::ImageSubresourceLayers {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    mip_level: 0,
    base_array_layer: 0,
    layer_count: 1,
};

/// A buffer bound to host-visible memory that stays mapped.
pub(crate) struct GpuBuffer {
    pub buffer: vk::Buffer,
    allocation: Allocation,
}

impl GpuBuffer {
    /// Creates a buffer of `size` bytes, which must not be 0.
    pub(crate) fn new(
        device: &ash::Device,
        allocator: &mut Allocator,
        name: &'static str,
        size: u64,
        usage: vk::BufferUsageFlags,
        location: MemoryLocation,
    ) -> Result<GpuBuffer, VkError> {
        let info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        // SAFETY: the device is live and `info` describes a valid buffer.
        let buffer = unsafe { device.create_buffer(&info, None) }
            .map_err(VkError::vulkan("vkCreateBuffer"))?;
        // SAFETY: the buffer was just created from this device.
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let allocation = allocator.allocate(&AllocationCreateDesc {
            name,
            requirements,
            location,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        });
        let allocation = match allocation {
            Ok(allocation) => allocation,
            Err(error) => {
                // SAFETY: the buffer is unused and not referenced anywhere else.
                unsafe { device.destroy_buffer(buffer, None) };
                return Err(memory_error(name, error));
            }
        };
        // SAFETY: the memory comes from this device's allocator, satisfies the buffer's
        // requirements, and neither is bound to anything else.
        let bound =
            unsafe { device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset()) };
        let buffer = GpuBuffer { buffer, allocation };
        match bound.map_err(VkError::vulkan("vkBindBufferMemory")) {
            Ok(()) => Ok(buffer),
            // Nothing of a lost device is used again: the buffer is leaked.
            Err(error) if error.is_device_lost() => Err(error),
            Err(error) => {
                buffer.destroy(device, allocator);
                Err(error)
            }
        }
    }

    /// The mapped contents. The memory is host-coherent, so no flush is needed.
    pub(crate) fn bytes(&self) -> &[u8] {
        self.allocation
            .mapped_slice()
            .expect("host-visible allocations are mapped")
    }

    pub(crate) fn bytes_mut(&mut self) -> &mut [u8] {
        self.allocation
            .mapped_slice_mut()
            .expect("host-visible allocations are mapped")
    }

    /// Destroys the buffer and frees its memory. The device must no longer be using it.
    pub(crate) fn destroy(self, device: &ash::Device, allocator: &mut Allocator) {
        // SAFETY: the caller guarantees the buffer is idle; it is consumed here.
        unsafe { device.destroy_buffer(self.buffer, None) };
        // Freeing only fails for an allocation the allocator does not know.
        let _ = allocator.free(self.allocation);
    }
}

/// A 2D image, of [`PIXEL_FORMAT`] unless made otherwise, with one view of the same format,
/// bound to device memory.
pub(crate) struct GpuImage {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub width: u32,
    pub height: u32,
    allocation: Allocation,
}

impl GpuImage {
    /// Creates an image of `width` by `height` pixels, neither of which may be 0.
    pub(crate) fn new(
        device: &ash::Device,
        allocator: &mut Allocator,
        name: &'static str,
        size: (u32, u32),
        usage: vk::ImageUsageFlags,
        flags: vk::ImageCreateFlags,
    ) -> Result<GpuImage, VkError> {
        GpuImage::with_format(device, allocator, name, size, PIXEL_FORMAT, usage, flags)
    }

    /// Like [`GpuImage::new`], but with `format` for the image and its view instead of
    /// [`PIXEL_FORMAT`]. The device must support the format with this usage.
    pub(crate) fn with_format(
        device: &ash::Device,
        allocator: &mut Allocator,
        name: &'static str,
        (width, height): (u32, u32),
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        flags: vk::ImageCreateFlags,
    ) -> Result<GpuImage, VkError> {
        let info = vk::ImageCreateInfo::default()
            .flags(flags)
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);
        // SAFETY: the device is live and `info` describes an image every Vulkan device supports
        // within its size limit, which the caller checked.
        let image = unsafe { device.create_image(&info, None) }
            .map_err(VkError::vulkan("vkCreateImage"))?;
        // SAFETY: the image was just created from this device.
        let requirements = unsafe { device.get_image_memory_requirements(image) };
        let allocation = allocator.allocate(&AllocationCreateDesc {
            name,
            requirements,
            location: MemoryLocation::GpuOnly,
            linear: false,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        });
        let allocation = match allocation {
            Ok(allocation) => allocation,
            Err(error) => {
                // SAFETY: the image is unused and not referenced anywhere else.
                unsafe { device.destroy_image(image, None) };
                return Err(memory_error(name, error));
            }
        };
        let mut image = GpuImage {
            image,
            view: vk::ImageView::null(),
            width,
            height,
            allocation,
        };
        // SAFETY: the memory comes from this device's allocator, satisfies the image's
        // requirements, and neither is bound to anything else.
        let bound = unsafe {
            device.bind_image_memory(
                image.image,
                image.allocation.memory(),
                image.allocation.offset(),
            )
        };
        if let Err(error) = bound.map_err(VkError::vulkan("vkBindImageMemory")) {
            // Nothing of a lost device is used again: the image is leaked.
            if !error.is_device_lost() {
                image.destroy(device, allocator);
            }
            return Err(error);
        }
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image.image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(COLOR_RANGE);
        // SAFETY: the image is bound to memory and the view matches its format and range.
        match unsafe { device.create_image_view(&view_info, None) } {
            Ok(view) => {
                image.view = view;
                Ok(image)
            }
            Err(result) => {
                let error = VkError::from_result("vkCreateImageView", result);
                // Nothing of a lost device is used again: the image is leaked.
                if !error.is_device_lost() {
                    image.destroy(device, allocator);
                }
                Err(error)
            }
        }
    }

    /// Destroys the view and the image and frees the memory. The device must no longer be
    /// using them.
    pub(crate) fn destroy(self, device: &ash::Device, allocator: &mut Allocator) {
        // SAFETY: the caller guarantees the image is idle; it is consumed here. The view may be
        // null when creation failed part way, which is a no-op.
        unsafe {
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.image, None);
        }
        // Freeing only fails for an allocation the allocator does not know.
        let _ = allocator.free(self.allocation);
    }
}

/// Creates a shader module from SPIR-V embedded by the build script.
pub(crate) fn create_shader_module(
    device: &ash::Device,
    spirv: &[u8],
) -> Result<vk::ShaderModule, VkError> {
    let words = read_words(spirv);
    let info = vk::ShaderModuleCreateInfo::default().code(&words);
    // SAFETY: the device is live and the code is SPIR-V that `spirv-val` accepted at build
    // time for the Vulkan version the device supports.
    unsafe { device.create_shader_module(&info, None) }
        .map_err(VkError::vulkan("vkCreateShaderModule"))
}

/// Copies 32-bit words to the start of `bytes` in the byte order of the machine, which is how
/// the device reads them.
pub(crate) fn write_words(bytes: &mut [u8], words: &[u32]) {
    for (target, word) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(words) {
        *target = word.to_ne_bytes();
    }
}

/// The 32-bit words that `bytes`, whose length is a multiple of four, hold.
pub(crate) fn read_words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_ne_bytes(*word))
        .collect()
}

fn memory_error(what: &'static str, error: gpu_allocator::AllocationError) -> VkError {
    VkError::Memory {
        what,
        message: error.to_string(),
    }
}

/// One way an image is used: the pipeline stages that touch it, how they access it, and the
/// layout it is in meanwhile. A barrier is recorded between two of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImageUse {
    pub stage: vk::PipelineStageFlags2,
    pub access: vk::AccessFlags2,
    pub layout: vk::ImageLayout,
}

impl ImageUse {
    /// A new image, or one whose contents may be discarded: nothing has used it.
    pub(crate) const UNDEFINED: ImageUse = ImageUse {
        stage: vk::PipelineStageFlags2::NONE,
        access: vk::AccessFlags2::NONE,
        layout: vk::ImageLayout::UNDEFINED,
    };
    /// Destination of a copy or a clear.
    pub(crate) const TRANSFER_WRITE: ImageUse = ImageUse {
        stage: vk::PipelineStageFlags2::ALL_TRANSFER,
        access: vk::AccessFlags2::TRANSFER_WRITE,
        layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
    };
    /// Source of a copy.
    pub(crate) const TRANSFER_READ: ImageUse = ImageUse {
        stage: vk::PipelineStageFlags2::ALL_TRANSFER,
        access: vk::AccessFlags2::TRANSFER_READ,
        layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
    };
    /// Fetched from as a sampled image by the compute shader.
    pub(crate) const COMPUTE_SAMPLED_READ: ImageUse = ImageUse {
        stage: vk::PipelineStageFlags2::COMPUTE_SHADER,
        access: vk::AccessFlags2::SHADER_SAMPLED_READ,
        layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    };
    /// Written as a storage image by the compute shader.
    pub(crate) const COMPUTE_STORAGE_WRITE: ImageUse = ImageUse {
        stage: vk::PipelineStageFlags2::COMPUTE_SHADER,
        access: vk::AccessFlags2::SHADER_STORAGE_WRITE,
        layout: vk::ImageLayout::GENERAL,
    };
    /// Fetched from as a sampled image by the fragment shader.
    pub(crate) const FRAGMENT_SAMPLED_READ: ImageUse = ImageUse {
        stage: vk::PipelineStageFlags2::FRAGMENT_SHADER,
        access: vk::AccessFlags2::SHADER_SAMPLED_READ,
        layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    };
    /// Drawn into as the colour attachment of a pass.
    pub(crate) const COLOR_ATTACHMENT_WRITE: ImageUse = ImageUse {
        stage: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        access: vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    };

    /// A swapchain image that was just acquired and whose old contents are not wanted. The
    /// stage is the one at which the submission waits for the acquire, so that the layout
    /// change comes after it.
    pub(crate) const ACQUIRED: ImageUse = ImageUse {
        stage: vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
        access: vk::AccessFlags2::NONE,
        layout: vk::ImageLayout::UNDEFINED,
    };
    /// Handed to the presentation engine, which no command of the device follows.
    pub(crate) const PRESENT: ImageUse = ImageUse {
        stage: vk::PipelineStageFlags2::NONE,
        access: vk::AccessFlags2::NONE,
        layout: vk::ImageLayout::PRESENT_SRC_KHR,
    };

    /// This use together with `other`, for an image that both stages use in the layout of
    /// this one.
    pub(crate) fn and(self, other: ImageUse) -> ImageUse {
        ImageUse {
            stage: self.stage | other.stage,
            access: self.access | other.access,
            layout: self.layout,
        }
    }

    /// The same stages and accesses with the image in `layout`.
    pub(crate) fn in_layout(self, layout: vk::ImageLayout) -> ImageUse {
        ImageUse { layout, ..self }
    }
}

/// Records a barrier between two uses of `image`: the commands recorded afterwards use it as
/// `to` says, once the earlier ones that used it as `from` says are done, and its layout
/// changes from the one to the other.
///
/// # Safety
///
/// `commands` must be a command buffer of `device` in the recording state, and `image` a live
/// image of that device that is in the layout of `from` (or whose contents may be discarded,
/// for `UNDEFINED`) when the barrier executes. The queue family of the command buffer must
/// support the stages of both uses.
pub(crate) unsafe fn transition(
    device: &ash::Device,
    commands: vk::CommandBuffer,
    image: vk::Image,
    from: ImageUse,
    to: ImageUse,
) {
    // SAFETY: guaranteed by the caller.
    unsafe { transition_all(device, commands, &[(image, from, to)]) };
}

/// Records the barriers between two uses of each of several images, as
/// [`transition`] does for one, in a single dependency. Nothing is recorded for none.
///
/// # Safety
///
/// As for [`transition`], for each image with its two uses; no image may appear twice.
pub(crate) unsafe fn transition_all(
    device: &ash::Device,
    commands: vk::CommandBuffer,
    transitions: &[(vk::Image, ImageUse, ImageUse)],
) {
    if transitions.is_empty() {
        return;
    }
    let barriers: Vec<vk::ImageMemoryBarrier2<'_>> = transitions
        .iter()
        .map(|&(image, from, to)| {
            vk::ImageMemoryBarrier2::default()
                .src_stage_mask(from.stage)
                .src_access_mask(from.access)
                .dst_stage_mask(to.stage)
                .dst_access_mask(to.access)
                .old_layout(from.layout)
                .new_layout(to.layout)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(COLOR_RANGE)
        })
        .collect();
    let info = vk::DependencyInfo::default().image_memory_barriers(&barriers);
    // SAFETY: guaranteed by the caller.
    unsafe { device.cmd_pipeline_barrier2(commands, &info) };
}

/// Records a barrier that makes what the device wrote to `buffer` visible to the host.
///
/// Waiting for a submission says that its commands have completed, not that
/// their writes can be seen through a mapping. That takes a dependency from the writing stage
/// to the host, and host-coherent memory does not stand in for it: coherence only removes the
/// need to invalidate the mapped range.
///
/// # Safety
///
/// `commands` must be a command buffer of `device` in the recording state and `buffer` a live
/// buffer of that device, written by the commands recorded before this one at `stage` with
/// `access`.
pub(crate) unsafe fn make_host_readable(
    device: &ash::Device,
    commands: vk::CommandBuffer,
    buffer: vk::Buffer,
    stage: vk::PipelineStageFlags2,
    access: vk::AccessFlags2,
) {
    let barriers = [vk::BufferMemoryBarrier2::default()
        .src_stage_mask(stage)
        .src_access_mask(access)
        .dst_stage_mask(vk::PipelineStageFlags2::HOST)
        .dst_access_mask(vk::AccessFlags2::HOST_READ)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .buffer(buffer)
        .offset(0)
        .size(vk::WHOLE_SIZE)];
    let info = vk::DependencyInfo::default().buffer_memory_barriers(&barriers);
    // SAFETY: guaranteed by the caller.
    unsafe { device.cmd_pipeline_barrier2(commands, &info) };
}
