//! Opening the device, building the compositor on it, and taking it down.

use std::collections::{HashMap, VecDeque};
use std::mem::ManuallyDrop;

use ash::vk;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use vkrgss_proto::Size;

use super::{COMPOSE_SPIRV, Filter, Mix, PUSH_CONSTANT_BYTES, VkCompositor, binding, pacer, query};
use crate::bitmaps::BitmapTable;
use crate::device::{BITMAP_SLOTS, COMPOSE_SET_DESCRIPTORS, Gpu, SurfaceSource};
use crate::resources::create_shader_module;
use crate::tables::TableStore;
use crate::{DeviceChoice, DeviceInfo, ValidationLog, VkError};

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

    /// Opens the chosen device for the surface of `window`, so that frames can be shown with
    /// [`present`](VkCompositor::present). The device has to be able to present to that
    /// surface: an explicit choice that cannot fails with [`DeviceError::CannotPresent`], and
    /// the automatic choice takes the most preferred device that can.
    ///
    /// The window is kept alive until the compositor is dropped. With a log, the validation
    /// layer is enabled when it is installed.
    ///
    /// [`DeviceError::CannotPresent`]: crate::DeviceError::CannotPresent
    pub fn for_window<W>(
        choice: DeviceChoice,
        window: W,
        log: Option<&ValidationLog>,
    ) -> Result<VkCompositor, VkError>
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        let source = SurfaceSource::Window(Box::new(window));
        VkCompositor::finish(Gpu::with_surface(&choice, log, source)?)
    }

    /// Opens the chosen device for a surface that has no window system behind it
    /// (`VK_EXT_headless_surface`). Presenting then goes through a real swapchain whose images
    /// nobody sees, which is how presentation is tested without a display.
    pub fn with_headless_surface(
        choice: DeviceChoice,
        log: Option<&ValidationLog>,
    ) -> Result<VkCompositor, VkError> {
        VkCompositor::finish(Gpu::with_surface(&choice, log, SurfaceSource::Headless)?)
    }

    fn create(choice: &DeviceChoice, log: Option<&ValidationLog>) -> Result<VkCompositor, VkError> {
        VkCompositor::finish(Gpu::new(choice, log)?)
    }

    /// Builds the compositor on an open device.
    pub(super) fn finish(gpu: Gpu) -> Result<VkCompositor, VkError> {
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
            tables: TableStore::new(),
            table_buffer: None,
            frame_buffers: Default::default(),
            staging_needs: VecDeque::new(),
            target: None,
            has_frame: false,
            set_layout: vk::DescriptorSetLayout::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            descriptors: vk::DescriptorSet::null(),
            presenter: None,
            mix: Mix::Newest,
            filter: Filter::Auto,
            mask: None,
            surface_size: Size::new(0, 0),
            last_scaled: None,
            pacer: pacer::Pacer::new(),
            unshown_frame: false,
            starved: false,
            timing_on: false,
            frame_timing: None,
            query_pool: vk::QueryPool::null(),
            probe: Default::default(),
            present_probe: Default::default(),
            allocator: ManuallyDrop::new(allocator),
            pool: vk::CommandPool::null(),
            commands: vk::CommandBuffer::null(),
            timeline: vk::Semaphore::null(),
            submissions: 0,
            lost: None,
            gpu: ManuallyDrop::new(gpu),
        };
        if let Err(error) = compositor.init() {
            // On a lost device `drop` leaks what was made, and the device.
            compositor.note_loss(&error);
            return Err(error);
        }
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
        let mut timeline_info = vk::SemaphoreTypeCreateInfo::default()
            .semaphore_type(vk::SemaphoreType::TIMELINE)
            .initial_value(0);
        let semaphore_info = vk::SemaphoreCreateInfo::default().push_next(&mut timeline_info);
        // SAFETY: the device is live and was created with the timeline semaphore feature.
        self.timeline = unsafe { device.create_semaphore(&semaphore_info, None) }
            .map_err(VkError::vulkan("vkCreateSemaphore"))?;
        self.gpu.name_object(self.timeline, "submissions");
        if self.gpu.timestamps.is_some() {
            let query_info = vk::QueryPoolCreateInfo::default()
                .query_type(vk::QueryType::TIMESTAMP)
                .query_count(query::COUNT);
            // SAFETY: the device is live.
            self.query_pool = unsafe { device.create_query_pool(&query_info, None) }
                .map_err(VkError::vulkan("vkCreateQueryPool"))?;
        }

        // One set: the bitmap array is partially bound and updated after bind; the frame
        // target, the four storage buffers of a frame and the table buffer are plain bindings.
        let storage_buffer = |binding| {
            vk::DescriptorSetLayoutBinding::default()
                .binding(binding)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
        };
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(binding::BITMAPS)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(BITMAP_SLOTS)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(binding::FRAME)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            storage_buffer(binding::OPS),
            storage_buffer(binding::TILE_HEADERS),
            storage_buffer(binding::TILE_OPS),
            storage_buffer(binding::TILEMAPS),
            storage_buffer(binding::TABLES),
        ];
        debug_assert_eq!(
            bindings
                .iter()
                .map(|binding| binding.descriptor_count)
                .sum::<u32>(),
            COMPOSE_SET_DESCRIPTORS,
            "the device check counts the descriptors of this set"
        );
        let mut binding_flags = [vk::DescriptorBindingFlags::empty(); 7];
        binding_flags[binding::BITMAPS as usize] = vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND;
        let mut flags_info =
            vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&binding_flags);
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .bindings(&bindings)
            .push_next(&mut flags_info);
        // SAFETY: the device is live and was created with the descriptor indexing features
        // these binding flags need; the array length is within the limit checked at selection.
        self.set_layout = unsafe { device.create_descriptor_set_layout(&layout_info, None) }
            .map_err(VkError::vulkan("vkCreateDescriptorSetLayout"))?;

        let set_layouts = [self.set_layout];
        let push_ranges = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .size(PUSH_CONSTANT_BYTES)];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_ranges);
        // SAFETY: the set layout was just created from this device.
        self.pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None) }
                .map_err(VkError::vulkan("vkCreatePipelineLayout"))?;

        let module = create_shader_module(device, COMPOSE_SPIRV)?;
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(module)
            .name(c"main");
        let pipeline_info = [vk::ComputePipelineCreateInfo::default()
            .stage(stage)
            .layout(self.pipeline_layout)];
        // SAFETY: the module holds a compute shader with entry point `main` whose interface
        // matches the pipeline layout.
        let pipelines = unsafe {
            device.create_compute_pipelines(vk::PipelineCache::null(), &pipeline_info, None)
        }
        .map_err(|(_, result)| VkError::from_result("vkCreateComputePipelines", result));
        // Nothing of a lost device is used again: the module is leaked.
        if let Err(error) = &pipelines
            && error.is_device_lost()
        {
            return Err(error.clone());
        }
        // SAFETY: the module is not needed once the pipeline exists, or could not be made.
        unsafe { device.destroy_shader_module(module, None) };
        self.pipeline = pipelines?[0];
        self.gpu.name_object(self.pipeline, "compose pipeline");

        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(BITMAP_SLOTS),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(5),
        ];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
            .max_sets(1)
            .pool_sizes(&pool_sizes);
        // SAFETY: the device is live.
        self.descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None) }
            .map_err(VkError::vulkan("vkCreateDescriptorPool"))?;
        let allocate = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&set_layouts);
        // SAFETY: the pool was created for exactly one set of this layout.
        self.descriptors = unsafe { device.allocate_descriptor_sets(&allocate) }
            .map_err(VkError::vulkan("vkAllocateDescriptorSets"))?[0];
        Ok(())
    }

    /// The device the compositor runs on.
    pub fn device(&self) -> &DeviceInfo {
        &self.gpu.info
    }
}

impl Drop for VkCompositor {
    fn drop(&mut self) {
        if let Some(call) = self.lost {
            // A submission may still be running, and idling the device could wait for ever.
            // Everything is leaked; the process is about to end.
            tracing::warn!(
                "the Vulkan device was lost (found by `{call}`); its objects are leaked"
            );
            return;
        }
        let device = &self.gpu.device;
        // SAFETY: every submission was waited for, so nothing below is in use; each handle was
        // created from this device or is null, and none is used again. The allocator is dropped
        // exactly once, here, after its allocations were freed and while the device is alive;
        // the device is dropped exactly once, last.
        unsafe {
            if device.device_wait_idle() == Err(vk::Result::ERROR_DEVICE_LOST) {
                tracing::warn!("the Vulkan device was lost; its objects are leaked");
                return;
            }
            if let Some(presenter) = self.presenter.take()
                && presenter.destroy(device, &mut self.allocator).is_err()
            {
                tracing::warn!("the Vulkan device was lost; its objects are leaked");
                return;
            }
            for (_, image) in self.images.drain() {
                image.destroy(device, &mut self.allocator);
            }
            if let Some((buffer, _)) = self.table_buffer.take() {
                buffer.destroy(device, &mut self.allocator);
            }
            for (buffer, _) in self.frame_buffers.iter_mut().filter_map(Option::take) {
                buffer.destroy(device, &mut self.allocator);
            }
            if let Some(target) = self.target.take() {
                target.destroy(device, &mut self.allocator);
            }
            // Frees the descriptor set with it.
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.set_layout, None);
            device.destroy_semaphore(self.timeline, None);
            device.destroy_query_pool(self.query_pool, None);
            // Frees the command buffer with it.
            device.destroy_command_pool(self.pool, None);
            ManuallyDrop::drop(&mut self.allocator);
            ManuallyDrop::drop(&mut self.gpu);
        }
    }
}
