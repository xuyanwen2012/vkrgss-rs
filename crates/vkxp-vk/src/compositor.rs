//! The compositor: owns the device and everything created from it.

use std::collections::{BTreeSet, HashMap};
use std::mem::ManuallyDrop;

use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use vkxp_proto::{BitmapDelta, FramePacket, GuestToHost, Rect, Size};

use crate::binning::{TileLists, bin};
use crate::bitmaps::{BitmapTable, Change};
use crate::device::{BITMAP_SLOTS, Gpu, SurfaceSource};
use crate::records::{ClipRect, OpRecord, RECORD_WORDS, check_logical_size, encode_ops};
use crate::resources::{
    COLOR_LAYERS, COLOR_RANGE, GpuBuffer, GpuImage, create_shader_module, make_host_readable,
    transition, write_words,
};
use crate::{DeviceChoice, DeviceInfo, Limit, ValidationLog, VkError};

/// The compositing shader, compiled and validated by the build script.
const COMPOSE_SPIRV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compose.comp.spv"));

/// Bindings of the one descriptor set, as `shaders/compose.comp` declares them.
mod binding {
    pub const BITMAPS: u32 = 0;
    pub const FRAME: u32 = 1;
    pub const OPS: u32 = 2;
    pub const TILE_HEADERS: u32 = 3;
    pub const TILE_OPS: u32 = 4;
}

/// Bytes of the push constants: logical width, logical height, tile grid width.
const PUSH_CONSTANT_BYTES: u32 = 12;

/// A composited frame: RGBA8 in row order, `size.width * size.height` pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub size: Size,
    pub rgba: Vec<u8>,
}

mod present;
#[cfg(test)]
mod test_compute;

use present::Presenter;
pub use present::letterbox;

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
    /// The frame target, in the `GENERAL` layout, sized for the last packet that got as far as
    /// drawing. It stays on the device; `read_frame` copies it out as a separate step.
    target: Option<GpuImage>,
    /// Whether the target holds a completely rendered frame.
    has_frame: bool,
    set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    descriptor_pool: vk::DescriptorPool,
    descriptors: vk::DescriptorSet,
    /// Everything that scales the frame into a swapchain image or an off-screen target. Made
    /// the first time it is needed, so a compositor that only renders never has it.
    presenter: Option<Presenter>,
    /// Weight of the newest frame against the held frame when a frame is shown.
    mix: f32,
    /// The mask that turns the mix into a masked one, when a transition has one.
    mask: Option<present::Mask>,
    /// Size of the window in pixels, as the owner last reported it.
    surface_size: Size,
    /// Size of the target that [`scale_frame`](VkCompositor::scale_frame) last drew into.
    last_scaled: Option<Size>,
    /// Failure injection: the number of per-submission buffers that may still be allocated
    /// before one allocation fails, once.
    #[cfg(test)]
    temporary_buffer_failure: Option<u32>,
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

/// Everything the shader reads for one frame.
struct FrameData {
    size: Size,
    records: Vec<OpRecord>,
    tiles: TileLists,
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
    fn finish(gpu: Gpu) -> Result<VkCompositor, VkError> {
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
            target: None,
            has_frame: false,
            set_layout: vk::DescriptorSetLayout::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            descriptors: vk::DescriptorSet::null(),
            presenter: None,
            mix: 1.0,
            mask: None,
            surface_size: Size::new(0, 0),
            last_scaled: None,
            #[cfg(test)]
            temporary_buffer_failure: None,
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

        // One set: the bitmap array is partially bound and updated after bind; the frame
        // target and the three per-frame buffers are plain bindings.
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
        ];
        let mut binding_flags = [vk::DescriptorBindingFlags::empty(); 5];
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
        // matches the pipeline layout; the module is not needed once the pipeline exists.
        let pipelines = unsafe {
            let pipelines =
                device.create_compute_pipelines(vk::PipelineCache::null(), &pipeline_info, None);
            device.destroy_shader_module(module, None);
            pipelines
        };
        self.pipeline = pipelines.map_err(|(_, result)| VkError::Vulkan {
            call: "vkCreateComputePipelines",
            result,
        })?[0];

        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(BITMAP_SLOTS),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(3),
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

    /// Largest width or height of an image on the device.
    #[cfg(test)]
    pub(crate) fn max_image_dimension(&self) -> u32 {
        self.gpu.max_image_dimension
    }

    /// Handles one message. A `Frame` message is rendered and yields `true`; every other
    /// message yields `false`. Freeze, transition, audio, modal and title messages are outside
    /// what this compositor models and have no effect on later frames.
    pub fn handle(&mut self, message: &GuestToHost) -> Result<bool, VkError> {
        match message {
            GuestToHost::Frame(packet) => self.render(packet).map(|()| true),
            _ => Ok(false),
        }
    }

    /// Applies the packet's bitmap deltas, then draws its ops into the frame target.
    ///
    /// Either a complete frame is rendered or an error is returned and the frame target keeps
    /// the previous frame. Bitmap state after an error is that of the reference compositor:
    /// the deltas before an invalid delta stay applied and the ones after it do not, and an
    /// invalid op leaves every delta applied.
    ///
    /// A [`VkError::Limit`] is returned for a packet that is valid but beyond the device or
    /// this backend. A limit met by a delta behaves like an invalid delta; a limit met by the
    /// frame as a whole leaves every delta applied.
    pub fn render(&mut self, packet: &FramePacket) -> Result<(), VkError> {
        check_logical_size(packet.logical_size)?;
        let mut pending = Pending::default();
        let prepared = self
            .queue_deltas(&packet.bitmaps, &mut pending)
            .and_then(|()| self.prepare_frame(packet));
        match prepared {
            Ok(frame) => self.execute(pending, Some(&frame)),
            Err(error) => {
                // A failure of the device outranks an invalid packet.
                self.execute(pending, None)?;
                Err(error)
            }
        }
    }

    /// The most recently rendered frame, copied from the device, or `None` before the first
    /// successful [`render`](VkCompositor::render).
    pub fn read_frame(&mut self) -> Result<Option<Frame>, VkError> {
        let Some(target) = self.target.as_ref().filter(|_| self.has_frame) else {
            return Ok(None);
        };
        let (image, size) = (target.image, Size::new(target.width, target.height));
        let rgba = self.read_image(image, (size.width, size.height), vk::ImageLayout::GENERAL)?;
        Ok(Some(Frame { size, rgba }))
    }

    /// Makes one allocation of a per-submission buffer (the bitmap staging buffer, then the
    /// three storage buffers of a frame) fail, after `successes` more of them have succeeded.
    #[cfg(test)]
    pub(crate) fn fail_temporary_buffer_after(&mut self, successes: u32) {
        self.temporary_buffer_failure = Some(successes);
    }

    /// Applies bitmap deltas in order and carries out their GPU work, without drawing.
    #[cfg(test)]
    pub(crate) fn apply_bitmaps(&mut self, deltas: &[BitmapDelta]) -> Result<(), VkError> {
        let mut pending = Pending::default();
        let checked = self.queue_deltas(deltas, &mut pending);
        self.execute(pending, None)?;
        checked
    }

    /// Checks the ops against the bitmaps as they are after the packet's deltas, and builds
    /// everything the shader reads. Nothing on the device is touched.
    fn prepare_frame(&self, packet: &FramePacket) -> Result<FrameData, VkError> {
        let size = packet.logical_size;
        let records = encode_ops(&self.table, size, &packet.ops)?;

        // From here on the packet is valid; what remains are limits.
        let longest = size.width.max(size.height);
        if longest > self.gpu.max_image_dimension {
            return Err(VkError::Limit {
                limit: Limit::FrameDimension,
                requested: u64::from(longest),
                maximum: u64::from(self.gpu.max_image_dimension),
            });
        }
        let max_bytes = u64::from(self.gpu.max_storage_buffer_range);
        let data_limit = |requested: u64| VkError::Limit {
            limit: Limit::FrameDataBytes,
            requested,
            maximum: max_bytes,
        };
        let record_bytes = records.len() as u64 * RECORD_WORDS as u64 * 4;
        if record_bytes > max_bytes {
            return Err(data_limit(record_bytes));
        }
        let clips: Vec<ClipRect> = records.iter().map(|record| record.clip).collect();
        let tiles = bin(size, &clips, max_bytes / 4).map_err(|entries| data_limit(entries * 4))?;
        Ok(FrameData {
            size,
            records,
            tiles,
        })
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

    /// Carries out queued bitmap work and, when there is one, the frame: one submission,
    /// waited for, then the release of everything the work was the last user of. That release
    /// happens on every path, so a failure leaks nothing.
    fn execute(&mut self, pending: Pending<'_>, frame: Option<&FrameData>) -> Result<(), VkError> {
        self.write_bitmap_descriptors(&pending.touched);
        // Buffers that live for this submission only.
        let mut temporary = Vec::new();
        let result = self.submit(&pending, frame, &mut temporary);
        if result.is_err() {
            // SAFETY: the device is live. Whatever was submitted has to finish before the
            // objects below are destroyed; if this fails too the device is lost.
            let _ = unsafe { self.gpu.device.device_wait_idle() };
        }
        for buffer in temporary {
            buffer.destroy(&self.gpu.device, &mut self.allocator);
        }
        for image in pending.retired {
            image.destroy(&self.gpu.device, &mut self.allocator);
        }
        result
    }

    /// Points the bitmap array at the current image of every slot whose occupant changed.
    ///
    /// A slot left without an image keeps its old descriptor. The binding is partially bound
    /// and no record refers to such a slot, so that descriptor is never used.
    fn write_bitmap_descriptors(&self, touched: &BTreeSet<u32>) {
        let infos: Vec<(u32, [vk::DescriptorImageInfo; 1])> = touched
            .iter()
            .filter_map(|slot| {
                let image = self.images.get(slot)?;
                let info = vk::DescriptorImageInfo::default()
                    .image_view(image.view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
                Some((*slot, [info]))
            })
            .collect();
        let writes: Vec<vk::WriteDescriptorSet<'_>> = infos
            .iter()
            .map(|(slot, info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(self.descriptors)
                    .dst_binding(binding::BITMAPS)
                    .dst_array_element(*slot)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(info)
            })
            .collect();
        if writes.is_empty() {
            return;
        }
        // SAFETY: the set and the views are live, every slot is below the array length, and
        // no command buffer that uses the set is pending.
        unsafe { self.gpu.device.update_descriptor_sets(&writes, &[]) };
    }

    /// Creates a host-visible buffer that lives for the current submission. It is handed to
    /// `temporary` at once, so `execute` releases it whatever fails afterwards.
    fn temporary_buffer<'t>(
        &mut self,
        temporary: &'t mut Vec<GpuBuffer>,
        name: &'static str,
        size: u64,
        usage: vk::BufferUsageFlags,
    ) -> Result<&'t mut GpuBuffer, VkError> {
        #[cfg(test)]
        match self.temporary_buffer_failure {
            Some(0) => {
                self.temporary_buffer_failure = None;
                return Err(VkError::Memory {
                    what: name,
                    message: "injected failure".to_owned(),
                });
            }
            Some(remaining) => self.temporary_buffer_failure = Some(remaining - 1),
            None => {}
        }
        temporary.push(GpuBuffer::new(
            &self.gpu.device,
            &mut self.allocator,
            name,
            size,
            usage,
            MemoryLocation::CpuToGpu,
        )?);
        Ok(temporary.last_mut().expect("just pushed"))
    }

    /// A storage buffer for this submission holding `words`, or one zero word when there are
    /// none, because a buffer cannot be empty.
    fn storage_buffer<'w>(
        &mut self,
        temporary: &mut Vec<GpuBuffer>,
        name: &'static str,
        word_count: usize,
        words: impl Iterator<Item = &'w [u32]>,
    ) -> Result<vk::Buffer, VkError> {
        let buffer = self.temporary_buffer(
            temporary,
            name,
            word_count.max(1) as u64 * 4,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )?;
        let bytes = buffer.bytes_mut();
        bytes[..4].fill(0);
        let mut offset = 0;
        for chunk in words {
            write_words(&mut bytes[offset..], chunk);
            offset += chunk.len() * 4;
        }
        Ok(buffer.buffer)
    }

    /// Makes everything a frame needs besides its commands: the three buffers the shader
    /// reads, their descriptors and the frame target. Returns whether the target is a new
    /// image, which is then in the `UNDEFINED` layout.
    fn prepare_frame_resources(
        &mut self,
        frame: &FrameData,
        temporary: &mut Vec<GpuBuffer>,
    ) -> Result<bool, VkError> {
        let buffers = [
            (
                binding::OPS,
                self.storage_buffer(
                    temporary,
                    "op records",
                    frame.records.len() * RECORD_WORDS,
                    frame.records.iter().map(|record| &record.words()[..]),
                )?,
            ),
            (
                binding::TILE_HEADERS,
                self.storage_buffer(
                    temporary,
                    "tile headers",
                    frame.tiles.headers.len() * 2,
                    frame.tiles.headers.iter().map(|header| &header[..]),
                )?,
            ),
            (
                binding::TILE_OPS,
                self.storage_buffer(
                    temporary,
                    "tile op indices",
                    frame.tiles.indices.len(),
                    std::iter::once(&frame.tiles.indices[..]),
                )?,
            ),
        ];
        let infos = buffers.map(|(_, buffer)| {
            [vk::DescriptorBufferInfo::default()
                .buffer(buffer)
                .range(vk::WHOLE_SIZE)]
        });
        let writes: Vec<vk::WriteDescriptorSet<'_>> = buffers
            .iter()
            .zip(&infos)
            .map(|((binding, _), info)| {
                vk::WriteDescriptorSet::default()
                    .dst_set(self.descriptors)
                    .dst_binding(*binding)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(info)
            })
            .collect();
        // SAFETY: the set and the buffers are live and no command buffer is pending.
        unsafe { self.gpu.device.update_descriptor_sets(&writes, &[]) };
        self.ensure_target(frame.size)
    }

    /// Makes the frame target `size` pixels large. Returns whether it is a new image, which is
    /// then in the `UNDEFINED` layout.
    fn ensure_target(&mut self, size: Size) -> Result<bool, VkError> {
        if let Some(target) = &self.target
            && (target.width, target.height) == (size.width, size.height)
        {
            return Ok(false);
        }
        self.has_frame = false;
        if let Some(old) = self.target.take() {
            // Nothing is pending, so the old target is idle.
            old.destroy(&self.gpu.device, &mut self.allocator);
        }
        let target = GpuImage::new(
            &self.gpu.device,
            &mut self.allocator,
            "frame target",
            (size.width, size.height),
            // Sampled for the presentation pass to come, which will view the image as
            // `R8G8B8A8_UNORM`; that view is what the mutable format is for.
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::SAMPLED,
            vk::ImageCreateFlags::MUTABLE_FORMAT,
        )?;
        let info = [vk::DescriptorImageInfo::default()
            .image_view(target.view)
            .image_layout(vk::ImageLayout::GENERAL)];
        let write = vk::WriteDescriptorSet::default()
            .dst_set(self.descriptors)
            .dst_binding(binding::FRAME)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&info);
        // SAFETY: the set and the new view are live and no command buffer is pending.
        unsafe { self.gpu.device.update_descriptor_sets(&[write], &[]) };
        self.target = Some(target);
        Ok(true)
    }

    /// Records and submits the bitmap steps and, when everything it needs could be made, the
    /// frame.
    ///
    /// The table already counts the packet's deltas as applied, so the bitmap steps are
    /// submitted even when a buffer cannot be allocated: every image ends up in the layout the
    /// rest of the backend expects. Without the staging buffer the uploads copy nothing and
    /// those bitmaps keep their previous pixels; without a frame buffer or the target there is
    /// no draw. Either way the allocation error is what the caller gets.
    fn submit(
        &mut self,
        pending: &Pending<'_>,
        frame: Option<&FrameData>,
        temporary: &mut Vec<GpuBuffer>,
    ) -> Result<(), VkError> {
        let mut failure = None;

        let mut staging_buffer = vk::Buffer::null();
        if pending.staging_len > 0 {
            let staging = self.temporary_buffer(
                temporary,
                "bitmap staging",
                pending.staging_len,
                vk::BufferUsageFlags::TRANSFER_SRC,
            );
            match staging {
                Ok(buffer) => {
                    let bytes = buffer.bytes_mut();
                    for step in &pending.steps {
                        if let Step::Upload { offset, rgba, .. } = step {
                            let start = *offset as usize;
                            bytes[start..start + rgba.len()].copy_from_slice(rgba);
                        }
                    }
                    staging_buffer = buffer.buffer;
                }
                Err(error) => failure = Some(error),
            }
        }

        // The frame, with whether its target is new. Made before recording starts.
        let mut draw = None;
        if let (Some(frame), None) = (frame, &failure) {
            match self.prepare_frame_resources(frame, temporary) {
                Ok(new_target) => draw = Some((frame, new_target)),
                Err(error) => failure = Some(error),
            }
        }
        if pending.steps.is_empty() && draw.is_none() {
            return failure.map_or(Ok(()), Err);
        }

        self.begin_commands()?;
        let (device, commands) = (&self.gpu.device, self.commands);
        for step in &pending.steps {
            // SAFETY: the command buffer is recording. Every image named by a step is alive
            // until `execute` has waited for this submission: live ones are in `self.images`
            // and destroyed ones in `pending.retired`. A new image starts in `UNDEFINED`; any
            // other is in `SHADER_READ_ONLY_OPTIMAL`, where the previous step or submission
            // left it. Upload rectangles were checked against the image size and their bytes
            // lie inside the staging buffer, which is only null when its allocation failed, in
            // which case no copy is recorded.
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
                        if staging_buffer != vk::Buffer::null() {
                            device.cmd_copy_buffer_to_image(
                                commands,
                                staging_buffer,
                                *image,
                                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                                &[region],
                            );
                        }
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

        if let Some((frame, new_target)) = draw {
            let target = self
                .target
                .as_ref()
                .expect("made with the frame resources")
                .image;
            let mut constants = [0; PUSH_CONSTANT_BYTES as usize];
            write_words(
                &mut constants,
                &[frame.size.width, frame.size.height, frame.tiles.tiles_x],
            );
            // SAFETY: the command buffer is recording. The pipeline, its layout and the set
            // belong to this device and match each other. Every descriptor the shader can use
            // is valid: the three buffers and the target were written by
            // `prepare_frame_resources`, and a quad record
            // only names a slot whose image is live and, after the steps above, in
            // `SHADER_READ_ONLY_OPTIMAL`. The buffers hold what the tile headers and indices
            // address, the target has the size in the push constants, and the tile grid is
            // within the dispatch limits every device has.
            unsafe {
                if new_target {
                    transition(
                        device,
                        commands,
                        target,
                        vk::ImageLayout::UNDEFINED,
                        vk::ImageLayout::GENERAL,
                    );
                }
                device.cmd_bind_pipeline(commands, vk::PipelineBindPoint::COMPUTE, self.pipeline);
                device.cmd_bind_descriptor_sets(
                    commands,
                    vk::PipelineBindPoint::COMPUTE,
                    self.pipeline_layout,
                    0,
                    &[self.descriptors],
                    &[],
                );
                device.cmd_push_constants(
                    commands,
                    self.pipeline_layout,
                    vk::ShaderStageFlags::COMPUTE,
                    0,
                    &constants,
                );
                device.cmd_dispatch(commands, frame.tiles.tiles_x, frame.tiles.tiles_y, 1);
            }
        }
        if draw.is_some() {
            // Until the dispatch has completed the target holds no whole frame.
            self.has_frame = false;
        }
        self.submit_and_wait()?;
        if draw.is_some() {
            self.has_frame = true;
        }
        failure.map_or(Ok(()), Err)
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
            // The caller reads the buffer through its mapping once the fence has signalled.
            make_host_readable(
                device,
                commands,
                buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::AccessFlags::TRANSFER_WRITE,
            );
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
            if let Some(presenter) = self.presenter.take() {
                presenter.destroy(device, &mut self.allocator);
            }
            for (_, image) in self.images.drain() {
                image.destroy(device, &mut self.allocator);
            }
            if let Some(target) = self.target.take() {
                target.destroy(device, &mut self.allocator);
            }
            // Frees the descriptor set with it.
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.set_layout, None);
            device.destroy_fence(self.fence, None);
            // Frees the command buffer with it.
            device.destroy_command_pool(self.pool, None);
            ManuallyDrop::drop(&mut self.allocator);
        }
    }
}
