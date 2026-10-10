//! Presentation: scaling the frame into a swapchain image or an off-screen target.
//!
//! One graphics pipeline draws a triangle over the whole target. `shaders/present.frag` puts
//! the frame inside the letterbox rectangle computed here and black outside it, mixing the
//! newest frame with the held one on the way, by one weight or per pixel under the mask of a
//! transition. The same draw goes to a swapchain image
//! ([`VkCompositor::present`]) or to an image that is read back
//! ([`VkCompositor::scale_frame`]), which is how scaling and mixing are tested without a
//! window.
//!
//! Like the rest of the compositor this is synchronous: each call submits its work and waits
//! for it before it returns or hands the image to the presentation engine.

use ash::vk;
use gpu_allocator::vulkan::Allocator;
use vkxp_proto::{BitmapId, Rect, Size};

use super::pacer::{SwapchainTiming, TimingCalls, surface_pacing};
use super::{Frame, VkCompositor, query};
use crate::device::Gpu;
use crate::pacing::IntervalSummary;
use crate::present_timing as ffi;
use crate::resources::{
    COLOR_LAYERS, COLOR_RANGE, GpuImage, ImageUse, create_shader_module, transition, write_words,
};
use crate::{Limit, VkError};

const VERTEX_SPIRV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/present.vert.spv"));
const FRAGMENT_SPIRV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/present.frag.spv"));

/// Bindings of the presenter's descriptor set, as `shaders/present.frag` declares them.
mod binding {
    pub(super) const NEWEST: u32 = 0;
    pub(super) const HELD: u32 = 1;
    pub(super) const MASK: u32 = 2;
}

/// Bytes of the push constants: for the newest and then the held frame the letterbox origin
/// and size and the frame size, then the mix weight, the flags, the threshold and the
/// vagueness, four bytes per component.
const PUSH_CONSTANT_BYTES: usize = 64;

/// Flags of the fragment shader, as `shaders/present.frag` defines them.
const FLAG_NEAREST: u32 = 1;
const FLAG_SRGB_TARGET: u32 = 2;
const FLAG_HELD_NEAREST: u32 = 4;
const FLAG_SAME_GRID: u32 = 8;
const FLAG_MASK: u32 = 16;
const FLAG_LINEAR: u32 = 32;
const FLAG_MIX: u32 = 64;

/// What is shown of the held and the newest frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mix {
    /// The newest frame alone: no transition is active.
    Newest,
    /// One step of a transition, with the values of both of its forms.
    Step {
        /// The weight `an` of the newest frame in a cross-fade: per channel the result is
        /// `(old * (256 - an) + new * an) >> 8`.
        weight: u8,
        /// The threshold `T` of a masked transition: the mask level up to which the newest
        /// frame is shown alone.
        threshold: u8,
    },
}

impl Mix {
    /// The held frame alone, where there is no mask.
    pub const HELD: Mix = Mix::Step {
        weight: 0,
        threshold: 0,
    };
}

/// The largest vagueness the shader's arithmetic is given. `Graphics.transition` passes none
/// above it, and the ones above 255 already behave alike.
const MAX_VAGUENESS: u32 = 65535;

/// Format of the off-screen target. Its values are what a `UNORM` swapchain image would hold.
const OFFSCREEN_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;

/// How long an acquire waits for a swapchain image, in nanoseconds: four frame periods at
/// 40 frames per second. A surface that hands out none in that time, as that of a hidden
/// window may, must not stop the game.
const ACQUIRE_TIMEOUT: u64 = 100_000_000;

/// How long the replacement of a swapchain waits for the fences of its presents, in
/// nanoseconds, before it idles the device instead. A lost surface may never signal them.
const PRESENT_FENCE_TIMEOUT: u64 = ACQUIRE_TIMEOUT;

/// What the held frame and an empty target are cleared to: opaque black.
const BLACK: [u32; 4] = [0, 0, 0, 255];

/// Where a frame of `frame` pixels goes in a window of `window` pixels: scaled by one factor
/// in both directions, the largest at which it still fits, and centred.
///
/// The side that does not fill the window is rounded down to whole pixels. The rectangle is
/// empty when either size has a zero dimension, or when the window is so narrow that the
/// scaled frame has no pixel.
pub fn letterbox(frame: Size, window: Size) -> Rect {
    let (frame_w, frame_h) = (u64::from(frame.width), u64::from(frame.height));
    let (window_w, window_h) = (u64::from(window.width), u64::from(window.height));
    if frame_w == 0 || frame_h == 0 || window_w == 0 || window_h == 0 {
        return Rect::default();
    }
    // The window is at least as wide as the frame's shape: the height is what limits.
    let (width, height) = if frame_w * window_h <= frame_h * window_w {
        (frame_w * window_h / frame_h, window_h)
    } else {
        (window_w, frame_h * window_w / frame_w)
    };
    if width == 0 || height == 0 {
        return Rect::default();
    }
    Rect::new(
        ((window_w - width) / 2) as i32,
        ((window_h - height) / 2) as i32,
        width as u32,
        height as u32,
    )
}

/// How the frame is scaled into its letterbox rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    /// Nearest when the factor is a whole number, sharp bilinear otherwise.
    #[default]
    Auto,
    /// Each output pixel shows the frame pixel under its centre.
    Nearest,
    /// Each output pixel interpolates between the four nearest frame pixels.
    Linear,
    /// Nearest, except in a band one output pixel wide along each frame pixel boundary,
    /// which interpolates.
    Sharp,
}

impl Filter {
    /// Whether a frame of `frame` pixels shown in `rect` has each of its pixels shown as it
    /// is, without interpolation.
    fn is_nearest(self, frame: Size, rect: &Rect) -> bool {
        match self {
            Filter::Auto => is_whole_factor(frame, rect),
            Filter::Nearest => true,
            Filter::Linear | Filter::Sharp => false,
        }
    }
}

/// Whether `rect` shows a frame of `frame` pixels at a whole-number factor.
fn is_whole_factor(frame: Size, rect: &Rect) -> bool {
    !rect.is_empty()
        && rect.width.is_multiple_of(frame.width)
        && rect.height.is_multiple_of(frame.height)
        && rect.width / frame.width == rect.height / frame.height
}

/// The picture that orders a transition: where it is dark the newest frame appears first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Mask {
    bitmap: BitmapId,
    /// Width of the soft edge in mask levels, at most [`MAX_VAGUENESS`]. 0 is a hard edge.
    vagueness: u32,
}

/// The sizes of the two frames a draw reads. They differ when the logical size changed after
/// the held frame was captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sources {
    newest: Size,
    held: Size,
    /// The vagueness of the mask the draw reads, or `None` for a mix by the one weight.
    vagueness: Option<u32>,
}

/// The push constants that put the frames of `sources` into a target of `target` pixels, each
/// in its own letterbox rectangle, or `None` when no pixel of either would be visible.
fn push_constants(
    sources: Sources,
    target: Size,
    mix: Mix,
    srgb_target: bool,
    filter: Filter,
) -> Option<[u8; PUSH_CONSTANT_BYTES]> {
    let newest = letterbox(sources.newest, target);
    let held = letterbox(sources.held, target);
    if newest.is_empty() && held.is_empty() {
        return None;
    }
    Some(encode_constants(
        sources,
        newest,
        held,
        mix,
        srgb_target,
        filter,
    ))
}

/// The push constants that stretch frames of one size, that of the newest frame of `sources`,
/// over the whole of a target of `target` pixels, which has to be the letterbox rectangle of
/// such a frame in some window.
/// The draw then gives exactly the pixels that rectangle has in that window.
fn filling_constants(
    sources: Sources,
    target: Size,
    mix: Mix,
    filter: Filter,
) -> [u8; PUSH_CONSTANT_BYTES] {
    let sources = Sources {
        held: sources.newest,
        ..sources
    };
    let whole = Rect::from_size(target);
    encode_constants(sources, whole, whole, mix, false, filter)
}

/// The push constants for the frames of `sources` in the rectangles `newest` and `held`.
fn encode_constants(
    sources: Sources,
    newest: Rect,
    held: Rect,
    mix: Mix,
    srgb_target: bool,
    filter: Filter,
) -> [u8; PUSH_CONSTANT_BYTES] {
    let mut flags = 0;
    if filter.is_nearest(sources.newest, &newest) {
        flags |= FLAG_NEAREST;
    }
    if filter.is_nearest(sources.held, &held) {
        flags |= FLAG_HELD_NEAREST;
    }
    if filter == Filter::Linear {
        flags |= FLAG_LINEAR;
    }
    if sources.newest == sources.held {
        flags |= FLAG_SAME_GRID;
    }
    if srgb_target {
        flags |= FLAG_SRGB_TARGET;
    }
    let (weight, threshold) = match mix {
        Mix::Newest => (0, 0),
        Mix::Step { weight, threshold } => {
            flags |= FLAG_MIX;
            if sources.vagueness.is_some() {
                flags |= FLAG_MASK;
            }
            (u32::from(weight), u32::from(threshold))
        }
    };
    let vagueness = sources.vagueness.unwrap_or(0);
    let mut bytes = [0; PUSH_CONSTANT_BYTES];
    write_words(
        &mut bytes,
        &[
            newest.x as u32,
            newest.y as u32,
            newest.width,
            newest.height,
            sources.newest.width,
            sources.newest.height,
            held.x as u32,
            held.y as u32,
            held.width,
            held.height,
            sources.held.width,
            sources.held.height,
            weight,
            flags,
            threshold,
            vagueness,
        ],
    );
    bytes
}

/// How a draw places the frames in its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placement {
    /// Each frame in the letterbox rectangle of its size, as a window shows them.
    Letterbox,
    /// Both frames, which have the same size, stretched over the whole target.
    Fill,
}

/// The surface format to present in, and whether it is an sRGB format.
///
/// A `UNORM` format passes the frame's values through unchanged and is preferred. With only
/// sRGB formats on offer the shader undoes the encoding the format applies. Anything else is
/// taken as it comes.
fn choose_format(formats: &[vk::SurfaceFormatKHR]) -> Option<(vk::SurfaceFormatKHR, bool)> {
    const UNORM: [vk::Format; 3] = [
        vk::Format::B8G8R8A8_UNORM,
        vk::Format::R8G8B8A8_UNORM,
        vk::Format::A8B8G8R8_UNORM_PACK32,
    ];
    const SRGB: [vk::Format; 3] = [
        vk::Format::B8G8R8A8_SRGB,
        vk::Format::R8G8B8A8_SRGB,
        vk::Format::A8B8G8R8_SRGB_PACK32,
    ];
    let find = |wanted: &[vk::Format]| {
        wanted.iter().find_map(|format| {
            formats.iter().copied().find(|offered| {
                offered.format == *format
                    && offered.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
        })
    };
    find(&UNORM)
        .map(|format| (format, false))
        .or_else(|| find(&SRGB).map(|format| (format, true)))
        .or_else(|| formats.first().map(|format| (*format, false)))
}

/// The extent of a swapchain for a surface with `capabilities` in a window of `requested`
/// pixels. A zero dimension means nothing can be presented now.
fn swapchain_extent(capabilities: &vk::SurfaceCapabilitiesKHR, requested: Size) -> vk::Extent2D {
    // The special value says the surface takes whatever size the swapchain has.
    if capabilities.current_extent.width != u32::MAX {
        return capabilities.current_extent;
    }
    let (min, max) = (capabilities.min_image_extent, capabilities.max_image_extent);
    vk::Extent2D {
        width: requested.width.clamp(min.width, max.width.max(min.width)),
        height: requested
            .height
            .clamp(min.height, max.height.max(min.height)),
    }
}

/// The pipeline that draws into a target of one format.
struct FormatPipeline {
    format: vk::Format,
    pipeline: vk::Pipeline,
}

/// A swapchain with what is needed to draw into and present its images.
struct Swapchain {
    handle: vk::SwapchainKHR,
    extent: vk::Extent2D,
    /// The window size the swapchain was made for.
    requested: Size,
    srgb: bool,
    pipeline: vk::Pipeline,
    images: Vec<vk::Image>,
    /// Per image: the view the pass draws through.
    views: Vec<vk::ImageView>,
    /// Signalled when an image has been acquired; waited for by the draw.
    acquired: vk::Semaphore,
    /// Per image: signalled by the draw, waited for by the presentation engine.
    drawn: Vec<vk::Semaphore>,
    /// Whether the swapchain no longer matches the surface and has to be replaced.
    stale: bool,
    /// The pacing extensions on this swapchain, when it was created for them.
    timing: Option<SwapchainTiming>,
    /// With swapchain maintenance: the fences that the presents carry, one per present,
    /// each signalled once the presentation engine has let go of what its present used. A
    /// fence is used again by a later present once it is signalled, and one is added when
    /// none is. `None` on a swapchain that presents without the extension.
    present_fences: Option<Vec<PresentFence>>,
}

/// A fence for the presents of a swapchain.
struct PresentFence {
    fence: vk::Fence,
    /// Whether a present carried the fence and it has not been seen signalled since.
    outstanding: bool,
}

impl Swapchain {
    /// Waits, for a bounded time, until the presentation engine has let go of what the
    /// presents of this swapchain used. Returns whether it has, which only the fences of
    /// swapchain maintenance tell: `false` without them and when one is not signalled in
    /// time.
    fn settle_presents(&self, device: &ash::Device) -> bool {
        let Some(fences) = &self.present_fences else {
            return false;
        };
        let outstanding: Vec<vk::Fence> = fences
            .iter()
            .filter(|present| present.outstanding)
            .map(|present| present.fence)
            .collect();
        if !outstanding.is_empty() {
            // SAFETY: the fences are live and belong to this device.
            let waited =
                unsafe { device.wait_for_fences(&outstanding, true, PRESENT_FENCE_TIMEOUT) };
            if waited.is_err() {
                return false;
            }
        }
        true
    }

    /// Destroys the swapchain and everything made for it. Every submission must have been
    /// waited for, and the presentation engine must have let go of the presents: their
    /// fences were waited for, or the device is idle.
    fn destroy(self, device: &ash::Device, loader: &ash::khr::swapchain::Device) {
        // SAFETY: the caller guarantees that nothing is in use; every handle was created from
        // this device for this swapchain and is consumed here. Destroying a swapchain also
        // releases an image that was acquired and not presented.
        unsafe {
            for view in self.views {
                device.destroy_image_view(view, None);
            }
            for semaphore in self.drawn {
                device.destroy_semaphore(semaphore, None);
            }
            device.destroy_semaphore(self.acquired, None);
            loader.destroy_swapchain(self.handle, None);
            for present in self.present_fences.into_iter().flatten() {
                device.destroy_fence(present.fence, None);
            }
        }
    }
}

/// Everything that scales the frame into a target. Owned by the compositor, which destroys it
/// before the device.
pub(super) struct Presenter {
    set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptors: vk::DescriptorSet,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
    /// One pipeline per target format in use: the off-screen one and the swapchain's.
    pipelines: Vec<FormatPipeline>,
    /// The held frame, in the `SHADER_READ_ONLY_OPTIMAL` layout between submissions. It has
    /// the size of the frame it was captured from, which later frames need not share, and
    /// lives until the next capture. Black, at the size of the first frame, until a freeze.
    held: Option<GpuImage>,
    /// One texel, in the `SHADER_READ_ONLY_OPTIMAL` layout, bound in place of a mask when a
    /// draw has none.
    no_mask: Option<GpuImage>,
    /// The views of the newest frame, the held frame and the mask that the descriptor set
    /// points at.
    bound: Option<[vk::ImageView; 3]>,
    swapchain_loader: Option<ash::khr::swapchain::Device>,
    /// The commands of `VK_EXT_swapchain_maintenance1`, loaded with the first swapchain that
    /// uses them.
    maintenance_loader: Option<ash::ext::swapchain_maintenance1::Device>,
    /// The commands of the pacing extensions, loaded with the first swapchain that is
    /// created for them.
    timing_commands: Option<(ffi::Commands, ash::khr::calibrated_timestamps::Device)>,
    swapchain: Option<Swapchain>,
}

/// One draw of the frame: what to bind and the constants to push.
struct DrawCall {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptors: vk::DescriptorSet,
    constants: [u8; PUSH_CONSTANT_BYTES],
}

impl Presenter {
    fn new(device: &ash::Device, allocator: &mut Allocator) -> Result<Presenter, VkError> {
        // Handles start out null so that `destroy` can clean up after a failure part way
        // through `init`: destroying a null handle is a no-op.
        let mut presenter = Presenter {
            set_layout: vk::DescriptorSetLayout::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            descriptors: vk::DescriptorSet::null(),
            vertex: vk::ShaderModule::null(),
            fragment: vk::ShaderModule::null(),
            pipelines: Vec::new(),
            held: None,
            no_mask: None,
            bound: None,
            swapchain_loader: None,
            maintenance_loader: None,
            timing_commands: None,
            swapchain: None,
        };
        match presenter.init(device) {
            Ok(()) => Ok(presenter),
            Err(error) => {
                presenter.destroy(device, allocator);
                Err(error)
            }
        }
    }

    fn init(&mut self, device: &ash::Device) -> Result<(), VkError> {
        let bindings = [binding::NEWEST, binding::HELD, binding::MASK].map(|binding| {
            vk::DescriptorSetLayoutBinding::default()
                .binding(binding)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
        });
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        // SAFETY: the device is live and the description is valid for the call.
        self.set_layout = unsafe { device.create_descriptor_set_layout(&layout_info, None) }
            .map_err(VkError::vulkan("vkCreateDescriptorSetLayout"))?;

        let set_layouts = [self.set_layout];
        let push_ranges = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .size(PUSH_CONSTANT_BYTES as u32)];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_ranges);
        // SAFETY: the set layout was just created from this device.
        self.pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None) }
                .map_err(VkError::vulkan("vkCreatePipelineLayout"))?;

        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(3)];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
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

        self.vertex = create_shader_module(device, VERTEX_SPIRV)?;
        self.fragment = create_shader_module(device, FRAGMENT_SPIRV)?;
        Ok(())
    }

    /// The pipeline that draws into a target of `format`, made on first use.
    fn pipeline(&mut self, gpu: &Gpu, format: vk::Format) -> Result<vk::Pipeline, VkError> {
        let device = &gpu.device;
        let known = self.pipelines.iter().find(|known| known.format == format);
        if let Some(known) = known {
            return Ok(known.pipeline);
        }

        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(self.vertex)
                .name(c"main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(self.fragment)
                .name(c"main"),
        ];
        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
        let assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
        let viewport = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);
        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let blend_attachments = [vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)];
        let blend =
            vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend_attachments);
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
        let formats = [format];
        let mut rendering =
            vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&formats);
        let pipeline_info = [vk::GraphicsPipelineCreateInfo::default()
            .push_next(&mut rendering)
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&assembly)
            .viewport_state(&viewport)
            .rasterization_state(&rasterization)
            .multisample_state(&multisample)
            .color_blend_state(&blend)
            .dynamic_state(&dynamic)
            .layout(self.pipeline_layout)];
        // SAFETY: the modules hold a vertex and a fragment shader with entry point `main`
        // whose interfaces match each other, the pipeline layout and the one colour attachment
        // of `format` that the pipeline is made to render into without a render pass object.
        let pipelines = unsafe {
            device.create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_info, None)
        };
        let pipeline = pipelines.map_err(|(_, result)| VkError::Vulkan {
            call: "vkCreateGraphicsPipelines",
            result,
        })?[0];
        gpu.name_object(pipeline, &format!("present pipeline {format:?}"));
        self.pipelines.push(FormatPipeline { format, pipeline });
        Ok(pipeline)
    }

    /// Points the descriptor set at the views of the newest frame, the held frame and the
    /// mask, which is a bitmap's view or that of the stand-in.
    fn bind(
        &mut self,
        device: &ash::Device,
        newest: vk::ImageView,
        held: vk::ImageView,
        mask: vk::ImageView,
    ) {
        // A bitmap's view is written every time: the bitmap may have been destroyed since,
        // and a new view can have the handle of the old one.
        let stand_in = self.no_mask.as_ref().map(|image| image.view);
        if self.bound == Some([newest, held, mask]) && stand_in == Some(mask) {
            return;
        }
        let infos = [
            (binding::NEWEST, newest, vk::ImageLayout::GENERAL),
            (
                binding::HELD,
                held,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ),
            (
                binding::MASK,
                mask,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ),
        ]
        .map(|(binding, view, layout)| {
            let info = vk::DescriptorImageInfo::default()
                .image_view(view)
                .image_layout(layout);
            (binding, [info])
        });
        let writes = infos.each_ref().map(|(binding, info)| {
            vk::WriteDescriptorSet::default()
                .dst_set(self.descriptors)
                .dst_binding(*binding)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(info)
        });
        // SAFETY: the set and the views are live and no command buffer that uses the set is
        // pending.
        unsafe { device.update_descriptor_sets(&writes, &[]) };
        self.bound = Some([newest, held, mask]);
    }

    /// Destroys everything. The device must be idle.
    pub(super) fn destroy(mut self, device: &ash::Device, allocator: &mut Allocator) {
        if let (Some(swapchain), Some(loader)) = (self.swapchain.take(), &self.swapchain_loader) {
            // The device is idle either way; this only gives the fences the time to signal.
            swapchain.settle_presents(device);
            swapchain.destroy(device, loader);
        }
        if let Some(held) = self.held.take() {
            held.destroy(device, allocator);
        }
        if let Some(no_mask) = self.no_mask.take() {
            no_mask.destroy(device, allocator);
        }
        // SAFETY: the caller guarantees that nothing is in use; each handle was created from
        // this device or is null, and none is used again.
        unsafe {
            for known in self.pipelines.drain(..) {
                device.destroy_pipeline(known.pipeline, None);
            }
            device.destroy_shader_module(self.fragment, None);
            device.destroy_shader_module(self.vertex, None);
            // Frees the descriptor set with it.
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.set_layout, None);
        }
    }
}

/// What one pass draws into.
struct PassTarget {
    image: vk::Image,
    /// A view of the whole image, in the format the pipeline was made for.
    view: vk::ImageView,
    extent: vk::Extent2D,
    /// How the image was last used. Its contents are not kept.
    before: ImageUse,
    /// The use the image is left to after the pass.
    after: ImageUse,
}

/// Records one pass over the whole target: a clear to black, then the frame when there is one
/// to draw, between the barriers that make the target the colour attachment and hand it on.
///
/// # Safety
///
/// `commands` must be a command buffer of `device` in the recording state, of a queue family
/// that draws. The image and the view of `target` must be live, belong to each other, be
/// `extent` pixels large and in the layout of `before`. For a draw, the pipeline must have
/// been made for the format of the view, and the images behind the descriptor set must be
/// live and in the layouts the set names, left there for the fragment stage to read, when the
/// commands execute.
unsafe fn record_pass(
    device: &ash::Device,
    commands: vk::CommandBuffer,
    target: &PassTarget,
    draw: Option<&DrawCall>,
) {
    let extent = target.extent;
    let area = vk::Rect2D {
        offset: vk::Offset2D::default(),
        extent,
    };
    let attachments = [vk::RenderingAttachmentInfo::default()
        .image_view(target.view)
        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .clear_value(vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        })];
    let rendering = vk::RenderingInfo::default()
        .render_area(area)
        .layer_count(1)
        .color_attachments(&attachments);
    // SAFETY: guaranteed by the caller.
    unsafe {
        transition(
            device,
            commands,
            target.image,
            target.before,
            ImageUse::COLOR_ATTACHMENT_WRITE,
        );
        device.cmd_begin_rendering(commands, &rendering);
        if let Some(draw) = draw {
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(commands, 0, &[viewport]);
            device.cmd_set_scissor(commands, 0, &[area]);
            device.cmd_bind_pipeline(commands, vk::PipelineBindPoint::GRAPHICS, draw.pipeline);
            device.cmd_bind_descriptor_sets(
                commands,
                vk::PipelineBindPoint::GRAPHICS,
                draw.pipeline_layout,
                0,
                &[draw.descriptors],
                &[],
            );
            device.cmd_push_constants(
                commands,
                draw.pipeline_layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                &draw.constants,
            );
            device.cmd_draw(commands, 3, 1, 0, 0);
        }
        device.cmd_end_rendering(commands);
        transition(
            device,
            commands,
            target.image,
            ImageUse::COLOR_ATTACHMENT_WRITE,
            target.after,
        );
    }
}

impl VkCompositor {
    /// Sets what is shown of the held and the newest frame from now on: the newest frame
    /// alone, or a step of a transition, mixed per channel on the frames' 8-bit values before
    /// scaling. It starts as [`Mix::Newest`].
    pub fn set_mix(&mut self, mix: Mix) {
        self.mix = mix;
    }

    /// What is shown of the held and the newest frame.
    pub fn mix(&self) -> Mix {
        self.mix
    }

    /// Sets the filter that scales everything shown from now on. It starts as
    /// [`Filter::Auto`].
    pub fn set_filter(&mut self, filter: Filter) {
        self.filter = filter;
    }

    /// The current scaling filter.
    pub fn filter(&self) -> Filter {
        self.filter
    }

    /// Makes the mix of a transition step from now on a masked one, or with `None` a
    /// cross-fade again. Under a mask each pixel has its own weight: with `L` the luminance
    /// of the mask there, `T` the step's threshold and `v` the vagueness, the held frame
    /// weighs `w = 0` for `L <= T` and `min(255, (L - T) * 255 / v)` above it, or for a
    /// vagueness of 0 `255` for `L >= T` and 0 below, and the result is the newest frame for
    /// `w = 0` and `new + (((old - new) * (w + 1)) >> 8)` otherwise. So the newest frame
    /// appears first where the mask is dark, behind an edge that is the softer the larger the
    /// vagueness. A mask smaller than the newest frame is tiled from the frame's corner.
    ///
    /// The bitmap is looked up each time something is shown, so it may be created by a packet
    /// rendered after this call. While there is no such bitmap, or it has no pixels, the mix
    /// is the cross-fade.
    pub fn set_mask(&mut self, mask: Option<(BitmapId, u32)>) {
        self.mask = mask.map(|(bitmap, vagueness)| Mask {
            bitmap,
            vagueness: vagueness.min(MAX_VAGUENESS),
        });
    }

    /// Whether `id` names a live bitmap.
    pub fn has_bitmap(&self, id: BitmapId) -> bool {
        self.table.get(id).is_ok()
    }

    /// The view and the vagueness of the mask in effect: the one set, if its bitmap is live
    /// and has pixels.
    fn mask_in_effect(&self) -> Option<(vk::ImageView, u32)> {
        let mask = self.mask?;
        self.table.get(mask.bitmap).ok()?;
        let image = self.images.get(&mask.bitmap.index)?;
        Some((image.view, mask.vagueness))
    }

    /// Whether the compositor was created for a surface and can [`present`].
    ///
    /// [`present`]: VkCompositor::present
    pub fn has_surface(&self) -> bool {
        self.gpu.surface.is_some()
    }

    /// Captures what is shown now as the held frame: the mix of the held and the newest frame
    /// as [`set_mix`](VkCompositor::set_mix) has it. The mix then blends that image with the
    /// frames rendered afterwards. Outside a transition this is the newest frame as it is, at
    /// weight 0 without a mask the held frame stays what it was, and before the first frame
    /// it is black.
    ///
    /// The held frame keeps its own size and its pixels whatever is rendered afterwards. When
    /// a later packet has another logical size, the held frame is still shown as it was, in
    /// the letterbox rectangle of its own size, and a cross-fade mixes the two frames where
    /// each of them lies in the window.
    ///
    /// In the middle of a cross-fade the mixed image exists only as it was shown, and it is
    /// captured so that showing it again in the same window gives the same pixels. The window
    /// is the swapchain, or without one the target [`scale_frame`](VkCompositor::scale_frame)
    /// last drew into.
    ///
    /// - Frames of one size shown without interpolation, as at a whole-number factor with
    ///   [`Filter::Auto`]: the mix is taken at the frame's size, since each of its pixels is
    ///   shown as it is.
    /// - Frames of one size otherwise: the mix is taken at the size of its letterbox
    ///   rectangle in that window, after scaling, because mixing first and scaling the
    ///   rounded result afterwards does not give the pixels that were shown.
    /// - Frames of different sizes: where the two lie depends on the window, so the whole
    ///   window is taken, bars included.
    ///
    /// If nothing has been shown yet the mix is taken at the size of the newest frame.
    pub fn freeze(&mut self) -> Result<(), VkError> {
        self.ensure_presenter()?;
        let target = self
            .target
            .as_ref()
            .filter(|_| self.has_frame)
            .map(|target| (target.image, (target.width, target.height)));
        let Some((newest, size)) = target else {
            // Nothing has been rendered, so black is what is shown.
            let held = self.presenter.as_ref().and_then(|p| p.held.as_ref());
            return match held.map(|held| held.image) {
                Some(held) => self.submit_or_idle(|compositor| {
                    compositor.clear_held(held, ImageUse::FRAGMENT_SAMPLED_READ)
                }),
                None => Ok(()),
            };
        };
        if self.mix == Mix::Newest {
            return self.capture_held(newest, vk::ImageLayout::GENERAL, size);
        }
        if matches!(self.mix, Mix::Step { weight: 0, .. }) && self.mask_in_effect().is_none() {
            // The held frame alone is shown, and it stays, at whatever size it has.
            return Ok(());
        }
        // In between, what is shown exists nowhere yet: draw it again as it was shown and
        // take the result; see the documentation above for the three cases. The off-screen
        // target and the held frame have formats of the same size, so the copy moves the
        // bytes as they are.
        let frame = Size::new(size.0, size.1);
        let held_size = self
            .presenter
            .as_ref()
            .and_then(|presenter| presenter.held.as_ref())
            .map_or(frame, |held| Size::new(held.width, held.height));
        let (capture, placement) = match self.shown_size() {
            None => (frame, Placement::Letterbox),
            Some(shown) if held_size != frame => (shown, Placement::Letterbox),
            Some(shown) => {
                let rect = letterbox(frame, shown);
                if rect.is_empty() || self.filter.is_nearest(frame, &rect) {
                    (frame, Placement::Letterbox)
                } else {
                    (Size::new(rect.width, rect.height), Placement::Fill)
                }
            }
        };
        self.with_offscreen(capture, placement, |compositor, mixed| {
            compositor.capture_held(
                mixed,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                (capture.width, capture.height),
            )
        })
    }

    /// The size of the target the current state was last shown in: the swapchain when there
    /// is one, else the last off-screen target.
    fn shown_size(&self) -> Option<Size> {
        let chain = self
            .presenter
            .as_ref()
            .and_then(|presenter| presenter.swapchain.as_ref());
        match chain {
            Some(chain) => Some(Size::new(chain.extent.width, chain.extent.height)),
            None => self.last_scaled,
        }
    }

    /// Makes the held frame a copy of `source`, an image of `size` pixels in `source_layout`.
    /// A held frame of another size is replaced by a new image once the copy has succeeded.
    fn capture_held(
        &mut self,
        source: vk::Image,
        source_layout: vk::ImageLayout,
        size: (u32, u32),
    ) -> Result<(), VkError> {
        let existing = self
            .presenter
            .as_ref()
            .and_then(|presenter| presenter.held.as_ref())
            .filter(|held| (held.width, held.height) == size)
            .map(|held| held.image);
        if let Some(held) = existing {
            return self.submit_or_idle(|compositor| {
                compositor.copy_to_held(
                    source,
                    source_layout,
                    held,
                    ImageUse::FRAGMENT_SAMPLED_READ,
                    size,
                )
            });
        }
        let fresh = self.new_held_image(size, "held frame")?;
        let image = fresh.image;
        let copied = self.submit_or_idle(|compositor| {
            compositor.copy_to_held(source, source_layout, image, ImageUse::UNDEFINED, size)
        });
        match copied {
            Ok(()) => {
                let presenter = self.presenter.as_mut().expect("made by the caller");
                presenter.bound = None;
                if let Some(old) = presenter.held.replace(fresh) {
                    // Every submission that read it was waited for.
                    old.destroy(&self.gpu.device, &mut self.allocator);
                }
                Ok(())
            }
            Err(error) => {
                fresh.destroy(&self.gpu.device, &mut self.allocator);
                Err(error)
            }
        }
    }

    /// A new image for the held frame, or for the texel that stands in for a mask, with the
    /// name it has in a capture.
    fn new_held_image(
        &mut self,
        size: (u32, u32),
        name: &'static str,
    ) -> Result<GpuImage, VkError> {
        let image = GpuImage::new(
            &self.gpu.device,
            &mut self.allocator,
            name,
            size,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::ImageCreateFlags::empty(),
        )?;
        self.gpu.name_object(image.image, name);
        Ok(image)
    }

    /// Draws what [`present`](VkCompositor::present) would show in a window of `size` pixels
    /// into an off-screen target and reads it back: the frame scaled by the largest factor
    /// that fits, centred, black around it, mixed with the held frame as the current mix says.
    /// Before the first frame the result is black. Alpha is 255 everywhere.
    ///
    /// The target is an `R8G8B8A8_UNORM` image, so the bytes are what a `UNORM` swapchain
    /// image would hold. Works with and without a surface.
    pub fn scale_frame(&mut self, size: Size) -> Result<Frame, VkError> {
        if size.width == 0 || size.height == 0 {
            return Ok(Frame {
                size,
                rgba: Vec::new(),
            });
        }
        let longest = size.width.max(size.height);
        if longest > self.gpu.max_image_dimension {
            return Err(VkError::Limit {
                limit: Limit::FrameDimension,
                requested: u64::from(longest),
                maximum: u64::from(self.gpu.max_image_dimension),
            });
        }
        self.last_scaled = Some(size);
        let rgba = self.with_offscreen(size, Placement::Letterbox, |compositor, image| {
            compositor.read_image(image, (size.width, size.height), ImageUse::TRANSFER_READ)
        })?;
        Ok(Frame { size, rgba })
    }

    /// Draws what a window of `size` pixels would show into a new off-screen target, or with
    /// [`Placement::Fill`] the frames stretched over all of it, and hands its image, in the
    /// `TRANSFER_SRC_OPTIMAL` layout, to `then`. The target is destroyed afterwards, so `then`
    /// has to wait for whatever it submits. `size` has no zero dimension and is within the
    /// device's limit.
    fn with_offscreen<T>(
        &mut self,
        size: Size,
        placement: Placement,
        then: impl FnOnce(&mut VkCompositor, vk::Image) -> Result<T, VkError>,
    ) -> Result<T, VkError> {
        let source = self.prepare_sources()?;
        let device = &self.gpu.device;
        let presenter = self.presenter.as_mut().expect("made with the sources");
        let pipeline = presenter.pipeline(&self.gpu, OFFSCREEN_FORMAT)?;
        let draw = source.and_then(|sources| {
            Some(DrawCall {
                pipeline,
                pipeline_layout: presenter.pipeline_layout,
                descriptors: presenter.descriptors,
                constants: match placement {
                    Placement::Letterbox => {
                        push_constants(sources, size, self.mix, false, self.filter)?
                    }
                    Placement::Fill => filling_constants(sources, size, self.mix, self.filter),
                },
            })
        });

        let extent = vk::Extent2D {
            width: size.width,
            height: size.height,
        };
        let image = GpuImage::with_format(
            device,
            &mut self.allocator,
            "off-screen target",
            (size.width, size.height),
            OFFSCREEN_FORMAT,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
            vk::ImageCreateFlags::empty(),
        )?;
        let target = PassTarget {
            image: image.image,
            view: image.view,
            extent,
            before: ImageUse::UNDEFINED,
            after: ImageUse::TRANSFER_READ,
        };

        let drawn = self.submit_or_idle(|compositor| {
            compositor.begin_commands()?;
            // SAFETY: the command buffer is recording, on a queue that draws since there is a
            // presenter. The image and its view were made above for `extent` and the format
            // of the pipeline, are new, and live until this submission has been waited for. A
            // draw call only exists when a frame does, in which case `prepare_sources`
            // pointed the set at the frame target (`GENERAL`), the held frame and the mask
            // (`SHADER_READ_ONLY_OPTIMAL`), each left to the fragment stage by its last
            // barrier.
            unsafe {
                compositor.gpu.begin_label(compositor.commands, c"present");
                record_pass(
                    &compositor.gpu.device,
                    compositor.commands,
                    &target,
                    draw.as_ref(),
                );
                compositor.gpu.end_label(compositor.commands);
            }
            compositor.submit_and_wait()
        });
        let result = drawn.and_then(|()| then(self, image.image));
        // Every submission that used the image was waited for, or the device was idled after
        // a failure.
        image.destroy(&self.gpu.device, &mut self.allocator);
        result
    }

    /// Sets the frame rate that presents are paced at from now on: where the device and the
    /// surface support `VK_EXT_present_timing`, each present of a new frame is given a target
    /// time one period of this rate after the one before. It starts as 40. A rate of 0
    /// counts as 1.
    pub fn set_frame_rate(&mut self, rate: u32) {
        self.pacer.set_frame_rate(rate);
    }

    /// Makes the target time of the next frame start from the clock again instead of
    /// following the one before, as after anything that broke the rhythm of the frames.
    pub fn restart_pacing(&mut self) {
        self.pacer.restart();
    }

    /// The intervals between the times at which the frames presented last became visible:
    /// the newest [`STATS_INTERVALS`](crate::STATS_INTERVALS) of them, or `None` while there
    /// is none. The times are the ones the presentation engine reports where presents are
    /// paced with `VK_EXT_present_timing`, and otherwise the times at which the presents
    /// were handed over, by the compositor's own clock. A present that repeats a frame
    /// already shown is not counted.
    pub fn present_summary(&self) -> Option<IntervalSummary> {
        self.pacer.summary()
    }

    /// Whether the current swapchain's presents carry target times.
    pub fn paces_with_present_timing(&self) -> bool {
        let chain = self.presenter.as_ref().and_then(|p| p.swapchain.as_ref());
        self.pacer.wants_timing() && chain.is_some_and(|chain| chain.timing.is_some())
    }

    /// Why presents stopped carrying target times, once: a call of the extension failed.
    /// Presenting goes on without them.
    pub fn take_pacing_report(&mut self) -> Option<String> {
        self.pacer.take_report()
    }

    /// Tells the compositor the size of the window in pixels. The swapchain is made for that
    /// size at the next [`present`](VkCompositor::present), and replaced when it changes.
    /// Until a size is given, and while either dimension is 0, nothing is presented.
    pub fn resize_surface(&mut self, size: Size) {
        self.surface_size = size;
    }

    /// Shows the current state in the window: the newest frame mixed with the held frame at
    /// the current weight, scaled into the window and centred, black around it. Before the
    /// first frame the window is black.
    ///
    /// Returns whether an image was presented. Nothing is presented, and no error is raised,
    /// while the window has no pixels, or when the surface hands out no image within a tenth
    /// of a second, as that of a hidden window may. After such a wait the presents that
    /// follow do not wait at all until the surface hands out an image again, and that
    /// present shows the newest frame. A swapchain that the surface reports as out of date
    /// is replaced and the present tried once more.
    ///
    /// Where the device and the surface support it, the present carries a target time; see
    /// [`set_frame_rate`](VkCompositor::set_frame_rate).
    ///
    /// Fails with [`VkError::NoSurface`] on a compositor that was created without a surface.
    pub fn present(&mut self) -> Result<bool, VkError> {
        if self.gpu.surface.is_none() {
            return Err(VkError::NoSurface);
        }
        if self.surface_size.width == 0 || self.surface_size.height == 0 {
            return Ok(false);
        }
        let source = self.prepare_sources()?;
        // Whether this present shows a frame for the first time, which is what is paced.
        let frame = std::mem::take(&mut self.unshown_frame);
        // One more attempt after an out-of-date swapchain, and one that does not count after
        // a present the pacing extension refused, which happens once at most.
        let mut attempts = 2;
        while attempts > 0 {
            attempts -= 1;
            if !self.ensure_swapchain()? {
                return Ok(false);
            }
            let presenter = self.presenter.as_mut().expect("made with the sources");
            let loader = presenter
                .swapchain_loader
                .clone()
                .expect("made with the swapchain");
            let chain = presenter.swapchain.as_mut().expect("just ensured");
            // A surface that gave no image in time is not waited for again until it gives
            // one, so a hidden window costs one wait and not one per frame.
            let timeout = match self.starved {
                true => 0,
                false => ACQUIRE_TIMEOUT,
            };
            #[cfg(test)]
            let injected = self.acquire_timeouts > 0;
            #[cfg(not(test))]
            let injected = false;
            let acquired = if injected {
                #[cfg(test)]
                {
                    self.acquire_timeouts -= 1;
                }
                Err(vk::Result::TIMEOUT)
            } else {
                // SAFETY: the swapchain and the semaphore are live and belong to this device,
                // and the semaphore is unsignalled: the draw that waited on it last has
                // completed, an acquire that gave no image left it as it was, and a swapchain
                // whose draw failed is replaced together with its semaphores.
                unsafe {
                    loader.acquire_next_image(
                        chain.handle,
                        timeout,
                        chain.acquired,
                        vk::Fence::null(),
                    )
                }
            };
            let index = match acquired {
                Ok((index, suboptimal)) => {
                    self.starved = false;
                    chain.stale |= suboptimal;
                    index
                }
                // No image in time, or none at once. The frame is rendered and stays the one
                // to show: like a window without pixels, this present counts for nothing.
                Err(vk::Result::TIMEOUT | vk::Result::NOT_READY) => {
                    self.starved = true;
                    self.unshown_frame = frame;
                    return Ok(false);
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    chain.stale = true;
                    continue;
                }
                Err(result) => {
                    return Err(VkError::Vulkan {
                        call: "vkAcquireNextImageKHR",
                        result,
                    });
                }
            };
            // Only now that there is an image to show the frame in: a present that shows
            // nothing must not move the target times on. This may wait for the frame before
            // to be shown.
            #[cfg(test)]
            let stand_in = self.timing_stand_in.is_some();
            #[cfg(not(test))]
            let stand_in = false;
            let calls = chain
                .timing
                .as_mut()
                .map(|timing| timing as &mut dyn TimingCalls);
            #[cfg(test)]
            let calls = match &mut self.timing_stand_in {
                Some(calls) => Some(&mut **calls as &mut dyn TimingCalls),
                None => calls,
            };
            let plan = self.pacer.plan(calls, frame);
            // The target of a stand-in is on no clock of the swapchain and is not sent.
            let timing_info = plan
                .as_ref()
                .filter(|_| !stand_in)
                .zip(chain.timing.as_ref())
                .map(|(plan, timing)| timing.present_info(plan));
            let (handle, extent) = (chain.handle, chain.extent);
            // An image that comes round again was presented; what it holds is not wanted.
            let pass_target = PassTarget {
                image: chain.images[index as usize],
                view: chain.views[index as usize],
                extent,
                before: ImageUse::ACQUIRED,
                after: ImageUse::PRESENT,
            };
            let (wait, signal) = (chain.acquired, chain.drawn[index as usize]);
            let target = Size::new(extent.width, extent.height);
            let draw = source.and_then(|sources| {
                Some(DrawCall {
                    pipeline: chain.pipeline,
                    pipeline_layout: presenter.pipeline_layout,
                    descriptors: presenter.descriptors,
                    constants: push_constants(sources, target, self.mix, chain.srgb, self.filter)?,
                })
            });

            let started = self.stage_start();
            let drawn = self.submit_or_idle(|compositor| {
                #[cfg(test)]
                if std::mem::take(&mut compositor.fail_draw) {
                    return Err(VkError::Memory {
                        what: "the draw",
                        message: "injected failure".to_owned(),
                    });
                }
                compositor.begin_commands()?;
                // SAFETY: the command buffer is recording, on the queue that presents and
                // draws. The image and its view belong to the live swapchain, which is
                // `extent` pixels large and has the format of the pipeline; the image was
                // just acquired, so it may be taken from `UNDEFINED`. A draw call only exists
                // when a frame does, in which case `prepare_sources` pointed the set at the
                // frame target (`GENERAL`), the held frame and the mask
                // (`SHADER_READ_ONLY_OPTIMAL`), each left to the fragment stage by its last
                // barrier. The label and the timestamps are recorded around the pass, each
                // end after its start.
                unsafe {
                    compositor.gpu.begin_label(compositor.commands, c"present");
                    compositor.record_span_start(query::PRESENT);
                    record_pass(
                        &compositor.gpu.device,
                        compositor.commands,
                        &pass_target,
                        draw.as_ref(),
                    );
                    compositor.record_span_end(query::PRESENT);
                    compositor.gpu.end_label(compositor.commands);
                }
                compositor.submit_drawn(wait, signal)
            });
            if let (Some(started), Ok(())) = (started, &drawn) {
                let present_device = self.read_span(query::PRESENT);
                let timing = self.frame_timing.get_or_insert_default();
                timing.present = Some(started.elapsed());
                timing.present_device = present_device;
            }
            if let Err(error) = drawn {
                // The image is acquired and the two semaphores are in an unknown state. With
                // swapchain maintenance the image is given back and the semaphores are
                // replaced; otherwise a new swapchain brings new ones.
                if !self.release_undrawn(index) {
                    self.mark_swapchain_stale();
                }
                return Err(error);
            }

            let (swapchains, indices, waits) = ([handle], [index], [signal]);
            let mut info = vk::PresentInfoKHR::default()
                .wait_semaphores(&waits)
                .swapchains(&swapchains)
                .image_indices(&indices);
            // With swapchain maintenance every present carries a fence.
            let present_fences = match self.take_present_fence() {
                Ok(fence) => [fence],
                Err(error) => {
                    // The image is drawn and its semaphore signalled, and neither is used.
                    self.mark_swapchain_stale();
                    return Err(error);
                }
            };
            let mut fence_info =
                vk::SwapchainPresentFenceInfoEXT::default().fences(&present_fences);
            if present_fences[0] != vk::Fence::null() {
                info = info.push_next(&mut fence_info);
                #[cfg(test)]
                {
                    self.fenced_presents += 1;
                }
            }
            // The structures of the pacing extensions, chained in when the present is paced.
            let present_ids = [plan.and_then(|plan| plan.present_id).unwrap_or(0)];
            let mut id_info = ffi::PresentId2 {
                swapchain_count: 1,
                p_present_ids: present_ids.as_ptr(),
                ..Default::default()
            };
            let mut timings_info = ffi::PresentTimingsInfo {
                swapchain_count: 1,
                ..Default::default()
            };
            if let Some(timing_info) = &timing_info {
                timings_info.p_timing_infos = timing_info;
                timings_info.p_next = info.p_next;
                info.p_next = (&raw const timings_info).cast();
            }
            if present_ids[0] != 0 {
                id_info.p_next = info.p_next;
                info.p_next = (&raw const id_info).cast();
            }
            // SAFETY: the queue can present to the surface, which was checked when the device
            // was chosen; the image was acquired from this swapchain and drawn by a submission
            // that has completed and signalled the semaphore. The chained structures, which
            // have the layout of the header, only exist for a swapchain that was created with
            // the flags of their extensions on a device that has them enabled; they and what
            // they point to live until the call returns; the target time is in a time domain
            // of the swapchain and the present id is greater than every one before it. A
            // fence is only chained on a device with swapchain maintenance, belongs to this
            // device, is unsignalled and is carried by no other present.
            match unsafe { loader.queue_present(self.gpu.queue, &info) } {
                Ok(suboptimal) => {
                    self.pacer.presented(plan.as_ref(), frame);
                    if suboptimal {
                        self.mark_swapchain_stale();
                    }
                    return Ok(true);
                }
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.mark_swapchain_stale(),
                // The results queue of the swapchain is full. The presentation engine did
                // not take the image: its acquisition is released, and the wait on the
                // semaphore the draw signalled is queued, so the semaphore is unsignalled
                // again. The swapchain is as it was before the acquire, and the frame is
                // acquired, drawn and presented once more, now without the extension.
                Err(result) if self.pacer.present_failed(plan.as_ref(), result) => attempts += 1,
                Err(result) => {
                    self.mark_swapchain_stale();
                    return Err(VkError::Vulkan {
                        call: "vkQueuePresentKHR",
                        result,
                    });
                }
            }
        }
        Ok(false)
    }

    /// The extent of the current swapchain, if there is one.
    #[cfg(test)]
    pub(crate) fn swapchain_extent(&self) -> Option<(u32, u32)> {
        let chain = self.presenter.as_ref()?.swapchain.as_ref()?;
        Some((chain.extent.width, chain.extent.height))
    }

    /// Makes the next `count` acquires of a swapchain image report that none arrived in
    /// time, without asking the surface.
    #[cfg(test)]
    pub(crate) fn fail_acquires(&mut self, count: u32) {
        self.acquire_timeouts = count;
    }

    /// Whether the last acquire gave no image, so that the next one does not wait.
    #[cfg(test)]
    pub(crate) fn is_starved(&self) -> bool {
        self.starved
    }

    /// Makes the next present replace the swapchain, as an out-of-date report does.
    #[cfg(test)]
    pub(crate) fn invalidate_swapchain(&mut self) {
        self.mark_swapchain_stale();
    }

    /// Makes the swapchains created from now on present without swapchain maintenance, as on
    /// a device that lacks the extension.
    #[cfg(test)]
    pub(crate) fn ignore_swapchain_maintenance(&mut self) {
        self.ignore_maintenance = true;
    }

    /// The number of presents handed over with a fence so far.
    #[cfg(test)]
    pub(crate) fn fenced_presents(&self) -> u32 {
        self.fenced_presents
    }

    /// The number of times the device was idled to replace a swapchain so far.
    #[cfg(test)]
    pub(crate) fn swapchain_idles(&self) -> u32 {
        self.swapchain_idles
    }

    /// Makes the draw of the next present fail before anything is submitted, with an image
    /// acquired.
    #[cfg(test)]
    pub(crate) fn fail_next_draw(&mut self) {
        self.fail_draw = true;
    }

    /// Whether a swapchain created now uses `VK_EXT_swapchain_maintenance1`.
    pub(crate) fn uses_swapchain_maintenance(&self) -> bool {
        #[cfg(test)]
        if self.ignore_maintenance {
            return false;
        }
        self.gpu.swapchain_maintenance
    }

    /// The fence the next present carries: null on a swapchain without swapchain
    /// maintenance, else one of the swapchain's fences that no present holds, or a new one
    /// when every one is still held. The fence is unsignalled and counts as outstanding from
    /// here on, whatever the present returns: the presentation engine may have taken it even
    /// then.
    fn take_present_fence(&mut self) -> Result<vk::Fence, VkError> {
        let device = &self.gpu.device;
        let presenter = self.presenter.as_mut().expect("made with the sources");
        let chain = presenter.swapchain.as_mut().expect("presenting into it");
        let Some(fences) = &mut chain.present_fences else {
            return Ok(vk::Fence::null());
        };
        for present in fences.iter_mut() {
            if present.outstanding {
                // SAFETY: the fence is live and belongs to this device.
                let signalled = unsafe { device.get_fence_status(present.fence) }
                    .map_err(VkError::vulkan("vkGetFenceStatus"))?;
                if !signalled {
                    continue;
                }
                // SAFETY: the fence is signalled, so the present that carried it is done
                // with it.
                unsafe { device.reset_fences(&[present.fence]) }
                    .map_err(VkError::vulkan("vkResetFences"))?;
            }
            present.outstanding = true;
            return Ok(present.fence);
        }
        // SAFETY: the device is live.
        let fence = unsafe { device.create_fence(&vk::FenceCreateInfo::default(), None) }
            .map_err(VkError::vulkan("vkCreateFence"))?;
        fences.push(PresentFence {
            fence,
            outstanding: true,
        });
        Ok(fence)
    }

    /// After a draw that failed: gives the acquired image `index` back to the swapchain and
    /// replaces the two semaphores of the draw, so that the swapchain can go on. Returns
    /// `false` when that is not possible, without swapchain maintenance or because a call
    /// failed, and the swapchain has to be replaced.
    fn release_undrawn(&mut self, index: u32) -> bool {
        let device = &self.gpu.device;
        let Some(presenter) = self.presenter.as_mut() else {
            return false;
        };
        let (Some(chain), Some(loader)) =
            (presenter.swapchain.as_mut(), &presenter.maintenance_loader)
        else {
            return false;
        };
        if chain.present_fences.is_none() {
            return false;
        }
        let semaphore = || {
            // SAFETY: the device is live.
            unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }.ok()
        };
        let Some(acquired) = semaphore() else {
            return false;
        };
        let Some(drawn) = semaphore() else {
            // SAFETY: the semaphore was just created and is not used.
            unsafe { device.destroy_semaphore(acquired, None) };
            return false;
        };
        let indices = [index];
        let info = vk::ReleaseSwapchainImagesInfoEXT::default()
            .swapchain(chain.handle)
            .image_indices(&indices);
        // SAFETY: the device has swapchain maintenance and the swapchain is live. The image
        // was acquired from it and not presented, and nothing uses it: the device was idled
        // after the failure. For the same reason the old semaphores, which belong to this
        // device and are not used again, can be destroyed.
        unsafe {
            if loader.release_swapchain_images(&info).is_err() {
                device.destroy_semaphore(acquired, None);
                device.destroy_semaphore(drawn, None);
                return false;
            }
            device.destroy_semaphore(chain.acquired, None);
            device.destroy_semaphore(chain.drawn[index as usize], None);
        }
        chain.acquired = acquired;
        chain.drawn[index as usize] = drawn;
        true
    }

    fn mark_swapchain_stale(&mut self) {
        if let Some(chain) = self.presenter.as_mut().and_then(|p| p.swapchain.as_mut()) {
            chain.stale = true;
        }
    }

    /// Runs `work`, which records and submits commands, and idles the device if it fails, so
    /// that whatever was submitted is finished before the caller releases anything.
    fn submit_or_idle(
        &mut self,
        work: impl FnOnce(&mut VkCompositor) -> Result<(), VkError>,
    ) -> Result<(), VkError> {
        let result = work(self);
        if result.is_err() {
            // SAFETY: the device is live. If this fails too the device is lost.
            let _ = unsafe { self.gpu.device.device_wait_idle() };
        }
        result
    }

    fn ensure_presenter(&mut self) -> Result<(), VkError> {
        if !self.gpu.can_draw {
            return Err(VkError::CannotDraw {
                device: self.gpu.info.name.clone(),
            });
        }
        if self.presenter.is_none() {
            self.presenter = Some(Presenter::new(&self.gpu.device, &mut self.allocator)?);
        }
        Ok(())
    }

    /// Makes everything a draw reads ready: the presenter, a held frame, and the descriptor
    /// set pointing at both frames and the mask. Returns their sizes, or `None` when no frame has been
    /// rendered and there is nothing to draw.
    fn prepare_sources(&mut self) -> Result<Option<Sources>, VkError> {
        self.ensure_presenter()?;
        let target = self
            .target
            .as_ref()
            .filter(|_| self.has_frame)
            .map(|target| (target.view, (target.width, target.height)));
        let Some((newest, size)) = target else {
            return Ok(None);
        };
        self.ensure_held(size)?;
        self.ensure_no_mask()?;
        let mask = self.mask_in_effect();
        let presenter = self.presenter.as_mut().expect("just ensured");
        let held = presenter.held.as_ref().expect("just ensured");
        let (held_view, held_size) = (held.view, Size::new(held.width, held.height));
        let no_mask = presenter.no_mask.as_ref().expect("just ensured").view;
        let mask_view = mask.map_or(no_mask, |(view, _)| view);
        presenter.bind(&self.gpu.device, newest, held_view, mask_view);
        Ok(Some(Sources {
            newest: Size::new(size.0, size.1),
            held: held_size,
            vagueness: mask.map(|(_, vagueness)| vagueness),
        }))
    }

    /// Makes sure there is the texel that stands in for a mask.
    fn ensure_no_mask(&mut self) -> Result<(), VkError> {
        let presenter = self.presenter.as_ref().expect("made by the caller");
        if presenter.no_mask.is_some() {
            return Ok(());
        }
        // Made and filled like a held frame of one pixel.
        let no_mask = self.new_held_image((1, 1), "no mask")?;
        let image = no_mask.image;
        let cleared =
            self.submit_or_idle(|compositor| compositor.clear_held(image, ImageUse::UNDEFINED));
        match cleared {
            Ok(()) => {
                let presenter = self.presenter.as_mut().expect("made by the caller");
                presenter.no_mask = Some(no_mask);
                Ok(())
            }
            Err(error) => {
                no_mask.destroy(&self.gpu.device, &mut self.allocator);
                Err(error)
            }
        }
    }

    /// Makes sure there is a held frame. One that exists is left alone whatever its size; a
    /// new one is `size` pixels large and black, which is what was shown before any frame.
    fn ensure_held(&mut self, size: (u32, u32)) -> Result<(), VkError> {
        let presenter = self.presenter.as_ref().expect("made by the caller");
        if presenter.held.is_some() {
            return Ok(());
        }
        let held = self.new_held_image(size, "held frame")?;
        let image = held.image;
        let cleared =
            self.submit_or_idle(|compositor| compositor.clear_held(image, ImageUse::UNDEFINED));
        match cleared {
            Ok(()) => {
                let presenter = self.presenter.as_mut().expect("made by the caller");
                presenter.bound = None;
                presenter.held = Some(held);
                Ok(())
            }
            Err(error) => {
                held.destroy(&self.gpu.device, &mut self.allocator);
                Err(error)
            }
        }
    }

    /// Fills the held frame, which was last used as `from` says, with opaque black and leaves
    /// it to the fragment shader.
    fn clear_held(&mut self, held: vk::Image, from: ImageUse) -> Result<(), VkError> {
        self.begin_commands()?;
        let (device, commands) = (&self.gpu.device, self.commands);
        // SAFETY: the command buffer is recording; the caller passes the live held image in
        // the layout of `from`, which has transfer-destination usage and stays alive until
        // the submission has been waited for. A held image only exists on a queue that draws.
        unsafe {
            transition(device, commands, held, from, ImageUse::TRANSFER_WRITE);
            device.cmd_clear_color_image(
                commands,
                held,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &vk::ClearColorValue { uint32: BLACK },
                &[COLOR_RANGE],
            );
            transition(
                device,
                commands,
                held,
                ImageUse::TRANSFER_WRITE,
                ImageUse::FRAGMENT_SAMPLED_READ,
            );
        }
        self.submit_and_wait()
    }

    /// Copies `source`, which is in `source_layout` for copies to read, into the held frame,
    /// which was last used as `held_use` says and is left to the fragment shader in
    /// `SHADER_READ_ONLY_OPTIMAL`; both are `size` pixels large.
    fn copy_to_held(
        &mut self,
        source: vk::Image,
        source_layout: vk::ImageLayout,
        held: vk::Image,
        held_use: ImageUse,
        (width, height): (u32, u32),
    ) -> Result<(), VkError> {
        self.begin_commands()?;
        let (device, commands) = (&self.gpu.device, self.commands);
        let region = vk::ImageCopy::default()
            .src_subresource(COLOR_LAYERS)
            .dst_subresource(COLOR_LAYERS)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });
        // SAFETY: the command buffer is recording. The source is the live frame target or an
        // off-screen target, in `source_layout`, which is `GENERAL` or
        // `TRANSFER_SRC_OPTIMAL`, with transfer-source usage, and the barrier that left it
        // there named the transfer stage as a reader; the held frame is live, in the layout
        // of `held_use`, which is `SHADER_READ_ONLY_OPTIMAL` or, for a new image whose
        // contents do not matter, `UNDEFINED`, with transfer-destination usage. Every pixel
        // of it is written, so nothing undefined is left. Their formats are four
        // 8-bit channels each, which copies allow between, both are `size` pixels large, and
        // both outlive the wait below.
        unsafe {
            transition(device, commands, held, held_use, ImageUse::TRANSFER_WRITE);
            device.cmd_copy_image(
                commands,
                source,
                source_layout,
                held,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
            transition(
                device,
                commands,
                held,
                ImageUse::TRANSFER_WRITE,
                ImageUse::FRAGMENT_SAMPLED_READ,
            );
        }
        self.submit_and_wait()
    }

    /// Ends recording and submits the command buffer so that it starts drawing once the
    /// swapchain image is available (`wait`) and signals `signal` when it is done, then
    /// blocks until the device has finished it.
    fn submit_drawn(&self, wait: vk::Semaphore, signal: vk::Semaphore) -> Result<(), VkError> {
        let device = &self.gpu.device;
        let (buffers, waits, signals) = ([self.commands], [wait], [signal]);
        let stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(&waits)
            .wait_dst_stage_mask(&stages)
            .command_buffers(&buffers)
            .signal_semaphores(&signals);
        // SAFETY: the command buffer is recording and everything it references stays alive
        // until the wait below returns. `wait` was signalled by the acquire of the image the
        // commands draw into, and `signal` is unsignalled: the presentation engine consumed
        // its previous signal before it handed the image out again. The queue and the fence
        // belong to this device, the fence is unsignalled, and the queue is only used from
        // here.
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

    /// Makes a swapchain that matches the surface and the window size. Returns `false` when
    /// the surface has no pixels now, in which case there is nothing to present into.
    fn ensure_swapchain(&mut self) -> Result<bool, VkError> {
        let requested = self.surface_size;
        let maintenance = self.uses_swapchain_maintenance();
        let device = &self.gpu.device;
        let surface = self.gpu.surface.as_ref().expect("checked by the caller");
        let presenter = self.presenter.as_mut().expect("made by the caller");
        if let Some(chain) = &presenter.swapchain
            && !chain.stale
            && chain.requested == requested
        {
            return Ok(true);
        }

        // SAFETY: the physical device and the surface belong to the live instance.
        let capabilities = unsafe {
            surface
                .loader
                .get_physical_device_surface_capabilities(self.gpu.physical, surface.handle)
        }
        .map_err(VkError::vulkan("vkGetPhysicalDeviceSurfaceCapabilitiesKHR"))?;
        let extent = swapchain_extent(&capabilities, requested);
        if extent.width == 0 || extent.height == 0 {
            return Ok(false);
        }
        // SAFETY: as above.
        let formats = unsafe {
            surface
                .loader
                .get_physical_device_surface_formats(self.gpu.physical, surface.handle)
        }
        .map_err(VkError::vulkan("vkGetPhysicalDeviceSurfaceFormatsKHR"))?;
        let (format, srgb) = choose_format(&formats).ok_or(VkError::Vulkan {
            call: "vkGetPhysicalDeviceSurfaceFormatsKHR",
            result: vk::Result::ERROR_FORMAT_NOT_SUPPORTED,
        })?;
        let pipeline = presenter.pipeline(&self.gpu, format.format)?;

        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 {
            image_count = image_count.min(capabilities.max_image_count);
        }
        let composite_alpha = [
            vk::CompositeAlphaFlagsKHR::OPAQUE,
            vk::CompositeAlphaFlagsKHR::INHERIT,
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
        ]
        .into_iter()
        .find(|mode| capabilities.supported_composite_alpha.contains(*mode))
        .unwrap_or(vk::CompositeAlphaFlagsKHR::OPAQUE);

        let loader = presenter
            .swapchain_loader
            .get_or_insert_with(|| ash::khr::swapchain::Device::new(self.gpu.instance(), device));
        if let Some(old) = &presenter.swapchain
            && !old.settle_presents(device)
        {
            // The presentation engine may still hold the semaphore of the last present. The
            // fence of a submission does not cover that. The fences of the presents do, where
            // swapchain maintenance gives them; without them, and when they are not
            // signalled in time, idling the device does.
            // SAFETY: the device is live.
            unsafe { device.device_wait_idle() }.map_err(VkError::vulkan("vkDeviceWaitIdle"))?;
            #[cfg(test)]
            {
                self.swapchain_idles += 1;
            }
        }
        if maintenance && presenter.maintenance_loader.is_none() {
            presenter.maintenance_loader = Some(ash::ext::swapchain_maintenance1::Device::new(
                self.gpu.instance(),
                device,
            ));
        }
        let old = presenter.swapchain.take();
        if old.is_some() {
            self.pacer.swapchain_replaced();
        }
        // Whether the swapchain is created for the pacing extensions, and with what.
        let mut pacing = None;
        if self.gpu.pacing.timing && self.pacer.wants_timing() {
            if presenter.timing_commands.is_none() {
                // SAFETY: the device is live, belongs to this instance and was created with
                // the extensions `pacing` names.
                let commands = unsafe {
                    ffi::Commands::load(self.gpu.instance(), device, self.gpu.pacing.wait)
                };
                match commands {
                    Some(commands) => {
                        let calibrated = ash::khr::calibrated_timestamps::Device::new(
                            self.gpu.instance(),
                            device,
                        );
                        presenter.timing_commands = Some((commands, calibrated));
                    }
                    None => self.pacer.fail((
                        "vkGetDeviceProcAddr",
                        vk::Result::ERROR_EXTENSION_NOT_PRESENT,
                    )),
                }
            }
            if let Some(commands) = &presenter.timing_commands {
                match surface_pacing(&self.gpu) {
                    Ok(supported) => pacing = supported.map(|pacing| (pacing, commands.clone())),
                    Err(failure) => self.pacer.fail(failure),
                }
            }
        }
        let flags = pacing
            .as_ref()
            .map_or(vk::SwapchainCreateFlagsKHR::empty(), |(pacing, _)| {
                pacing.swapchain_flags()
            });
        let info = vk::SwapchainCreateInfoKHR::default()
            .flags(flags)
            .surface(surface.handle)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(composite_alpha)
            // Every surface supports it, and it paces presentation to the display.
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .old_swapchain(
                old.as_ref()
                    .map_or(vk::SwapchainKHR::null(), |old| old.handle),
            );
        // SAFETY: the surface belongs to the instance of this device, which was created with
        // the swapchain extension and whose queue can present to it. The values come from the
        // surface's own capabilities and formats; the flags are those of extensions the
        // device was created with and the surface reports support for. Every submission was
        // waited for, so the old swapchain, if any, is idle and is retired by this call.
        let created = unsafe { loader.create_swapchain(&info, None) };
        if let Some(old) = old {
            old.destroy(device, loader);
        }
        let handle = created.map_err(VkError::vulkan("vkCreateSwapchainKHR"))?;
        let timing = pacing.and_then(|(pacing, (commands, calibrated))| {
            // SAFETY: the swapchain was just created from this device with the flags of
            // `pacing`, and the value is kept in the `Swapchain` that owns the handle; the
            // commands were loaded for this device.
            let timing = unsafe {
                SwapchainTiming::new(commands, calibrated, device.handle(), handle, pacing)
            };
            // The swapchain presents without the extension like any other.
            timing.map_err(|failure| self.pacer.fail(failure)).ok()
        });

        let mut chain = Swapchain {
            handle,
            extent,
            requested,
            srgb,
            pipeline,
            images: Vec::new(),
            views: Vec::new(),
            acquired: vk::Semaphore::null(),
            drawn: Vec::new(),
            stale: false,
            timing,
            present_fences: maintenance.then(Vec::new),
        };
        let filled = (|| {
            // SAFETY: the swapchain was just created from this device.
            chain.images = unsafe { loader.get_swapchain_images(chain.handle) }
                .map_err(VkError::vulkan("vkGetSwapchainImagesKHR"))?;
            let semaphore = || {
                // SAFETY: the device is live.
                unsafe { device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }
                    .map_err(VkError::vulkan("vkCreateSemaphore"))
            };
            chain.acquired = semaphore()?;
            for (index, &image) in chain.images.iter().enumerate() {
                self.gpu
                    .name_object(image, &format!("swapchain image {index}"));
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .subresource_range(COLOR_RANGE);
                // SAFETY: the image belongs to the live swapchain and the view matches its
                // format and its single colour subresource.
                let view = unsafe { device.create_image_view(&view_info, None) }
                    .map_err(VkError::vulkan("vkCreateImageView"))?;
                chain.views.push(view);
                chain.drawn.push(semaphore()?);
            }
            Ok(())
        })();
        match filled {
            Ok(()) => {
                presenter.swapchain = Some(chain);
                Ok(true)
            }
            Err(error) => {
                chain.destroy(device, loader);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_that_fits_exactly_fills_the_window() {
        let rect = letterbox(Size::new(640, 480), Size::new(1280, 960));
        assert_eq!(rect, Rect::new(0, 0, 1280, 960));
        assert!(is_whole_factor(Size::new(640, 480), &rect));
        assert_eq!(
            letterbox(Size::new(640, 480), Size::new(640, 480)),
            Rect::new(0, 0, 640, 480)
        );
    }

    #[test]
    fn wide_window_gets_bars_left_and_right() {
        let rect = letterbox(Size::new(640, 480), Size::new(1920, 1080));
        assert_eq!(rect, Rect::new(240, 0, 1440, 1080));
        assert!(!is_whole_factor(Size::new(640, 480), &rect), "2.25");
    }

    #[test]
    fn tall_window_gets_bars_above_and_below() {
        let rect = letterbox(Size::new(640, 480), Size::new(1280, 1200));
        assert_eq!(rect, Rect::new(0, 120, 1280, 960));
        assert!(is_whole_factor(Size::new(640, 480), &rect));
    }

    #[test]
    fn smaller_window_scales_the_frame_down() {
        assert_eq!(
            letterbox(Size::new(640, 480), Size::new(320, 300)),
            Rect::new(0, 30, 320, 240)
        );
        // Odd leftovers are split with the extra pixel after the image.
        assert_eq!(
            letterbox(Size::new(4, 4), Size::new(7, 4)),
            Rect::new(1, 0, 4, 4)
        );
        // A side that does not divide is rounded down.
        assert_eq!(
            letterbox(Size::new(640, 480), Size::new(1000, 1000)),
            Rect::new(0, 125, 1000, 750)
        );
        assert_eq!(
            letterbox(Size::new(3, 2), Size::new(100, 10)),
            Rect::new(42, 0, 15, 10)
        );
    }

    #[test]
    fn nothing_is_shown_without_pixels() {
        let frame = Size::new(640, 480);
        for window in [Size::new(0, 480), Size::new(640, 0), Size::new(0, 0)] {
            assert!(letterbox(frame, window).is_empty(), "{window:?}");
            assert_eq!(
                push_constants(same(frame), window, Mix::Newest, false, Filter::Auto),
                None
            );
        }
        assert!(letterbox(Size::new(0, 0), Size::new(640, 480)).is_empty());
        // One pixel wide: the scaled frame would be less than a pixel high.
        assert!(letterbox(frame, Size::new(1, 1000)).is_empty());
        assert!(letterbox(Size::new(u32::MAX, 1), Size::new(u32::MAX, u32::MAX)).width > 0);
    }

    #[test]
    fn whole_factor_needs_the_same_whole_number_both_ways() {
        let frame = Size::new(4, 3);
        assert!(is_whole_factor(frame, &Rect::new(0, 0, 4, 3)));
        assert!(is_whole_factor(frame, &Rect::new(5, 0, 12, 9)));
        assert!(!is_whole_factor(frame, &Rect::new(0, 0, 6, 4)));
        assert!(!is_whole_factor(frame, &Rect::new(0, 0, 8, 9)));
        assert!(!is_whole_factor(frame, &Rect::new(0, 0, 2, 1)));
        assert!(!is_whole_factor(frame, &Rect::default()));
    }

    #[test]
    fn filter_selects_the_sampling_flags() {
        let frame = Size::new(4, 4);
        let flags = |window: u32, filter| {
            words(push_constants(
                same(frame),
                Size::new(window, window),
                Mix::Newest,
                false,
                filter,
            ))[13]
        };
        let nearest = FLAG_NEAREST | FLAG_HELD_NEAREST | FLAG_SAME_GRID;
        assert_eq!(flags(12, Filter::Auto), nearest);
        assert_eq!(flags(10, Filter::Auto), FLAG_SAME_GRID);
        assert_eq!(flags(10, Filter::Nearest), nearest);
        assert_eq!(flags(12, Filter::Sharp), FLAG_SAME_GRID);
        assert_eq!(flags(12, Filter::Linear), FLAG_SAME_GRID | FLAG_LINEAR);
    }

    /// Both frames of one size, as when the logical size never changes.
    fn same(size: Size) -> Sources {
        Sources {
            newest: size,
            held: size,
            vagueness: None,
        }
    }

    /// A step whose weight and threshold differ, so that neither is taken for the other.
    const STEP: Mix = Mix::Step {
        weight: 64,
        threshold: 63,
    };

    fn words(bytes: Option<[u8; PUSH_CONSTANT_BYTES]>) -> Vec<u32> {
        crate::resources::read_words(&bytes.expect("something is visible"))
    }

    #[test]
    fn push_constants_carry_the_rectangles_the_frame_sizes_the_weight_and_the_flags() {
        let frame = Size::new(640, 480);
        let whole = push_constants(
            same(frame),
            Size::new(1920, 1440),
            Mix::Newest,
            false,
            Filter::Auto,
        );
        let rect = [0, 0, 1920, 1440, 640, 480];
        assert_eq!(
            words(whole),
            [
                rect.as_slice(),
                rect.as_slice(),
                &[0, FLAG_NEAREST | FLAG_HELD_NEAREST | FLAG_SAME_GRID, 0, 0]
            ]
            .concat()
        );
        let fractional =
            push_constants(same(frame), Size::new(1920, 1080), STEP, true, Filter::Auto);
        let rect = [240, 0, 1440, 1080, 640, 480];
        assert_eq!(
            words(fractional),
            [
                rect.as_slice(),
                rect.as_slice(),
                &[64, FLAG_SRGB_TARGET | FLAG_SAME_GRID | FLAG_MIX, 63, 0]
            ]
            .concat()
        );
    }

    #[test]
    fn push_constants_of_a_masked_draw_carry_the_flag_the_threshold_and_the_vagueness() {
        let frame = Size::new(640, 480);
        let masked = Sources {
            vagueness: Some(64),
            ..same(frame)
        };
        let constants = words(push_constants(masked, frame, STEP, false, Filter::Auto));
        assert_eq!(
            constants[12..],
            [
                64,
                FLAG_NEAREST | FLAG_HELD_NEAREST | FLAG_SAME_GRID | FLAG_MIX | FLAG_MASK,
                63,
                64
            ]
        );
        // Outside a transition the mask is not read.
        let alone = words(push_constants(
            masked,
            frame,
            Mix::Newest,
            false,
            Filter::Auto,
        ));
        assert_eq!(alone[13], FLAG_NEAREST | FLAG_HELD_NEAREST | FLAG_SAME_GRID);
        let filling = words(Some(filling_constants(masked, frame, STEP, Filter::Auto)));
        assert_eq!(filling, constants);
    }

    #[test]
    fn filling_constants_are_those_of_the_letterbox_rectangle_moved_to_the_origin() {
        let frame = Size::new(20, 18);
        // Windows in which the frame is scaled by a factor that is not whole, with bars on
        // either axis, including one whose rectangle would not be found again from its size.
        for window in [
            Size::new(30, 27),
            Size::new(50, 27),
            Size::new(30, 40),
            Size::new(100, 10),
            Size::new(1920, 1080),
        ] {
            let rect = letterbox(frame, window);
            let in_window = words(push_constants(
                same(frame),
                window,
                STEP,
                false,
                Filter::Auto,
            ));
            let filling = words(Some(filling_constants(
                same(frame),
                Size::new(rect.width, rect.height),
                STEP,
                Filter::Auto,
            )));
            // The same sizes, weight and flags; only the origins differ.
            assert_eq!(filling[2..6], in_window[2..6], "{window:?}");
            assert_eq!(filling[8..], in_window[8..], "{window:?}");
            assert_eq!([filling[0], filling[1], filling[6], filling[7]], [0; 4]);
            // Shown again in that window, the captured rectangle lies where it was.
            assert_eq!(
                letterbox(Size::new(rect.width, rect.height), window),
                rect,
                "{window:?}"
            );
        }
    }

    #[test]
    fn held_frame_of_another_size_gets_its_own_rectangle() {
        let sources = Sources {
            newest: Size::new(32, 24),
            held: Size::new(20, 18),
            vagueness: None,
        };
        // The held frame fills the target four times over; the newest is 2.5 times as wide
        // and leaves bars above and below.
        assert_eq!(
            words(push_constants(
                sources,
                Size::new(80, 72),
                STEP,
                false,
                Filter::Auto
            )),
            [
                0,
                6,
                80,
                60,
                32,
                24,
                0,
                0,
                80,
                72,
                20,
                18,
                64,
                FLAG_HELD_NEAREST | FLAG_MIX,
                63,
                0
            ]
        );
        // One of the two may have no pixel while the other still has.
        let thin = Sources {
            newest: Size::new(640, 480),
            held: Size::new(2, 100),
            vagueness: None,
        };
        let constants = words(push_constants(
            thin,
            Size::new(1, 50),
            STEP,
            false,
            Filter::Auto,
        ));
        assert_eq!(constants[..6], [0, 0, 0, 0, 640, 480]);
        assert_eq!(constants[6..12], [0, 0, 1, 50, 2, 100]);
    }

    fn format(format: vk::Format) -> vk::SurfaceFormatKHR {
        vk::SurfaceFormatKHR {
            format,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        }
    }

    #[test]
    fn unorm_surface_format_is_preferred_over_srgb() {
        let offered = [
            format(vk::Format::B8G8R8A8_SRGB),
            format(vk::Format::R8G8B8A8_UNORM),
            format(vk::Format::B8G8R8A8_UNORM),
        ];
        assert_eq!(
            choose_format(&offered),
            Some((format(vk::Format::B8G8R8A8_UNORM), false))
        );
        assert_eq!(
            choose_format(&offered[..2]),
            Some((format(vk::Format::R8G8B8A8_UNORM), false))
        );
    }

    #[test]
    fn srgb_surface_format_is_taken_with_the_inverse_transfer_when_nothing_else_is_offered() {
        let offered = [
            format(vk::Format::R16G16B16A16_SFLOAT),
            format(vk::Format::R8G8B8A8_SRGB),
        ];
        assert_eq!(
            choose_format(&offered),
            Some((format(vk::Format::R8G8B8A8_SRGB), true))
        );
        // A UNORM format in another colour space is not the pass-through one.
        let wide = vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_UNORM,
            color_space: vk::ColorSpaceKHR::DISPLAY_P3_NONLINEAR_EXT,
        };
        assert_eq!(
            choose_format(&[wide, format(vk::Format::B8G8R8A8_SRGB)]),
            Some((format(vk::Format::B8G8R8A8_SRGB), true))
        );
        assert_eq!(choose_format(&[wide]), Some((wide, false)));
        assert_eq!(choose_format(&[]), None);
    }

    #[test]
    fn swapchain_extent_follows_the_surface_or_the_window() {
        let extent = |width, height| vk::Extent2D { width, height };
        let fixed = vk::SurfaceCapabilitiesKHR {
            current_extent: extent(800, 600),
            min_image_extent: extent(800, 600),
            max_image_extent: extent(800, 600),
            ..Default::default()
        };
        assert_eq!(
            swapchain_extent(&fixed, Size::new(640, 480)),
            extent(800, 600)
        );

        let free = vk::SurfaceCapabilitiesKHR {
            current_extent: extent(u32::MAX, u32::MAX),
            min_image_extent: extent(1, 1),
            max_image_extent: extent(4096, 4096),
            ..Default::default()
        };
        assert_eq!(
            swapchain_extent(&free, Size::new(640, 480)),
            extent(640, 480)
        );
        assert_eq!(
            swapchain_extent(&free, Size::new(8000, 480)),
            extent(4096, 480)
        );
        let minimised = vk::SurfaceCapabilitiesKHR {
            current_extent: extent(0, 0),
            ..Default::default()
        };
        assert_eq!(
            swapchain_extent(&minimised, Size::new(640, 480)),
            extent(0, 0)
        );
    }
}
