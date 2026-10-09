//! `Rect`: four integers stored exactly as given.

use crate::DecodeError;

/// Length of a serialized rectangle: four little-endian `i32`.
const SERIALIZED_LEN: usize = 16;

/// A rectangle as scripts see it: x, y, width and height, stored as given. Width and height
/// may be zero or negative; such a rectangle covers no pixel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    /// Sets all four values.
    pub fn set(&mut self, x: i32, y: i32, width: i32, height: i32) {
        *self = Rect::new(x, y, width, height);
    }

    /// Sets all four values to 0.
    pub fn empty(&mut self) {
        *self = Rect::default();
    }

    /// The rectangle in protocol form. A rectangle whose width or height is 0 or less covers
    /// no pixel and becomes an empty protocol rectangle (width and height 0) at its position.
    pub fn to_proto(self) -> vkxp_proto::Rect {
        if self.width <= 0 || self.height <= 0 {
            vkxp_proto::Rect::new(self.x, self.y, 0, 0)
        } else {
            vkxp_proto::Rect::new(
                self.x,
                self.y,
                self.width.unsigned_abs(),
                self.height.unsigned_abs(),
            )
        }
    }

    /// The serialized form: x, y, width and height as little-endian `i32`, 16 bytes.
    ///
    /// PLACEHOLDER: the commonly documented layout, not yet checked against a data file of the
    /// original runtime.
    pub fn to_bytes(&self) -> [u8; SERIALIZED_LEN] {
        let mut bytes = [0; SERIALIZED_LEN];
        let values = [self.x, self.y, self.width, self.height];
        for (chunk, value) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(values) {
            *chunk = value.to_le_bytes();
        }
        bytes
    }

    /// Restores a rectangle from its serialized form.
    pub fn from_bytes(bytes: &[u8]) -> Result<Rect, DecodeError> {
        if bytes.len() != SERIALIZED_LEN {
            return Err(DecodeError::Length {
                what: "rectangle",
                expected: SERIALIZED_LEN,
                found: bytes.len(),
            });
        }
        let mut values = [0; 4];
        for (value, chunk) in values.iter_mut().zip(bytes.as_chunks::<4>().0) {
            *value = i32::from_le_bytes(*chunk);
        }
        let [x, y, width, height] = values;
        Ok(Rect::new(x, y, width, height))
    }
}

impl From<Rect> for vkxp_proto::Rect {
    fn from(rect: Rect) -> vkxp_proto::Rect {
        rect.to_proto()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn values_are_stored_as_given() {
        let mut rect = Rect::default();
        rect.set(-10, 20, -30, 0);
        assert_eq!((rect.x, rect.y, rect.width, rect.height), (-10, 20, -30, 0));
        assert_eq!(rect, Rect::new(-10, 20, -30, 0));

        let extreme = Rect::new(i32::MIN, i32::MAX, i32::MIN, i32::MAX);
        assert_eq!(
            (extreme.x, extreme.y, extreme.width, extreme.height),
            (i32::MIN, i32::MAX, i32::MIN, i32::MAX)
        );
    }

    #[test]
    fn emptying_zeroes_all_four() {
        let mut rect = Rect::new(1, -2, 3, -4);
        rect.empty();
        assert_eq!((rect.x, rect.y, rect.width, rect.height), (0, 0, 0, 0));
    }

    #[test]
    fn equality_compares_all_four() {
        let rect = Rect::new(1, 2, 3, 4);
        assert_eq!(rect, Rect::new(1, 2, 3, 4));
        assert_ne!(rect, Rect::new(0, 2, 3, 4));
        assert_ne!(rect, Rect::new(1, 0, 3, 4));
        assert_ne!(rect, Rect::new(1, 2, 0, 4));
        assert_ne!(rect, Rect::new(1, 2, 3, 0));
    }

    #[test]
    fn positive_sizes_convert_unchanged() {
        assert_eq!(
            Rect::new(-5, 6, 7, 8).to_proto(),
            vkxp_proto::Rect::new(-5, 6, 7, 8)
        );
        assert_eq!(
            vkxp_proto::Rect::from(Rect::new(0, 0, i32::MAX, 1)),
            vkxp_proto::Rect::new(0, 0, i32::MAX as u32, 1)
        );
    }

    #[test]
    fn non_positive_sizes_convert_to_an_empty_rectangle() {
        for rect in [
            Rect::new(1, 2, -3, 4),
            Rect::new(1, 2, 3, -4),
            Rect::new(1, 2, 0, 4),
            Rect::new(1, 2, 3, 0),
            Rect::new(1, 2, -3, -4),
            Rect::new(1, 2, i32::MIN, i32::MIN),
        ] {
            let converted = rect.to_proto();
            assert!(converted.is_empty(), "{rect:?}");
            assert_eq!(converted, vkxp_proto::Rect::new(1, 2, 0, 0), "{rect:?}");
        }
    }

    #[test]
    fn serialized_values_are_little_endian_in_order() {
        assert_eq!(
            Rect::new(1, -2, 0x0403_0201, 4).to_bytes(),
            [1, 0, 0, 0, 0xFE, 0xFF, 0xFF, 0xFF, 1, 2, 3, 4, 4, 0, 0, 0]
        );
    }

    #[test]
    fn wrong_lengths_fail() {
        for len in [0, 15, 17, 32] {
            assert_eq!(
                Rect::from_bytes(&vec![0; len]),
                Err(DecodeError::Length {
                    what: "rectangle",
                    expected: 16,
                    found: len,
                })
            );
        }
    }

    proptest! {
        #[test]
        fn every_rectangle_round_trips(
            x in any::<i32>(),
            y in any::<i32>(),
            width in any::<i32>(),
            height in any::<i32>(),
        ) {
            let rect = Rect::new(x, y, width, height);
            prop_assert_eq!(Rect::from_bytes(&rect.to_bytes()), Ok(rect));
        }
    }
}
