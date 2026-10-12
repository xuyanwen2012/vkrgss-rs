//! The held frame: capturing what is shown, and the images a draw reads beside the newest
//! frame.

use ash::vk;
use vkxp_proto::Size;

use super::scaling::{Mix, Placement, letterbox};
use crate::VkError;
use crate::compositor::VkCompositor;
use crate::resources::{COLOR_LAYERS, COLOR_RANGE, GpuImage, ImageUse, transition};

/// What the held frame and an empty target are cleared to: opaque black.
const BLACK: [u32; 4] = [0, 0, 0, 255];

impl VkCompositor {
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
    ///
    /// [`Filter::Auto`]: crate::Filter::Auto
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

    /// Makes sure there is the texel that stands in for a mask.
    pub(super) fn ensure_no_mask(&mut self) -> Result<(), VkError> {
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
    pub(super) fn ensure_held(&mut self, size: (u32, u32)) -> Result<(), VkError> {
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
}
