//! Test only: runs a compute shader that maps one storage buffer to another, so that GLSL
//! helpers can be checked on the device without going through a frame; and records a
//! synchronization hazard on purpose, so that the tests can see it reported.

use ash::vk;
use gpu_allocator::MemoryLocation;

use super::VkCompositor;
use crate::resources::{
    COLOR_LAYERS, COLOR_RANGE, GpuBuffer, GpuImage, ImageUse, create_shader_module,
    make_host_readable, read_words, transition, write_words,
};

impl VkCompositor {
    /// Runs `spirv` with `input` at binding 0 and a zeroed buffer of `output_words` words at
    /// binding 1, passing `count` as the only push constant and dispatching enough workgroups
    /// of `local_size` invocations to cover `count`. Returns the output buffer.
    ///
    /// Panics on any Vulkan error, as test code may.
    pub(crate) fn run_buffer_shader(
        &mut self,
        spirv: &[u8],
        input: &[u32],
        output_words: usize,
        count: u32,
        local_size: u32,
    ) -> Vec<u32> {
        // A clone, so that the submission below can borrow the compositor.
        let device = &self.gpu.device.clone();
        let buffer = |allocator: &mut _, name, words: usize, location| {
            GpuBuffer::new(
                device,
                allocator,
                name,
                (words * 4) as u64,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                location,
            )
            .unwrap()
        };
        let mut input_buffer = buffer(
            &mut self.allocator,
            "test input",
            input.len(),
            MemoryLocation::CpuToGpu,
        );
        write_words(input_buffer.bytes_mut(), input);
        let mut output_buffer = buffer(
            &mut self.allocator,
            "test output",
            output_words,
            MemoryLocation::GpuToCpu,
        );
        output_buffer.bytes_mut().fill(0);

        let bindings = [0, 1].map(|binding| {
            vk::DescriptorSetLayoutBinding::default()
                .binding(binding)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE)
        });
        let set_layouts;
        let push_ranges = [vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .size(4)];
        let pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(2)];
        let buffer_infos = [&input_buffer, &output_buffer].map(|buffer| {
            [vk::DescriptorBufferInfo::default()
                .buffer(buffer.buffer)
                .range(vk::WHOLE_SIZE)]
        });
        // SAFETY: every object is created from the live device, used by one submission that is
        // waited for, and destroyed afterwards; the shader only indexes its buffers below
        // `count`, which the caller sized them for.
        unsafe {
            let module = create_shader_module(device, spirv).unwrap();
            let set_layout = device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings),
                    None,
                )
                .unwrap();
            set_layouts = [set_layout];
            let layout = device
                .create_pipeline_layout(
                    &vk::PipelineLayoutCreateInfo::default()
                        .set_layouts(&set_layouts)
                        .push_constant_ranges(&push_ranges),
                    None,
                )
                .unwrap();
            let stage = vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::COMPUTE)
                .module(module)
                .name(c"main");
            let pipeline = device
                .create_compute_pipelines(
                    vk::PipelineCache::null(),
                    &[vk::ComputePipelineCreateInfo::default()
                        .stage(stage)
                        .layout(layout)],
                    None,
                )
                .unwrap()[0];
            let pool = device
                .create_descriptor_pool(
                    &vk::DescriptorPoolCreateInfo::default()
                        .max_sets(1)
                        .pool_sizes(&pool_sizes),
                    None,
                )
                .unwrap();
            let set = device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::default()
                        .descriptor_pool(pool)
                        .set_layouts(&set_layouts),
                )
                .unwrap()[0];
            let writes = [0, 1].map(|binding| {
                vk::WriteDescriptorSet::default()
                    .dst_set(set)
                    .dst_binding(binding)
                    .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                    .buffer_info(&buffer_infos[binding as usize])
            });
            device.update_descriptor_sets(&writes, &[]);

            self.begin_commands().unwrap();
            let commands = self.commands;
            device.cmd_bind_pipeline(commands, vk::PipelineBindPoint::COMPUTE, pipeline);
            device.cmd_bind_descriptor_sets(
                commands,
                vk::PipelineBindPoint::COMPUTE,
                layout,
                0,
                &[set],
                &[],
            );
            device.cmd_push_constants(
                commands,
                layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                &count.to_ne_bytes(),
            );
            device.cmd_dispatch(commands, count.div_ceil(local_size), 1, 1);
            // The output is read through its mapping once the submission is done.
            make_host_readable(
                device,
                commands,
                output_buffer.buffer,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_WRITE,
            );
            self.submit_and_wait().unwrap();

            device.destroy_descriptor_pool(pool, None);
            device.destroy_pipeline(pipeline, None);
            device.destroy_pipeline_layout(layout, None);
            device.destroy_descriptor_set_layout(set_layout, None);
            device.destroy_shader_module(module, None);
        }

        let output = read_words(&output_buffer.bytes()[..output_words * 4]);
        input_buffer.destroy(device, &mut self.allocator);
        output_buffer.destroy(device, &mut self.allocator);
        output
    }
}

impl VkCompositor {
    /// Clears a new image and copies it to a buffer in the same submission with no barrier
    /// between the clear, which writes the image, and the copy, which reads it: a
    /// read-after-write hazard that synchronization validation reports.
    ///
    /// Panics on any Vulkan error, as test code may.
    pub(crate) fn record_unguarded_read(&mut self) {
        // A clone, so that the submission below can borrow the compositor.
        let device = &self.gpu.device.clone();
        let size = (4, 4);
        let image = GpuImage::new(
            device,
            &mut self.allocator,
            "test hazard image",
            size,
            // Sampled only for the view every image gets.
            vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::SAMPLED,
            vk::ImageCreateFlags::empty(),
        )
        .unwrap();
        let buffer = GpuBuffer::new(
            device,
            &mut self.allocator,
            "test hazard buffer",
            u64::from(size.0 * size.1 * 4),
            vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::GpuToCpu,
        )
        .unwrap();
        let region = vk::BufferImageCopy::default()
            .image_subresource(COLOR_LAYERS)
            .image_extent(vk::Extent3D {
                width: size.0,
                height: size.1,
                depth: 1,
            });
        // SAFETY: the command buffer is recording; the image and the buffer are live, of the
        // sizes the commands use, and are destroyed only after the submission was waited
        // for. The missing barrier is the point of the test; in `GENERAL` both commands are
        // valid apart from it.
        unsafe {
            self.begin_commands().unwrap();
            let commands = self.commands;
            let general = ImageUse::TRANSFER_WRITE.in_layout(vk::ImageLayout::GENERAL);
            transition(device, commands, image.image, ImageUse::UNDEFINED, general);
            device.cmd_clear_color_image(
                commands,
                image.image,
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue { uint32: [0; 4] },
                &[COLOR_RANGE],
            );
            device.cmd_copy_image_to_buffer(
                commands,
                image.image,
                vk::ImageLayout::GENERAL,
                buffer.buffer,
                &[region],
            );
            self.submit_and_wait().unwrap();
        }
        image.destroy(device, &mut self.allocator);
        buffer.destroy(device, &mut self.allocator);
    }
}
