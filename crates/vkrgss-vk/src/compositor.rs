//! The compositor: owns the device and everything created from it.

use std::collections::{BTreeSet, HashMap};
use std::mem::ManuallyDrop;
use std::time::Duration;

use ash::vk;
use gpu_allocator::vulkan::Allocator;
use vkrgss_proto::{Rect, Size};

use crate::FrameTiming;
use crate::binning::TileLists;
use crate::bitmaps::BitmapTable;
use crate::device::Gpu;
use crate::records::Records;
use crate::resources::{GpuBuffer, GpuImage};
use crate::tables::TableStore;

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

/// Bytes of the push constants: logical width, logical height, tile grid width, brightness.
const PUSH_CONSTANT_BYTES: u32 = 16;

/// A composited frame: RGBA8 in row order, `size.width * size.height` pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub size: Size,
    pub rgba: Vec<u8>,
}

mod buffers;
mod measure;
mod pacer;
mod present;
mod probe;
mod render;
mod setup;
mod submit;
#[cfg(test)]
mod test_compute;

use present::Presenter;
pub use present::{Filter, Mix, OverlayImage, letterbox};

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
    /// What the tests record of rendering and inject into it.
    probe: probe::Probe,
    /// What the tests record of presentation and inject into it.
    present_probe: present::Probe,
    allocator: ManuallyDrop<Allocator>,
    pool: vk::CommandPool,
    commands: vk::CommandBuffer,
    /// The timeline semaphore every submission signals, each with the next value, and that
    /// the waits for submissions wait on.
    timeline: vk::Semaphore,
    /// The value the last submission signals on `timeline`, 0 before the first.
    submissions: u64,
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
    /// The brightness of the packet.
    brightness: u8,
    records: Records,
    tiles: TileLists,
    /// How long encoding and binning took, when frames are measured.
    encode: Option<Duration>,
    bin: Option<Duration>,
}

enum Step<'a> {
    /// Fill a new image with transparent black.
    Clear { image: vk::Image },
    /// Replace the pixels inside `rect` with `rgba`, staged at `offset`. `whole` says that
    /// `rect` covers the whole image.
    Upload {
        image: vk::Image,
        rect: Rect,
        whole: bool,
        offset: u64,
        rgba: &'a [u8],
    },
}
