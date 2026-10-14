//! What a frame reads and writes on the device: the bitmap descriptors, the frame buffers,
//! the table buffer and the frame target.

use std::collections::BTreeSet;

use ash::vk;
use gpu_allocator::MemoryLocation;
use vkrgss_proto::Size;

use super::{FrameBuffer, FrameData, VkCompositor, binding};
use crate::VkError;
use crate::records::{RECORD_WORDS, TILEMAP_WORDS};
use crate::resources::{GpuBuffer, GpuImage, write_words};

impl VkCompositor {
    /// Points the bitmap array at the current image of every slot whose occupant changed.
    ///
    /// A slot left without an image keeps its old descriptor. The binding is partially bound
    /// and no record refers to such a slot, so that descriptor is never used.
    pub(super) fn write_bitmap_descriptors(&self, touched: &BTreeSet<u32>) {
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
    pub(super) fn frame_buffer(
        &mut self,
        role: FrameBuffer,
        size: u64,
    ) -> Result<&mut GpuBuffer, VkError> {
        let slot = role as usize;
        let kept = self.frame_buffers[slot].as_ref().map(|(_, bytes)| *bytes);
        if kept.is_none_or(|bytes| bytes < size) {
            self.probe.before_frame_buffer(role.name())?;
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
            self.probe.frame_buffer_created();
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
    pub(super) fn prepare_frame_resources(&mut self, frame: &FrameData) -> Result<bool, VkError> {
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
}
