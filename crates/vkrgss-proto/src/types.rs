//! Small value types used throughout the protocol.

use serde::{Deserialize, Serialize};

/// A width and height in whole pixels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub const fn new(width: u32, height: u32) -> Size {
        Size { width, height }
    }
}

/// An axis-aligned rectangle in whole pixels. It covers `x..x + width` by `y..y + height`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// The rectangle at the origin with the given size.
    pub const fn from_size(size: Size) -> Rect {
        Rect::new(0, 0, size.width, size.height)
    }

    /// Whether the rectangle covers no pixel.
    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Whether the pixel at (`x`, `y`) is inside the rectangle.
    pub const fn contains(&self, x: i32, y: i32) -> bool {
        let (x, y) = (x as i64, y as i64);
        let (left, top) = (self.x as i64, self.y as i64);
        x >= left && x < left + self.width as i64 && y >= top && y < top + self.height as i64
    }

    /// Whether every pixel of `other` is inside this rectangle. An empty `other` always is.
    pub const fn contains_rect(&self, other: &Rect) -> bool {
        if other.is_empty() {
            return true;
        }
        let (left, top) = (self.x as i64, self.y as i64);
        let (other_left, other_top) = (other.x as i64, other.y as i64);
        other_left >= left
            && other_top >= top
            && other_left + other.width as i64 <= left + self.width as i64
            && other_top + other.height as i64 <= top + self.height as i64
    }
}

/// A colour with straight (non-premultiplied) alpha, 0 to 255 per channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    /// The channels in red, green, blue, alpha order.
    pub const fn to_rgba(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub const fn from_rgba(rgba: [u8; 4]) -> Color {
        Color::new(rgba[0], rgba[1], rgba[2], rgba[3])
    }
}

/// A tone: signed offsets added to red, green and blue, and an amount of desaturation.
///
/// The offsets are meaningful from -255 to 255 and `gray` from 0 (none) to 255 (full).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tone {
    pub red: i16,
    pub green: i16,
    pub blue: i16,
    pub gray: u8,
}

impl Tone {
    /// The tone that changes nothing.
    pub const NEUTRAL: Tone = Tone::new(0, 0, 0, 0);

    pub const fn new(red: i16, green: i16, blue: i16, gray: u8) -> Tone {
        Tone {
            red,
            green,
            blue,
            gray,
        }
    }
}

/// Identifies a bitmap known to both sides.
///
/// `index` names a slot and `generation` names one occupant of that slot. The guest gives every
/// new bitmap in a slot a generation it has not used for that slot before, so a reference kept
/// after the bitmap was destroyed never equals the id of a later occupant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BitmapId {
    pub index: u32,
    pub generation: u32,
}

impl BitmapId {
    pub const fn new(index: u32, generation: u32) -> BitmapId {
        BitmapId { index, generation }
    }
}

impl std::fmt::Display for BitmapId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}v{}", self.index, self.generation)
    }
}

/// Identifies a table known to both sides. Tables have a number space of their own, apart
/// from the bitmaps'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TableId(pub u32);

impl TableId {
    pub const fn new(index: u32) -> TableId {
        TableId(index)
    }
}

impl std::fmt::Display for TableId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmap_ids_are_equal_only_with_same_index_and_generation() {
        let id = BitmapId::new(7, 3);
        assert_eq!(id, BitmapId::new(7, 3));
        assert_ne!(id, BitmapId::new(7, 4), "same slot, later occupant");
        assert_ne!(id, BitmapId::new(8, 3), "other slot, same generation");
        assert_eq!(id.to_string(), "7v3");
    }

    #[test]
    fn bitmap_ids_hash_by_both_parts() {
        let ids: std::collections::HashSet<BitmapId> = [
            BitmapId::new(1, 0),
            BitmapId::new(1, 1),
            BitmapId::new(2, 0),
            BitmapId::new(1, 0),
        ]
        .into();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn rect_contains_its_pixels_only() {
        let rect = Rect::new(-2, 3, 4, 2);
        assert!(rect.contains(-2, 3));
        assert!(rect.contains(1, 4));
        assert!(!rect.contains(2, 4), "right edge is exclusive");
        assert!(!rect.contains(1, 5), "bottom edge is exclusive");
        assert!(!rect.contains(-3, 3));
        assert!(!Rect::new(0, 0, 0, 5).contains(0, 0));
    }

    #[test]
    fn rect_containment_does_not_overflow() {
        let rect = Rect::new(i32::MAX - 1, i32::MIN, u32::MAX, u32::MAX);
        assert!(rect.contains(i32::MAX, 0));
        assert!(!rect.contains(0, 0));
    }

    #[test]
    fn rect_contains_rect() {
        let outer = Rect::new(0, 0, 10, 10);
        assert!(outer.contains_rect(&Rect::new(0, 0, 10, 10)));
        assert!(outer.contains_rect(&Rect::new(9, 9, 1, 1)));
        assert!(!outer.contains_rect(&Rect::new(9, 9, 2, 1)));
        assert!(!outer.contains_rect(&Rect::new(-1, 0, 2, 2)));
        assert!(outer.contains_rect(&Rect::new(50, 50, 0, 0)));
    }

    #[test]
    fn color_converts_to_and_from_rgba() {
        let color = Color::new(1, 2, 3, 4);
        assert_eq!(color.to_rgba(), [1, 2, 3, 4]);
        assert_eq!(Color::from_rgba([1, 2, 3, 4]), color);
    }

    #[test]
    fn neutral_tone_is_all_zero() {
        assert_eq!(Tone::NEUTRAL, Tone::default());
    }
}
