//! Bitmap deltas: how the pixels of the arena's bitmaps reach the host.
//!
//! Every drawing operation records the bounding rectangle of the pixels it may have changed.
//! [`Bitmaps::deltas`] turns that, for the bitmaps a frame refers to, into the smallest list of
//! [`BitmapDelta`]s that brings a compositor's bitmap store up to date.

use vkxp_proto::{BitmapDelta, BitmapId};

use crate::bitmap::{Area, Bitmap};
use crate::{BitmapError, Bitmaps};

impl Area {
    /// The area as a protocol rectangle.
    fn to_proto(self) -> vkxp_proto::Rect {
        // Bitmap sides are at most `MAX_BITMAP_PIXELS`, which fits both types.
        vkxp_proto::Rect::new(
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
    pub fn changed_rect(&self, id: BitmapId) -> Result<Option<vkxp_proto::Rect>, BitmapError> {
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
}

#[cfg(test)]
mod tests;
