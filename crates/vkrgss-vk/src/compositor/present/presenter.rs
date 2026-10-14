//! The presenter: the pipelines, the descriptor set and the images that scale the frame into
//! a target, and the pass that draws it.

use ash::vk;
use gpu_allocator::vulkan::Allocator;
use vkrgss_proto::Size;

use super::scaling::{PUSH_CONSTANT_BYTES, Sources};
use super::swapchain::{FormatPipeline, Swapchain};
use crate::VkError;
use crate::compositor::VkCompositor;
use crate::device::Gpu;
use crate::present_timing as ffi;
use crate::resources::{GpuImage, ImageUse, create_shader_module, transition};

const VERTEX_SPIRV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/present.vert.spv"));
const FRAGMENT_SPIRV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/present.frag.spv"));

/// Bindings of the presenter's descriptor set, as `shaders/present.frag` declares them.
mod binding {
    pub(super) const NEWEST: u32 = 0;
    pub(super) const HELD: u32 = 1;
    pub(super) const MASK: u32 = 2;
}

/// Everything that scales the frame into a target. Owned by the compositor, which destroys it
/// before the device.
pub(in crate::compositor) struct Presenter {
    set_layout: vk::DescriptorSetLayout,
    pub(super) pipeline_layout: vk::PipelineLayout,
    descriptor_pool: vk::DescriptorPool,
    pub(super) descriptors: vk::DescriptorSet,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
    /// One pipeline per target format in use: the off-screen one and the swapchain's.
    pipelines: Vec<FormatPipeline>,
    /// The held frame, in the `SHADER_READ_ONLY_OPTIMAL` layout between submissions. It has
    /// the size of the frame it was captured from, which later frames need not share, and
    /// lives until the next capture. Black, at the size of the first frame, until a freeze.
    pub(super) held: Option<GpuImage>,
    /// The buffer of the frozen picture of RGSS3: 640 by 480 texels, in the
    /// `SHADER_READ_ONLY_OPTIMAL` layout between submissions, opaque black when it is made,
    /// which is when a freeze first asks for it. It lives as long as the presenter and is
    /// only ever written in its top left corner.
    pub(super) buffer: Option<GpuImage>,
    /// Whether the buffer is the held picture: since the last freeze into it, until the
    /// next capture of a held frame.
    pub(super) buffer_held: bool,
    /// One texel, in the `SHADER_READ_ONLY_OPTIMAL` layout, bound in place of a mask when a
    /// draw has none.
    pub(super) no_mask: Option<GpuImage>,
    /// The views of the newest frame, the held frame and the mask that the descriptor set
    /// points at.
    pub(super) bound: Option<[vk::ImageView; 3]>,
    pub(super) swapchain_loader: Option<ash::khr::swapchain::Device>,
    /// The commands of `VK_EXT_swapchain_maintenance1`, loaded with the first swapchain that
    /// uses them.
    pub(super) maintenance_loader: Option<ash::ext::swapchain_maintenance1::Device>,
    /// The commands of the pacing extensions, loaded with the first swapchain that is
    /// created for them.
    pub(super) timing_commands: Option<(ffi::Commands, ash::khr::calibrated_timestamps::Device)>,
    pub(super) swapchain: Option<Swapchain>,
}

/// One draw of the frame: what to bind and the constants to push.
pub(super) struct DrawCall {
    pub(super) pipeline: vk::Pipeline,
    pub(super) pipeline_layout: vk::PipelineLayout,
    pub(super) descriptors: vk::DescriptorSet,
    pub(super) constants: [u8; PUSH_CONSTANT_BYTES],
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
            buffer: None,
            buffer_held: false,
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
    pub(super) fn pipeline(
        &mut self,
        gpu: &Gpu,
        format: vk::Format,
    ) -> Result<vk::Pipeline, VkError> {
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

    /// Forgets what the descriptor set points at, so that the next [`Presenter::bind`]
    /// writes it. For when a view it holds was destroyed: a new view may get the handle of
    /// the old one, and the comparison in `bind` would then take the set for current.
    pub(in crate::compositor) fn forget_bound(&mut self) {
        self.bound = None;
    }

    /// Destroys everything. The device must be idle.
    pub(in crate::compositor) fn destroy(
        mut self,
        device: &ash::Device,
        allocator: &mut Allocator,
    ) {
        if let (Some(swapchain), Some(loader)) = (self.swapchain.take(), &self.swapchain_loader) {
            // The device is idle either way; this only gives the fences the time to signal.
            swapchain.settle_presents(device);
            swapchain.destroy(device, loader);
        }
        if let Some(held) = self.held.take() {
            held.destroy(device, allocator);
        }
        if let Some(buffer) = self.buffer.take() {
            buffer.destroy(device, allocator);
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
pub(super) struct PassTarget {
    pub(super) image: vk::Image,
    /// A view of the whole image, in the format the pipeline was made for.
    pub(super) view: vk::ImageView,
    pub(super) extent: vk::Extent2D,
    /// How the image was last used. Its contents are not kept.
    pub(super) before: ImageUse,
    /// The use the image is left to after the pass.
    pub(super) after: ImageUse,
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
pub(super) unsafe fn record_pass(
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
    /// Runs `work`, which records and submits commands, and idles the device if it fails, so
    /// that whatever was submitted is finished before the caller releases anything.
    pub(super) fn submit_or_idle(
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

    pub(super) fn ensure_presenter(&mut self) -> Result<(), VkError> {
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
    ///
    /// While the buffer of RGSS3 is the held picture and the newest frame fits into it, the
    /// buffer is bound as the held frame and counted at the size of the newest frame: the
    /// draw then mixes buffer texel (x, y) with newest texel (x, y), as it mixes two frames
    /// of one size. A newest frame that does not fit meets the whole buffer as a held frame
    /// of another size.
    pub(super) fn prepare_sources(&mut self) -> Result<Option<Sources>, VkError> {
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
        let newest_size = Size::new(size.0, size.1);
        let (held_view, held_size) = match presenter.buffer.as_ref() {
            Some(buffer) if presenter.buffer_held => {
                let whole = Size::new(buffer.width, buffer.height);
                let fits = size.0 <= buffer.width && size.1 <= buffer.height;
                (buffer.view, if fits { newest_size } else { whole })
            }
            _ => {
                let held = presenter.held.as_ref().expect("just ensured");
                (held.view, Size::new(held.width, held.height))
            }
        };
        let no_mask = presenter.no_mask.as_ref().expect("just ensured").view;
        let mask_view = mask.map_or(no_mask, |(view, _)| view);
        presenter.bind(&self.gpu.device, newest, held_view, mask_view);
        Ok(Some(Sources {
            newest: newest_size,
            held: held_size,
            vagueness: mask.map(|(_, vagueness)| vagueness),
        }))
    }
}
