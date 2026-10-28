//! Rendering a packet: its deltas, the data of its frame, and reading the frame back.

use ash::vk;
use vkrgss_proto::{BitmapDelta, FramePacket, GuestToHost, Rect, Size, TableDelta};

use super::{Frame, FrameData, Pending, Step, VkCompositor};
use crate::binning::{FRAME_WORK_BUDGET, bin};
use crate::bitmaps::Change;
use crate::records::{ClipRect, RECORD_WORDS, TILEMAP_WORDS, check_logical_size, encode_frame};
use crate::resources::GpuImage;
use crate::{FrameTiming, Limit, VkError};

impl VkCompositor {
    /// Handles one message. A `Frame` message is rendered and yields `true`; every other
    /// message yields `false`. The bitmap changes of a `Bitmaps` message are applied as those
    /// of a frame are, which is the same as applying them with the next frame before its own.
    /// Freeze, transition, audio, modal and title messages are outside what this compositor
    /// models and have no effect on later frames.
    pub fn handle(&mut self, message: &GuestToHost) -> Result<bool, VkError> {
        match message {
            GuestToHost::Frame(packet) => self.render(packet).map(|()| true),
            GuestToHost::Bitmaps(deltas) => self
                .unless_lost(|compositor| compositor.apply_bitmaps(deltas))
                .map(|()| false),
            _ => Ok(false),
        }
    }

    /// Applies bitmap deltas in order and carries out their GPU work, without drawing, and
    /// waits for it.
    pub(crate) fn apply_bitmaps(&mut self, deltas: &[BitmapDelta]) -> Result<(), VkError> {
        let mut pending = Pending::default();
        let checked = self.queue_deltas(deltas, &mut pending);
        if let Err(error) = &checked {
            self.note_loss(error);
        }
        self.execute_and_wait(pending)?;
        checked
    }

    /// Applies the packet's bitmap deltas and then its table deltas, then draws its ops into
    /// the frame target, darkened by the packet's brightness.
    ///
    /// Either a complete frame is rendered or an error is returned and the frame target keeps
    /// the previous frame, also when the logical size changed and the target of the new size
    /// cannot be had. Bitmap state after an error is that of the reference compositor:
    /// the deltas before an invalid delta stay applied and the ones after it do not, and an
    /// invalid op leaves every delta applied.
    ///
    /// A [`VkError::Limit`] is returned for a packet that is valid but beyond the device or
    /// this backend, among them a frame whose ops cover more tiles in total than the work
    /// budget ([`Limit::FrameWork`]), which is refused before anything is allocated for it. A
    /// limit met by a delta behaves like an invalid delta; a limit met by the frame as a whole
    /// leaves every delta applied.
    ///
    /// [`VkError::DeviceLost`] is returned when the device is lost, or the frame did not
    /// complete in time, and for every frame after that.
    ///
    /// The uploads and the compose dispatch are submitted without waiting for them: this
    /// returns once they are submitted, and the next method that waits for them finds out
    /// how they ended. An error of the device while it runs them, a lost device or a frame
    /// that did not complete in time, is therefore returned by that method, which may be the
    /// next `render`, and not by this one. `render` itself waits for the compose of the
    /// `render` before it, so at most one is in flight.
    pub fn render(&mut self, packet: &FramePacket) -> Result<(), VkError> {
        self.unless_lost(|compositor| compositor.render_packet(packet))
    }

    fn render_packet(&mut self, packet: &FramePacket) -> Result<(), VkError> {
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
                // A failure of the device outranks an invalid packet. A lost device, which
                // an image of a new bitmap may have met, ends the packet here.
                self.note_loss(&error);
                self.execute(pending, None)?;
                Err(error)
            }
        }
    }

    /// Forgets every bitmap and table, as the deltas that destroy each of them would. The
    /// frame target and what the presenter holds stay. For a guest that starts sending anew:
    /// the next packet may create any bitmap and set any table.
    pub fn forget_resources(&mut self) -> Result<(), VkError> {
        self.unless_lost(|compositor| compositor.forget_all())
    }

    fn forget_all(&mut self) -> Result<(), VkError> {
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
        if let Err(error) = &checked {
            self.note_loss(error);
        }
        self.execute_and_wait(pending)?;
        checked
    }

    /// The most recently rendered frame, copied from the device, or `None` before the first
    /// successful [`render`](VkCompositor::render). The copy is submitted after the compose
    /// of that frame, and one wait covers both.
    pub fn read_frame(&mut self) -> Result<Option<Frame>, VkError> {
        self.unless_lost_after_compose(|compositor| compositor.read_target())
    }

    fn read_target(&mut self) -> Result<Option<Frame>, VkError> {
        let Some(target) = self.target.as_ref().filter(|_| self.has_frame) else {
            return Ok(None);
        };
        let (image, size) = (target.image, Size::new(target.width, target.height));
        let rgba = self.read_image(image, (size.width, size.height), self.frame_use())?;
        Ok(Some(Frame { size, rgba }))
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
        // The budget comes first: a frame over it is refused before its lists are made.
        let max_entries = (max_bytes / 4).min(FRAME_WORK_BUDGET);
        let tiles = bin(size, &clips, max_entries).map_err(|entries| match entries {
            entries if entries > FRAME_WORK_BUDGET => VkError::Limit {
                limit: Limit::FrameWork,
                requested: entries,
                maximum: FRAME_WORK_BUDGET,
            },
            entries => data_limit(entries * 4),
        })?;
        let bin = started.map(|started| started.elapsed());
        Ok(FrameData {
            size,
            brightness: packet.brightness,
            records,
            tiles,
            encode,
            bin,
        })
    }

    /// Checks each delta against the table, applies it there and queues its GPU work.
    pub(super) fn queue_deltas<'a>(
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
                    let image = &self.images[&slot];
                    pending.steps.push(Step::Upload {
                        image: image.image,
                        rect,
                        whole: rect == Rect::new(0, 0, image.width, image.height),
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

    /// Carries out queued bitmap work, without a frame, and waits for what it submitted, also
    /// when it returns an error other than a lost device: a failed allocation may follow a
    /// submission of the steps. A lost device that the wait finds is returned before that
    /// error; once the device is found lost nothing is waited for.
    fn execute_and_wait(&mut self, pending: Pending<'_>) -> Result<(), VkError> {
        let executed = self.execute(pending, None);
        if let Err(error) = &executed {
            self.note_loss(error);
        }
        if self.lost.is_none() {
            self.settle()?;
        }
        executed
    }

    /// Carries out queued bitmap work and, when there is one, the frame: once the compose in
    /// flight has completed, one submission, which becomes the compose in flight. Everything
    /// the work was the last user of is released once a wait has seen it complete, or at once
    /// when nothing was submitted. That release happens on every path, so a failure leaks
    /// nothing, except on a lost device, which is not used at all once it is found lost,
    /// before or during the submission.
    pub(super) fn execute(
        &mut self,
        pending: Pending<'_>,
        frame: Option<&FrameData>,
    ) -> Result<(), VkError> {
        if let Some(call) = self.lost {
            return Err(VkError::DeviceLost { call });
        }
        // The descriptors, the frame buffers and the images are only written or released
        // once nothing in flight uses them.
        self.settle()?;
        self.write_bitmap_descriptors(&pending.touched);
        let result = self.submit(&pending, frame);
        if let Err(error) = &result {
            self.note_loss(error);
        }
        if let Some(in_flight) = &mut self.in_flight {
            // Submitted, so the images go once it has completed.
            in_flight.retired.extend(pending.retired);
            return result;
        }
        // Nothing went out, or the submission failed. Whatever was submitted has to finish
        // before the objects below are destroyed and the frame buffers are written again. On
        // a lost device it may still be running: the retired images are leaked, and the
        // device is not idled, which might never end.
        if result.is_err() && !self.idle_after_failure() {
            return result;
        }
        for image in pending.retired {
            image.destroy(&self.gpu.device, &mut self.allocator);
        }
        result
    }
}
