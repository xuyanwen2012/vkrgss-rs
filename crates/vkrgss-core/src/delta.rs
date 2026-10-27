//! Bitmap deltas: how the pixels of the arena's bitmaps reach the host.
//!
//! Every drawing operation records the bounding rectangle of the pixels it may have changed.
//! [`Bitmaps::deltas`] turns that, for the bitmaps a frame refers to, into the smallest list of
//! [`BitmapDelta`]s that brings a compositor's bitmap store up to date. [`split_deltas`] cuts
//! such a list into parts of a bounded size, for the bitmap changes a guest sends ahead of a
//! frame.

use std::collections::HashSet;

use vkrgss_proto::{BitmapDelta, BitmapId, Rect};

use crate::bitmap::{Area, Bitmap};
use crate::{BitmapError, Bitmaps};

impl Area {
    /// The area as a protocol rectangle.
    fn to_proto(self) -> vkrgss_proto::Rect {
        // Bitmap sides are at most `MAX_BITMAP_PIXELS`, which fits both types.
        vkrgss_proto::Rect::new(
            self.left as i32,
            self.top as i32,
            (self.right - self.left) as u32,
            (self.bottom - self.top) as u32,
        )
    }
}

/// An update carrying the pixels of `area` as they are now.
fn update(id: BitmapId, bitmap: &Bitmap, area: Area) -> BitmapDelta {
    let rgba = if area == bitmap.bounds() {
        bitmap.rgba.clone()
    } else {
        let row_len = (area.right - area.left) * 4;
        let mut rgba = Vec::with_capacity(row_len * (area.bottom - area.top));
        for y in area.top..area.bottom {
            let start = (y * bitmap.width + area.left) * 4;
            rgba.extend_from_slice(&bitmap.rgba[start..start + row_len]);
        }
        rgba
    };
    BitmapDelta::Update {
        id,
        rect: area.to_proto(),
        rgba,
    }
}

impl Bitmaps {
    /// The bounding rectangle of the pixels that may have changed since the bitmap's last
    /// update was emitted, or `None` when nothing has. A bitmap starts without one: its first
    /// reference uploads it whole.
    pub fn changed_rect(&self, id: BitmapId) -> Result<Option<vkrgss_proto::Rect>, BitmapError> {
        Ok(self.get(id)?.changed.map(Area::to_proto))
    }

    /// Takes the host to have none of the bitmaps, so that the next [`Bitmaps::deltas`]
    /// create and upload whole every bitmap they are asked for and destroy none. For a host
    /// that starts anew; [`Scene::resend_tables`](crate::Scene::resend_tables) is the same
    /// for the tables of a scene.
    pub fn resend_all(&mut self) {
        self.destroyed.clear();
        for bitmap in self.live_mut() {
            bitmap.known = false;
            bitmap.changed = None;
        }
    }

    /// The bitmap changes the host needs before it draws a frame that refers to `referenced`,
    /// in an order a compositor's bitmap store accepts:
    ///
    /// 1. `Destroy` for every disposed bitmap the host still knows.
    /// 2. For each referenced bitmap the host does not know: `Create`, then `Update` of the
    ///    whole bitmap.
    /// 3. For each referenced bitmap the host knows and that has a changed rectangle: `Update`
    ///    of that rectangle.
    ///
    /// Destroys come first so that a create into a reused slot is never rejected. A bitmap
    /// that is not referenced is not uploaded and keeps its changed rectangle for the request
    /// that does refer to it. Naming a bitmap more than once is the same as naming it once.
    ///
    /// The caller must deliver the result: afterwards the arena takes the host to be up to
    /// date. If `referenced` names a disposed bitmap the call fails with
    /// [`BitmapError::Disposed`], emits nothing and changes nothing.
    pub fn deltas(&mut self, referenced: &[BitmapId]) -> Result<Vec<BitmapDelta>, BitmapError> {
        #[cfg(test)]
        if std::mem::take(&mut self.fail_deltas) {
            return Err(BitmapError::Disposed);
        }
        for &id in referenced {
            self.get(id)?;
        }
        let mut deltas: Vec<BitmapDelta> = self
            .destroyed
            .drain(..)
            .map(|id| BitmapDelta::Destroy { id })
            .collect();

        for &id in referenced {
            let bitmap = self.get_mut(id)?;
            if bitmap.known {
                continue;
            }
            bitmap.known = true;
            bitmap.changed = None;
            deltas.push(BitmapDelta::Create {
                id,
                width: bitmap.width as u32,
                height: bitmap.height as u32,
            });
            deltas.push(update(id, bitmap, bitmap.bounds()));
        }

        for &id in referenced {
            let bitmap = self.get_mut(id)?;
            if let Some(area) = bitmap.changed.take() {
                deltas.push(update(id, bitmap, area));
            }
        }
        Ok(deltas)
    }

    /// The deltas that give a compositor that holds nothing the live bitmaps among
    /// `referenced` as they are now: `Create` and an `Update` of the whole bitmap for each,
    /// once. What the host is taken to have does not change.
    pub(crate) fn whole(&self, referenced: &[BitmapId]) -> Vec<BitmapDelta> {
        let mut seen = HashSet::new();
        let mut deltas = Vec::new();
        for &id in referenced {
            let Ok(bitmap) = self.get(id) else {
                continue;
            };
            if seen.insert(id) {
                deltas.push(BitmapDelta::Create {
                    id,
                    width: bitmap.width as u32,
                    height: bitmap.height as u32,
                });
                deltas.push(update(id, bitmap, bitmap.bounds()));
            }
        }
        deltas
    }
}

/// What a delta counts for beside its pixels: its kind, id and rectangle, and the length of
/// its pixels, as encoded, with room to spare.
const DELTA_OVERHEAD: usize = 64;

/// The bytes `delta` counts for in a part of [`split_deltas`].
fn cost(delta: &BitmapDelta) -> usize {
    match delta {
        BitmapDelta::Update { rgba, .. } => rgba.len() + DELTA_OVERHEAD,
        BitmapDelta::Create { .. } | BitmapDelta::Destroy { .. } => DELTA_OVERHEAD,
    }
}

/// The parts being made, and where a finished one goes.
struct Parts<F> {
    limit: usize,
    part: Vec<BitmapDelta>,
    bytes: usize,
    send: F,
}

impl<F: FnMut(Vec<BitmapDelta>)> Parts<F> {
    fn push(&mut self, delta: BitmapDelta) {
        let cost = cost(&delta);
        if self.bytes + cost > self.limit && !self.part.is_empty() {
            self.flush();
        }
        self.bytes += cost;
        self.part.push(delta);
    }

    fn flush(&mut self) {
        if !self.part.is_empty() {
            (self.send)(std::mem::take(&mut self.part));
            self.bytes = 0;
        }
    }
}

/// Hands `deltas` to `send` in parts, in order: concatenated, the parts are `deltas` with
/// every `Update` that alone passes `limit` cut into updates of bands of whole rows, or of
/// pieces of one row where a row alone passes it. Applied in order, the parts change a
/// bitmap store as `deltas` do.
///
/// Each part counts at most `limit` bytes: the pixels of its updates and 64 bytes for each
/// delta, which is more than the encoding of a delta adds to its pixels. Only one band is
/// copied at a time, so the pixels are held at most once more than in `deltas` while the
/// parts are sent.
///
/// # Panics
///
/// If `limit` leaves no room for the pixel of an update: it is below 68.
pub fn split_deltas(deltas: Vec<BitmapDelta>, limit: usize, send: impl FnMut(Vec<BitmapDelta>)) {
    let room = limit
        .checked_sub(DELTA_OVERHEAD)
        .filter(|room| *room >= 4)
        .expect("a part has room for a pixel");
    let mut parts = Parts {
        limit,
        part: Vec::new(),
        bytes: 0,
        send,
    };
    // The offsets of bands and pieces are inside the rectangle of the update, whose sides fit
    // `i32`.
    for delta in deltas {
        match delta {
            BitmapDelta::Update { id, rect, rgba } if rgba.len() > room => {
                let row = rect.width as usize * 4;
                if row <= room {
                    // Bands of as many rows as fit.
                    let rows = room / row;
                    for (band, pixels) in (0usize..).step_by(rows).zip(rgba.chunks(rows * row)) {
                        parts.push(BitmapDelta::Update {
                            id,
                            rect: Rect::new(
                                rect.x,
                                rect.y + band as i32,
                                rect.width,
                                (pixels.len() / row) as u32,
                            ),
                            rgba: pixels.to_vec(),
                        });
                    }
                } else {
                    // Pieces of each row.
                    let columns = room / 4;
                    for (line, pixels) in (0usize..).zip(rgba.chunks(row)) {
                        for (piece, pixels) in
                            (0usize..).step_by(columns).zip(pixels.chunks(columns * 4))
                        {
                            parts.push(BitmapDelta::Update {
                                id,
                                rect: Rect::new(
                                    rect.x + piece as i32,
                                    rect.y + line as i32,
                                    (pixels.len() / 4) as u32,
                                    1,
                                ),
                                rgba: pixels.to_vec(),
                            });
                        }
                    }
                }
            }
            delta => parts.push(delta),
        }
    }
    parts.flush();
}

#[cfg(test)]
mod tests;
