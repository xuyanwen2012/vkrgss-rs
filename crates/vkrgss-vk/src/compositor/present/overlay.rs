//! The overlay: a picture of the caller's drawn over the frame in the window, such as the
//! on-screen buttons of a phone or a screen of the app itself, with nothing beneath it before
//! the first frame.
//!
//! The presentation pass draws it after the frame, blended by its alpha, through a pipeline of
//! its own (`shaders/overlay.frag`). Only presents draw it: frames read back with
//! [`VkCompositor::read_frame`] or [`VkCompositor::scale_frame`] never hold it.

use ash::vk;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::Allocator;
use vkrgss_proto::Size;

use super::presenter::DrawCall;
use super::scaling::PUSH_CONSTANT_BYTES;
use super::swapchain::FormatPipeline;
use crate::VkError;
use crate::compositor::VkCompositor;
use crate::device::Gpu;
use crate::resources::{
    COLOR_LAYERS, GpuBuffer, GpuImage, ImageUse, create_shader_module, transition, write_words,
};
use crate::shared::FLAG_SRGB_TARGET;

const VERTEX_SPIRV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/present.vert.spv"));
const FRAGMENT_SPIRV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/overlay.frag.spv"));

/// A picture to draw over the frame: `size.width * size.height` pixels of RGBA8 in row order,
/// premultiplied by their alpha, with sRGB-encoded colour. It is drawn at the size of the
/// window; a picture of another size is scaled to it by the nearest pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayImage {
    pub size: Size,
    pub pixels: Vec<u8>,
}

/// The pipelines, the descriptor set and the image of the overlay. Owned by the presenter,
/// which destroys it before the device.
pub(in crate::compositor) struct Overlay {
    set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptors: vk::DescriptorSet,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
    /// One pipeline per target format in use.
    pipelines: Vec<FormatPipeline>,
    /// The picture, in the `SHADER_READ_ONLY_OPTIMAL` layout between submissions, which the
    /// descriptor set points at. `None` while there is nothing to draw.
    image: Option<GpuImage>,
}

impl Overlay {
    fn new(device: &ash::Device) -> Result<Overlay, VkError> {
        // Handles start out null so that `destroy` can clean up after a failure part way
        // through `init`: destroying a null handle is a no-op.
        let mut overlay = Overlay {
            set_layout: vk::DescriptorSetLayout::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            descriptors: vk::DescriptorSet::null(),
            vertex: vk::ShaderModule::null(),
            fragment: vk::ShaderModule::null(),
            pipelines: Vec::new(),
            image: None,
        };
        match overlay.init(device) {
            Ok(()) => Ok(overlay),
            // Nothing of a lost device is used again: what was made is leaked.
            Err(error) if error.is_device_lost() => Err(error),
            Err(error) => {
                overlay.destroy_handles(device);
                Err(error)
            }
        }
    }

    fn init(&mut self, device: &ash::Device) -> Result<(), VkError> {
        let bindings = [vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        // SAFETY: the device is live and the description is valid for the call.
        self.set_layout = unsafe { device.create_descriptor_set_layout(&layout_info, None) }
            .map_err(VkError::vulkan("vkCreateDescriptorSetLayout"))?;
        let set_layouts = [self.set_layout];
        // The constants of a draw call are as large as those of the frame's draw.
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
            .descriptor_count(1)];
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

    /// The pipeline that blends the picture into a target of `format`, made on first use.
    fn pipeline(&mut self, gpu: &Gpu, format: vk::Format) -> Result<vk::Pipeline, VkError> {
        if let Some(known) = self.pipelines.iter().find(|known| known.format == format) {
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
        // Premultiplied colour over what is there.
        let blend_attachments = [vk::PipelineColorBlendAttachmentState::default()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
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
        // Every format a target of the presenter has can be blended into: the swapchain's
        // formats are `UNORM` or `SRGB` formats of 8-bit channels, which Vulkan requires to
        // support blending.
        let pipelines = unsafe {
            gpu.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_info, None)
        };
        let pipeline = pipelines
            .map_err(|(_, result)| VkError::from_result("vkCreateGraphicsPipelines", result))?[0];
        gpu.name_object(pipeline, &format!("overlay pipeline {format:?}"));
        self.pipelines.push(FormatPipeline { format, pipeline });
        Ok(pipeline)
    }

    /// The draw of the picture into a target of `format` that is `target` pixels large, or
    /// `None` without a picture.
    pub(super) fn draw(
        &mut self,
        gpu: &Gpu,
        format: vk::Format,
        target: Size,
        srgb_target: bool,
    ) -> Result<Option<DrawCall>, VkError> {
        if self.image.is_none() {
            return Ok(None);
        }
        let pipeline = self.pipeline(gpu, format)?;
        let mut constants = [0; PUSH_CONSTANT_BYTES];
        let flags = if srgb_target { FLAG_SRGB_TARGET } else { 0 };
        write_words(&mut constants, &[target.width, target.height, flags]);
        Ok(Some(DrawCall {
            pipeline,
            pipeline_layout: self.pipeline_layout,
            descriptors: self.descriptors,
            constants,
        }))
    }

    /// Destroys the handles, not the image.
    fn destroy_handles(&mut self, device: &ash::Device) {
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

    /// Destroys everything. The device must be idle.
    pub(in crate::compositor) fn destroy(
        mut self,
        device: &ash::Device,
        allocator: &mut Allocator,
    ) {
        if let Some(image) = self.image.take() {
            image.destroy(device, allocator);
        }
        self.destroy_handles(device);
    }
}

impl VkCompositor {
    /// Sets the picture drawn over the frame by every present from now on, or with `None`
    /// draws none. See [`OverlayImage`]. The picture is copied to the device before this
    /// returns; frames read back never hold it.
    ///
    /// # Panics
    ///
    /// When the picture has no pixels or does not hold four bytes for each of them.
    pub fn set_overlay(&mut self, picture: Option<&OverlayImage>) -> Result<(), VkError> {
        self.unless_lost(|compositor| compositor.set_overlay_now(picture))
    }

    fn set_overlay_now(&mut self, picture: Option<&OverlayImage>) -> Result<(), VkError> {
        let Some(picture) = picture else {
            if let Some(overlay) = self.presenter.as_mut().and_then(|p| p.overlay.as_mut())
                && let Some(image) = overlay.image.take()
            {
                // Every submission was waited for, so nothing uses the image.
                image.destroy(&self.gpu.device, &mut self.allocator);
            }
            return Ok(());
        };
        let Size { width, height } = picture.size;
        assert!(width > 0 && height > 0, "an overlay picture without pixels");
        assert_eq!(
            picture.pixels.len(),
            width as usize * height as usize * 4,
            "the pixels of an overlay picture of {width} by {height}"
        );
        self.ensure_presenter()?;
        let presenter = self.presenter.as_mut().expect("just ensured");
        if presenter.overlay.is_none() {
            presenter.overlay = Some(Overlay::new(&self.gpu.device)?);
        }
        let overlay = presenter.overlay.as_mut().expect("just made");
        // A picture of another size replaces the image: the one there is goes.
        let kept = match overlay.image.take() {
            Some(image) if (image.width, image.height) == (width, height) => Some(image),
            Some(image) => {
                image.destroy(&self.gpu.device, &mut self.allocator);
                None
            }
            None => None,
        };
        let (image, before) = match kept {
            Some(image) => (image, ImageUse::FRAGMENT_SAMPLED_READ),
            None => {
                let image = GpuImage::new(
                    &self.gpu.device,
                    &mut self.allocator,
                    "overlay",
                    (width, height),
                    vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                    vk::ImageCreateFlags::empty(),
                )?;
                self.gpu.name_object(image.image, "overlay");
                (image, ImageUse::UNDEFINED)
            }
        };
        match self.upload_overlay(&image, before, &picture.pixels) {
            Ok(()) => {
                let view = image.view;
                let overlay = self
                    .presenter
                    .as_mut()
                    .and_then(|presenter| presenter.overlay.as_mut())
                    .expect("made above");
                overlay.image = Some(image);
                let info = [vk::DescriptorImageInfo::default()
                    .image_view(view)
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
                let write = vk::WriteDescriptorSet::default()
                    .dst_set(overlay.descriptors)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&info);
                // SAFETY: the set and the view are live and no command buffer that uses the
                // set is pending: every submission is waited for.
                unsafe { self.gpu.device.update_descriptor_sets(&[write], &[]) };
                Ok(())
            }
            Err(error) => {
                // The device is idle after a failed submission; on a lost device, which the
                // staging buffer may have found too, the image is leaked.
                self.note_loss(&error);
                self.discard_image(image);
                Err(error)
            }
        }
    }

    /// Copies `pixels` into the overlay image, which was last used as `before` says, and
    /// leaves it to the fragment shader.
    fn upload_overlay(
        &mut self,
        image: &GpuImage,
        before: ImageUse,
        pixels: &[u8],
    ) -> Result<(), VkError> {
        let mut staging = GpuBuffer::new(
            &self.gpu.device,
            &mut self.allocator,
            "overlay staging",
            pixels.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
        )?;
        staging.bytes_mut()[..pixels.len()].copy_from_slice(pixels);
        let (buffer, target, extent) = (
            staging.buffer,
            image.image,
            vk::Extent3D {
                width: image.width,
                height: image.height,
                depth: 1,
            },
        );
        let copied = self.submit_or_idle(|compositor| {
            compositor.begin_commands()?;
            let (device, commands) = (&compositor.gpu.device, compositor.commands);
            let region = vk::BufferImageCopy::default()
                .image_subresource(COLOR_LAYERS)
                .image_extent(extent);
            // SAFETY: the command buffer is recording, on a queue that draws since there is a
            // presenter. The image is live, in the layout of `before`, with transfer-
            // destination usage and as large as `extent`; the buffer is live, holds four
            // bytes for each of its pixels, and both outlive the wait of the submission.
            unsafe {
                transition(device, commands, target, before, ImageUse::TRANSFER_WRITE);
                device.cmd_copy_buffer_to_image(
                    commands,
                    buffer,
                    target,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[region],
                );
                transition(
                    device,
                    commands,
                    target,
                    ImageUse::TRANSFER_WRITE,
                    ImageUse::FRAGMENT_SAMPLED_READ,
                );
            }
            compositor.submit_and_wait()
        });
        // Waited for, or the device was idled after a failure.
        self.discard_buffer(staging);
        copied
    }
}
