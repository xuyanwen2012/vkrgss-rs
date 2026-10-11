//! The compositor: owns the device and everything created from it.

use std::collections::{BTreeSet, HashMap};
use std::mem::ManuallyDrop;
use std::time::{Duration, Instant};

use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use vkxp_proto::{BitmapDelta, FramePacket, GuestToHost, Rect, Size, TableDelta};

use crate::binning::{TileLists, bin};
use crate::bitmaps::{BitmapTable, Change};
use crate::device::{BITMAP_SLOTS, Gpu, SurfaceSource};
use crate::records::{
    ClipRect, RECORD_WORDS, Records, TILEMAP_WORDS, check_logical_size, encode_frame,
};
use crate::resources::{
    COLOR_LAYERS, COLOR_RANGE, GpuBuffer, GpuImage, ImageUse, create_shader_module,
    make_host_readable, transition, write_words,
};
use crate::tables::TableStore;
use crate::{DeviceChoice, DeviceInfo, FrameTiming, Limit, ValidationLog, VkError};

/// The compositing shader, compiled and validated by the build script.
const COMPOSE_SPIRV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compose.comp.spv"));

/// Bindings of the one descriptor set, as `shaders/compose.comp` declares them.
mod binding {
    pub(super) const BITMAPS: u32 = 0;
    pub(super) const FRAME: u32 = 1;
    pub(super) const OPS: u32 = 2;
    pub(super) const TILE_HEADERS: u32 = 3;
    pub(super) const TILE_OPS: u32 = 4;
    pub(super) const TILEMAPS: u32 = 5;
    pub(super) const TABLES: u32 = 6;
}

/// The buffers that carry a frame to the device, one of each, kept from frame to frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameBuffer {
    /// The bytes of the bitmap uploads.
    Staging,
    Ops,
    TileHeaders,
    TileOps,
    Tilemaps,
}

impl FrameBuffer {
    const COUNT: usize = 5;

    fn name(self) -> &'static str {
        match self {
            FrameBuffer::Staging => "bitmap staging",
            FrameBuffer::Ops => "op records",
            FrameBuffer::TileHeaders => "tile headers",
            FrameBuffer::TileOps => "tile op indices",
            FrameBuffer::Tilemaps => "tilemap records",
        }
    }

    /// The binding the shader reads the buffer through, or `None` for the staging buffer,
    /// which only copies read.
    fn binding(self) -> Option<u32> {
        match self {
            FrameBuffer::Staging => None,
            FrameBuffer::Ops => Some(binding::OPS),
            FrameBuffer::TileHeaders => Some(binding::TILE_HEADERS),
            FrameBuffer::TileOps => Some(binding::TILE_OPS),
            FrameBuffer::Tilemaps => Some(binding::TILEMAPS),
        }
    }
}

/// The timestamp queries of the one query pool: a start and an end for each device duration.
mod query {
    pub(super) const COMPOSE: u32 = 0;
    pub(super) const PRESENT: u32 = 2;
    pub(super) const COUNT: u32 = 4;
}

/// Bytes of the push constants: logical width, logical height, tile grid width.
const PUSH_CONSTANT_BYTES: u32 = 12;

/// A composited frame: RGBA8 in row order, `size.width * size.height` pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub size: Size,
    pub rgba: Vec<u8>,
}

mod pacer;
mod present;
#[cfg(test)]
mod test_compute;

use present::Presenter;
pub use present::{Filter, Mix, letterbox};

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
    /// The tables the guest has announced.
    tables: TableStore,
    /// The buffer that holds every table for the shader, with the revision of the store it
    /// was written from. Made for the first frame and anew when a table has changed since.
    table_buffer: Option<(GpuBuffer, u64)>,
    /// The buffers of a frame by [`FrameBuffer`], each with its size in bytes. One is made
    /// when a frame first needs it and replaced by a larger one when a frame needs more room;
    /// the descriptor of a storage buffer names the whole of it.
    frame_buffers: [Option<(GpuBuffer, u64)>; FrameBuffer::COUNT],
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
    /// What is shown of the held and the newest frame when a frame is shown.
    mix: Mix,
    /// The filter that scales a frame when it is shown.
    filter: Filter,
    /// The mask that turns the mix into a masked one, when a transition has one.
    mask: Option<present::Mask>,
    /// Size of the window in pixels, as the owner last reported it.
    surface_size: Size,
    /// Size of the target that [`scale_frame`](VkCompositor::scale_frame) last drew into.
    last_scaled: Option<Size>,
    /// The target times of presents and the statistics of the intervals achieved.
    pacer: pacer::Pacer,
    /// Whether a frame has been rendered that no present has shown yet.
    unshown_frame: bool,
    /// Whether the last acquire of a swapchain image gave none in time. Until one succeeds
    /// the acquires do not wait.
    starved: bool,
    /// Whether frames are measured.
    timing_on: bool,
    /// What was measured of the newest frame and has not been taken yet.
    frame_timing: Option<FrameTiming>,
    /// The timestamp queries of the device durations, or null on a queue without timestamps.
    query_pool: vk::QueryPool,
    /// Failure injection: the number of acquires that still report a timeout.
    #[cfg(test)]
    acquire_timeouts: u32,
    /// Makes swapchains present as on a device without swapchain maintenance.
    #[cfg(test)]
    ignore_maintenance: bool,
    /// Makes swapchains take an sRGB format of the surface when it has one.
    #[cfg(test)]
    force_srgb: bool,
    /// Failure injection: whether the draw of the next present fails.
    #[cfg(test)]
    fail_draw: bool,
    /// The number of presents handed over with a present fence.
    #[cfg(test)]
    fenced_presents: u32,
    /// The number of times the device was idled to replace a swapchain.
    #[cfg(test)]
    swapchain_idles: u32,
    /// Stands in for the pacing extensions of the swapchain when the presents are planned.
    #[cfg(test)]
    timing_stand_in: Option<Box<dyn pacer::TimingCalls>>,
    /// Failure injection: the number of frame buffers that may still be created or grown
    /// before one such allocation fails, once.
    #[cfg(test)]
    frame_buffer_failure: Option<u32>,
    /// The number of frame buffers created so far, first ones and larger ones alike.
    #[cfg(test)]
    frame_buffers_created: u32,
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
    /// Total bytes of all uploads, which is what they fill of the staging buffer.
    staging_len: u64,
    /// Slots whose occupant changed, so their descriptor has to be rewritten.
    touched: BTreeSet<u32>,
    /// Images of destroyed bitmaps, freed once the packet's work has completed.
    retired: Vec<GpuImage>,
}

/// Everything the shader reads for one frame.
struct FrameData {
    size: Size,
    records: Records,
    tiles: TileLists,
    /// How long encoding and binning took, when frames are measured.
    encode: Option<Duration>,
    bin: Option<Duration>,
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
            tables: TableStore::new(),
            table_buffer: None,
            frame_buffers: Default::default(),
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
            #[cfg(test)]
            acquire_timeouts: 0,
            #[cfg(test)]
            ignore_maintenance: false,
            #[cfg(test)]
            force_srgb: false,
            #[cfg(test)]
            fail_draw: false,
            #[cfg(test)]
            fenced_presents: 0,
            #[cfg(test)]
            swapchain_idles: 0,
            #[cfg(test)]
            timing_stand_in: None,
            #[cfg(test)]
            frame_buffer_failure: None,
            #[cfg(test)]
            frame_buffers_created: 0,
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

    /// Turns the measuring of frames on or off. It starts off, and then no clock is read and
    /// no query is recorded. While it is on, each [`render`](VkCompositor::render) starts a
    /// [`FrameTiming`] and fills in its stages, and a [`present`](VkCompositor::present)
    /// that shows something adds its own; [`take_frame_timing`] hands the value out.
    /// Measuring changes no frame.
    ///
    /// [`take_frame_timing`]: VkCompositor::take_frame_timing
    pub fn set_frame_timing(&mut self, on: bool) {
        self.timing_on = on;
        if !on {
            self.frame_timing = None;
        }
    }

    /// What was measured since the last call: the stages of the newest frame rendered, with
    /// those of the presents since. `None` when nothing was measured, as always while the
    /// measuring is off, and after a render that failed.
    pub fn take_frame_timing(&mut self) -> Option<FrameTiming> {
        self.frame_timing.take()
    }

    /// The clock reading a measured stage starts at, or `None` while frames are not measured.
    fn stage_start(&self) -> Option<Instant> {
        self.timing_on.then(Instant::now)
    }

    /// Whether device durations are measured: frames are, and the queue has timestamps.
    fn times_device(&self) -> bool {
        self.timing_on && self.query_pool != vk::QueryPool::null()
    }

    /// Records the start of the device duration whose queries begin at `first`, when device
    /// durations are measured.
    ///
    /// # Safety
    ///
    /// The command buffer must be recording, outside a render pass.
    unsafe fn record_span_start(&self, first: u32) {
        if self.times_device() {
            let (device, commands) = (&self.gpu.device, self.commands);
            // SAFETY: the pool has timestamp queries `first` and `first + 1`, which no
            // pending command buffer uses since every submission is waited for, and the
            // queue has timestamps; the rest is guaranteed by the caller.
            unsafe {
                device.cmd_reset_query_pool(commands, self.query_pool, first, 2);
                device.cmd_write_timestamp2(
                    commands,
                    vk::PipelineStageFlags2::ALL_COMMANDS,
                    self.query_pool,
                    first,
                );
            }
        }
    }

    /// Records the end of the device duration whose queries begin at `first`.
    ///
    /// # Safety
    ///
    /// The command buffer must be recording, outside a render pass, after a
    /// [`record_span_start`](VkCompositor::record_span_start) of the same queries in it.
    unsafe fn record_span_end(&self, first: u32) {
        if self.times_device() {
            // SAFETY: as in `record_span_start`, which reset the query.
            unsafe {
                self.gpu.device.cmd_write_timestamp2(
                    self.commands,
                    vk::PipelineStageFlags2::ALL_COMMANDS,
                    self.query_pool,
                    first + 1,
                );
            }
        }
    }

    /// The device duration whose queries begin at `first`, once the submission that wrote
    /// them has been waited for. `None` when device durations are not measured or the
    /// queries cannot be read.
    fn read_span(&self, first: u32) -> Option<Duration> {
        let timestamps = self.gpu.timestamps.filter(|_| self.times_device())?;
        let mut ticks = [0u64; 2];
        // SAFETY: the pool is live and has the two queries, which the submission just waited
        // for wrote; the results are 64-bit values, as the flag says.
        unsafe {
            self.gpu.device.get_query_pool_results(
                self.query_pool,
                first,
                &mut ticks,
                vk::QueryResultFlags::TYPE_64,
            )
        }
        .ok()?;
        Some(Duration::from_nanos(timestamps.nanos(ticks[0], ticks[1])))
    }

    /// Whether the device was created with `VK_KHR_swapchain_mutable_format`.
    #[cfg(test)]
    pub(crate) fn has_swapchain_mutable_format(&self) -> bool {
        self.gpu.swapchain_mutable_format
    }

    /// Whether the queue of the device has timestamps.
    #[cfg(test)]
    pub(crate) fn has_timestamps(&self) -> bool {
        self.gpu.timestamps.is_some()
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

    /// Applies the packet's bitmap deltas and then its table deltas, then draws its ops into
    /// the frame target.
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
        // What an earlier frame left is not this frame's, also when this one is refused.
        self.frame_timing = None;
        check_logical_size(packet.logical_size)?;
        let mut pending = Pending::default();
        let prepared = self
            .queue_deltas(&packet.bitmaps, &mut pending)
            .and_then(|()| self.apply_table_deltas(&packet.tables))
            .and_then(|()| self.prepare_frame(packet));
        match prepared {
            Ok(frame) => {
                if self.timing_on {
                    self.frame_timing = Some(FrameTiming {
                        encode: frame.encode,
                        bin: frame.bin,
                        ..FrameTiming::default()
                    });
                }
                let executed = self.execute(pending, Some(&frame));
                if executed.is_err() {
                    self.frame_timing = None;
                }
                executed?;
                self.unshown_frame = true;
                Ok(())
            }
            Err(error) => {
                // A failure of the device outranks an invalid packet.
                self.execute(pending, None)?;
                Err(error)
            }
        }
    }

    /// Forgets every bitmap and table, as the deltas that destroy each of them would. The
    /// frame target and what the presenter holds stay. For a guest that starts sending anew:
    /// the next packet may create any bitmap and set any table.
    pub fn forget_resources(&mut self) -> Result<(), VkError> {
        let bitmaps: Vec<BitmapDelta> = self
            .table
            .ids()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|id| BitmapDelta::Destroy { id })
            .collect();
        let tables: Vec<TableDelta> = self
            .tables
            .ids()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|id| TableDelta::Destroy { id })
            .collect();
        let mut pending = Pending::default();
        let checked = self
            .queue_deltas(&bitmaps, &mut pending)
            .and_then(|()| self.apply_table_deltas(&tables));
        self.execute(pending, None)?;
        checked
    }

    /// The most recently rendered frame, copied from the device, or `None` before the first
    /// successful [`render`](VkCompositor::render).
    pub fn read_frame(&mut self) -> Result<Option<Frame>, VkError> {
        let Some(target) = self.target.as_ref().filter(|_| self.has_frame) else {
            return Ok(None);
        };
        let (image, size) = (target.image, Size::new(target.width, target.height));
        let rgba = self.read_image(image, (size.width, size.height), self.frame_use())?;
        Ok(Some(Frame { size, rgba }))
    }

    /// Makes one creation of a frame buffer (the bitmap staging buffer, then the four storage
    /// buffers of a frame), a first one or a larger one, fail, after `successes` more of them
    /// have succeeded.
    #[cfg(test)]
    pub(crate) fn fail_frame_buffer_after(&mut self, successes: u32) {
        self.frame_buffer_failure = Some(successes);
    }

    /// The number of frame buffers created so far.
    #[cfg(test)]
    pub(crate) fn frame_buffers_created(&self) -> u32 {
        self.frame_buffers_created
    }

    /// Applies bitmap deltas in order and carries out their GPU work, without drawing.
    #[cfg(test)]
    pub(crate) fn apply_bitmaps(&mut self, deltas: &[BitmapDelta]) -> Result<(), VkError> {
        let mut pending = Pending::default();
        let checked = self.queue_deltas(deltas, &mut pending);
        self.execute(pending, None)?;
        checked
    }

    /// Checks each table delta and applies it to the store. The buffer follows when the next
    /// frame is drawn.
    fn apply_table_deltas(&mut self, deltas: &[TableDelta]) -> Result<(), VkError> {
        for (index, delta) in deltas.iter().enumerate() {
            self.tables
                .apply(delta)
                .map_err(|source| VkError::InvalidTableDelta { index, source })?;
        }
        Ok(())
    }

    /// Checks the ops against the bitmaps and tables as they are after the packet's deltas, and builds
    /// everything the shader reads. Nothing on the device is touched.
    fn prepare_frame(&self, packet: &FramePacket) -> Result<FrameData, VkError> {
        let size = packet.logical_size;
        let started = self.stage_start();
        let records = encode_frame(&self.table, &self.tables, size, &packet.ops)?;
        let encode = started.map(|started| started.elapsed());

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
        let table_bytes = self.tables.words() * 4;
        if table_bytes > max_bytes {
            return Err(VkError::Limit {
                limit: Limit::TableBytes,
                requested: table_bytes,
                maximum: max_bytes,
            });
        }
        let record_bytes = records.ops.len() as u64 * RECORD_WORDS as u64 * 4;
        let tilemap_bytes = records.tilemaps.len() as u64 * TILEMAP_WORDS as u64 * 4;
        for bytes in [record_bytes, tilemap_bytes] {
            if bytes > max_bytes {
                return Err(data_limit(bytes));
            }
        }
        let started = self.stage_start();
        let clips: Vec<ClipRect> = records.ops.iter().map(|record| record.clip).collect();
        let tiles = bin(size, &clips, max_bytes / 4).map_err(|entries| data_limit(entries * 4))?;
        let bin = started.map(|started| started.elapsed());
        Ok(FrameData {
            size,
            records,
            tiles,
            encode,
            bin,
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
                            self.gpu.name_object(image.image, &format!("bitmap {slot}"));
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
        let result = self.submit(&pending, frame);
        if result.is_err() {
            // SAFETY: the device is live. Whatever was submitted has to finish before the
            // objects below are destroyed and the frame buffers are written again; if this
            // fails too the device is lost.
            let _ = unsafe { self.gpu.device.device_wait_idle() };
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

    /// The frame buffer of `role` with room for `size` bytes, which is not 0: the one kept
    /// when it is large enough, else a new one of at least twice the size of the one it
    /// replaces, as far as the device lets the shader read a buffer. Nothing is pending, so
    /// the buffer it replaces is idle and is destroyed at once. When the new one cannot be
    /// had the old one stays.
    fn frame_buffer(&mut self, role: FrameBuffer, size: u64) -> Result<&mut GpuBuffer, VkError> {
        let slot = role as usize;
        let kept = self.frame_buffers[slot].as_ref().map(|(_, bytes)| *bytes);
        if kept.is_none_or(|bytes| bytes < size) {
            #[cfg(test)]
            match self.frame_buffer_failure {
                Some(0) => {
                    self.frame_buffer_failure = None;
                    return Err(VkError::Memory {
                        what: role.name(),
                        message: "injected failure".to_owned(),
                    });
                }
                Some(remaining) => self.frame_buffer_failure = Some(remaining - 1),
                None => {}
            }
            // The descriptor of a storage buffer names all of it, so it stays within the
            // range the device allows, which `size` was checked against with the frame.
            let (usage, limit) = match role.binding() {
                Some(_) => (
                    vk::BufferUsageFlags::STORAGE_BUFFER,
                    u64::from(self.gpu.max_storage_buffer_range),
                ),
                None => (vk::BufferUsageFlags::TRANSFER_SRC, u64::MAX),
            };
            let doubled = kept.map_or(0, |bytes| bytes.saturating_mul(2).min(limit));
            let bytes = size.max(doubled);
            let buffer = GpuBuffer::new(
                &self.gpu.device,
                &mut self.allocator,
                role.name(),
                bytes,
                usage,
                MemoryLocation::CpuToGpu,
            )?;
            self.gpu.name_object(buffer.buffer, role.name());
            if let Some(binding) = role.binding() {
                self.write_buffer_descriptor(binding, buffer.buffer);
            }
            if let Some((old, _)) = self.frame_buffers[slot].replace((buffer, bytes)) {
                old.destroy(&self.gpu.device, &mut self.allocator);
            }
            #[cfg(test)]
            {
                self.frame_buffers_created += 1;
            }
        }
        let (buffer, _) = self.frame_buffers[slot].as_mut().expect("just ensured");
        Ok(buffer)
    }

    /// Points a storage buffer binding at the whole of `buffer`.
    fn write_buffer_descriptor(&self, binding: u32, buffer: vk::Buffer) {
        let info = [vk::DescriptorBufferInfo::default()
            .buffer(buffer)
            .range(vk::WHOLE_SIZE)];
        let write = vk::WriteDescriptorSet::default()
            .dst_set(self.descriptors)
            .dst_binding(binding)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&info);
        // SAFETY: the set and the buffer are live, the buffer is a storage buffer no larger
        // than the range the device allows, and no command buffer is pending.
        unsafe { self.gpu.device.update_descriptor_sets(&[write], &[]) };
    }

    /// Writes `words` to the start of the storage buffer of `role`, or one zero word when
    /// there are none, because a buffer cannot be empty. What the buffer holds beyond them is
    /// left from earlier frames; the shader reads only as far as the counts it is given.
    fn fill_storage_buffer<'w>(
        &mut self,
        role: FrameBuffer,
        word_count: usize,
        words: impl Iterator<Item = &'w [u32]>,
    ) -> Result<(), VkError> {
        let buffer = self.frame_buffer(role, word_count.max(1) as u64 * 4)?;
        let bytes = buffer.bytes_mut();
        bytes[..4].fill(0);
        let mut offset = 0;
        for chunk in words {
            write_words(&mut bytes[offset..], chunk);
            offset += chunk.len() * 4;
        }
        Ok(())
    }

    /// Makes everything a frame needs besides its commands: what the shader reads of the
    /// four frame buffers and the table buffer, and the frame target. Returns whether the
    /// target is a new image, which is then in the `UNDEFINED` layout.
    fn prepare_frame_resources(&mut self, frame: &FrameData) -> Result<bool, VkError> {
        self.fill_storage_buffer(
            FrameBuffer::Ops,
            frame.records.ops.len() * RECORD_WORDS,
            frame.records.ops.iter().map(|record| &record.words()[..]),
        )?;
        self.fill_storage_buffer(
            FrameBuffer::TileHeaders,
            frame.tiles.headers.len() * 2,
            frame.tiles.headers.iter().map(|header| &header[..]),
        )?;
        self.fill_storage_buffer(
            FrameBuffer::TileOps,
            frame.tiles.indices.len(),
            std::iter::once(&frame.tiles.indices[..]),
        )?;
        self.fill_storage_buffer(
            FrameBuffer::Tilemaps,
            frame.records.tilemaps.len() * TILEMAP_WORDS,
            frame.records.tilemaps.iter().map(|record| &record[..]),
        )?;
        self.ensure_table_buffer()?;
        self.ensure_target(frame.size)
    }

    /// Makes the table buffer hold every table as the store has them now: it is made anew,
    /// and its descriptor written, when a table changed since it was made. Nothing is
    /// pending, so the buffer it replaces is idle.
    fn ensure_table_buffer(&mut self) -> Result<(), VkError> {
        let revision = self.tables.revision();
        if let Some((_, written)) = &self.table_buffer
            && *written == revision
        {
            return Ok(());
        }
        // One zero word when there is no element, because a buffer cannot be empty. The
        // length was checked against the device's limit with the frame.
        let mut buffer = GpuBuffer::new(
            &self.gpu.device,
            &mut self.allocator,
            "tables",
            self.tables.words().max(1) * 4,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::CpuToGpu,
        )?;
        let bytes = buffer.bytes_mut();
        bytes[..4].fill(0);
        let words = bytes.as_chunks_mut::<4>().0.iter_mut();
        for (target, word) in words.zip(self.tables.buffer_words()) {
            *target = word.to_ne_bytes();
        }
        self.gpu.name_object(buffer.buffer, "tables");
        self.write_buffer_descriptor(binding::TABLES, buffer.buffer);
        if let Some((old, _)) = self.table_buffer.replace((buffer, revision)) {
            old.destroy(&self.gpu.device, &mut self.allocator);
        }
        Ok(())
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
            // The presentation set still points at the view that is gone, and the new one
            // may get its handle.
            if let Some(presenter) = self.presenter.as_mut() {
                presenter.forget_bound();
            }
        }
        let target = GpuImage::new(
            &self.gpu.device,
            &mut self.allocator,
            "frame target",
            (size.width, size.height),
            // Sampled by the presentation pass.
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::SAMPLED,
            vk::ImageCreateFlags::empty(),
        )?;
        self.gpu.name_object(target.image, "frame target");
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
    /// rest of the backend expects. Without a staging buffer that holds the uploads they copy
    /// nothing and those bitmaps keep their previous pixels; without a frame buffer or the
    /// target there is no draw. Either way the allocation error is what the caller gets.
    fn submit(&mut self, pending: &Pending<'_>, frame: Option<&FrameData>) -> Result<(), VkError> {
        let mut failure = None;
        let started = self.stage_start();

        let mut staging_buffer = vk::Buffer::null();
        if pending.staging_len > 0 {
            match self.frame_buffer(FrameBuffer::Staging, pending.staging_len) {
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
            match self.prepare_frame_resources(frame) {
                Ok(new_target) => draw = Some((frame, new_target)),
                Err(error) => failure = Some(error),
            }
        }
        if pending.steps.is_empty() && draw.is_none() {
            return failure.map_or(Ok(()), Err);
        }

        self.begin_commands()?;
        let (bitmap_use, frame_use) = (self.bitmap_use(), self.frame_use());
        let (device, commands) = (&self.gpu.device, self.commands);
        if !pending.steps.is_empty() {
            // SAFETY: the command buffer is recording.
            unsafe { self.gpu.begin_label(commands, c"upload") };
        }
        for step in &pending.steps {
            // SAFETY: the command buffer is recording. Every image named by a step is alive
            // until `execute` has waited for this submission: live ones are in `self.images`
            // and destroyed ones in `pending.retired`. A new image starts in `UNDEFINED`; any
            // other is in `SHADER_READ_ONLY_OPTIMAL`, where the previous step or submission
            // left it for its readers. Upload rectangles were checked against the image size and their bytes
            // lie inside the staging buffer, which is only null when one large enough could not
            // be had, in which case no copy is recorded.
            unsafe {
                match step {
                    Step::Clear { image } => {
                        transition(
                            device,
                            commands,
                            *image,
                            ImageUse::UNDEFINED,
                            ImageUse::TRANSFER_WRITE,
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
                            bitmap_use,
                            ImageUse::TRANSFER_WRITE,
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
                    ImageUse::TRANSFER_WRITE,
                    bitmap_use,
                );
            }
        }
        if !pending.steps.is_empty() {
            // SAFETY: the command buffer is recording and the label above is open in it.
            unsafe { self.gpu.end_label(commands) };
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
            // is valid: `prepare_frame_resources` saw to the five buffers and the target, each
            // written when it was made, and a quad or tilemap record
            // only names a slot whose image is live and, after the steps above, in
            // `SHADER_READ_ONLY_OPTIMAL`. The buffers hold what the tile headers and indices
            // address, the table buffer holds every element a tilemap record's offsets and
            // sizes reach, the target has the size in the push constants, and the tile grid is
            // within the dispatch limits every device has. The target is new or in `GENERAL`,
            // where the dispatch before left it for its readers. The label and the timestamps
            // are recorded outside any render pass, each end after its start.
            unsafe {
                self.gpu.begin_label(commands, c"compose");
                let before = match new_target {
                    true => ImageUse::UNDEFINED,
                    false => frame_use,
                };
                transition(
                    device,
                    commands,
                    target,
                    before,
                    ImageUse::COMPUTE_STORAGE_WRITE,
                );
                self.record_span_start(query::COMPOSE);
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
                self.record_span_end(query::COMPOSE);
                // Hands the composed frame to the presentation pass and the copies.
                transition(
                    device,
                    commands,
                    target,
                    ImageUse::COMPUTE_STORAGE_WRITE,
                    frame_use,
                );
                self.gpu.end_label(commands);
            }
        }
        if draw.is_some() {
            // Until the dispatch has completed the target holds no whole frame.
            self.has_frame = false;
        }
        let recorded = self.stage_start();
        self.submit_and_wait()?;
        if draw.is_some() {
            self.has_frame = true;
            let compose_device = self.read_span(query::COMPOSE);
            if let (Some(timing), Some(started), Some(recorded)) =
                (&mut self.frame_timing, started, recorded)
            {
                timing.upload = Some(recorded.duration_since(started));
                timing.submit = Some(recorded.elapsed());
                timing.compose_device = compose_device;
            }
        }
        failure.map_or(Ok(()), Err)
    }

    /// How the image of a bitmap is used between its uploads: the compute dispatch fetches
    /// from it, and so does the fragment shader of the presentation pass while the bitmap is a
    /// transition mask, which only a queue that can draw gets to.
    fn bitmap_use(&self) -> ImageUse {
        match self.gpu.can_draw {
            true => ImageUse::COMPUTE_SAMPLED_READ.and(ImageUse::FRAGMENT_SAMPLED_READ),
            false => ImageUse::COMPUTE_SAMPLED_READ,
        }
    }

    /// How the frame target is used between dispatches, in the `GENERAL` layout it stays in:
    /// copies read it (read-back, freeze), and so does the fragment shader of the
    /// presentation pass on a queue that can draw.
    fn frame_use(&self) -> ImageUse {
        let readers = match self.gpu.can_draw {
            true => ImageUse::TRANSFER_READ.and(ImageUse::FRAGMENT_SAMPLED_READ),
            false => ImageUse::TRANSFER_READ,
        };
        readers.in_layout(vk::ImageLayout::GENERAL)
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
    /// `resting` is the use the image is in, which is restored; an image whose use already
    /// lets copies read it, in `GENERAL` or `TRANSFER_SRC_OPTIMAL`, is copied from as it is.
    fn read_image(
        &mut self,
        image: vk::Image,
        (width, height): (u32, u32),
        resting: ImageUse,
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
        let result = self.copy_image_to_buffer(image, (width, height), resting, buffer.buffer);
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
        resting: ImageUse,
        buffer: vk::Buffer,
    ) -> Result<(), VkError> {
        self.begin_commands()?;
        let (device, commands) = (&self.gpu.device, self.commands);
        let copying = match resting.layout {
            vk::ImageLayout::GENERAL | vk::ImageLayout::TRANSFER_SRC_OPTIMAL => resting,
            _ => ImageUse::TRANSFER_READ,
        };
        let region = vk::BufferImageCopy::default()
            .image_subresource(COLOR_LAYERS)
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });
        // SAFETY: the command buffer is recording; the caller passes a live image of this size
        // in the use `resting`, created with transfer-source usage, and a live buffer that
        // holds the whole image; both outlive the wait in `submit_and_wait`.
        unsafe {
            if copying != resting {
                transition(device, commands, image, resting, copying);
            }
            device.cmd_copy_image_to_buffer(commands, image, copying.layout, buffer, &[region]);
            // The caller reads the buffer through its mapping once the fence has signalled.
            make_host_readable(
                device,
                commands,
                buffer,
                vk::PipelineStageFlags2::ALL_TRANSFER,
                vk::AccessFlags2::TRANSFER_WRITE,
            );
            if copying != resting {
                transition(device, commands, image, copying, resting);
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
            Some(image) => {
                self.read_image(image.image, (image.width, image.height), self.bitmap_use())
            }
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
            device.destroy_fence(self.fence, None);
            device.destroy_query_pool(self.query_pool, None);
            // Frees the command buffer with it.
            device.destroy_command_pool(self.pool, None);
            ManuallyDrop::drop(&mut self.allocator);
        }
    }
}
