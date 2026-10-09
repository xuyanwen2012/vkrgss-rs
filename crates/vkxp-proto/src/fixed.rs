//! Fixed-point numbers for coordinates and transforms.

use serde::{Deserialize, Serialize};

/// A signed fixed-point number with 16 fractional bits, stored in an `i32`.
///
/// The representable range is -32768 up to just under 32768, in steps of 1/65536.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Fixed(i32);

impl Fixed {
    /// Number of fractional bits.
    pub const FRAC_BITS: u32 = 16;
    /// The value 0.
    pub const ZERO: Fixed = Fixed(0);
    /// The value 1.
    pub const ONE: Fixed = Fixed(1 << Self::FRAC_BITS);
    /// The value 1/2.
    pub const HALF: Fixed = Fixed(1 << (Self::FRAC_BITS - 1));

    /// Builds a value from its raw representation, `value * 65536`.
    pub const fn from_bits(bits: i32) -> Fixed {
        Fixed(bits)
    }

    /// The raw representation, `value * 65536`.
    pub const fn to_bits(self) -> i32 {
        self.0
    }

    /// Converts an integer. Every `i16` is representable.
    pub const fn from_int(value: i16) -> Fixed {
        Fixed((value as i32) << Self::FRAC_BITS)
    }

    /// Converts an integer, or returns `None` if it is outside the representable range.
    pub const fn try_from_int(value: i32) -> Option<Fixed> {
        if value < i16::MIN as i32 || value > i16::MAX as i32 {
            None
        } else {
            Some(Fixed(value << Self::FRAC_BITS))
        }
    }

    /// The fraction `numerator / denominator`, rounded towards negative infinity to the nearest
    /// representable value. Returns `None` for a zero denominator or a result out of range.
    pub fn from_ratio(numerator: i32, denominator: i32) -> Option<Fixed> {
        if denominator == 0 {
            return None;
        }
        let scaled = i64::from(numerator) << Self::FRAC_BITS;
        i32::try_from(scaled.div_euclid(i64::from(denominator)))
            .ok()
            .map(Fixed)
    }

    /// The largest integer not greater than the value.
    pub const fn floor(self) -> i32 {
        self.0 >> Self::FRAC_BITS
    }

    /// The fractional part in units of 1/65536, always in `0..65536`.
    pub const fn fract_bits(self) -> u32 {
        (self.0 & 0xFFFF) as u32
    }
}

impl From<i16> for Fixed {
    fn from(value: i16) -> Fixed {
        Fixed::from_int(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_have_expected_bits() {
        assert_eq!(Fixed::ZERO.to_bits(), 0);
        assert_eq!(Fixed::ONE.to_bits(), 65536);
        assert_eq!(Fixed::HALF.to_bits(), 32768);
        assert_eq!(Fixed::default(), Fixed::ZERO);
    }

    #[test]
    fn integers_convert_exactly() {
        for value in [i16::MIN, -640, -1, 0, 1, 480, i16::MAX] {
            let fixed = Fixed::from_int(value);
            assert_eq!(fixed.to_bits(), i32::from(value) * 65536);
            assert_eq!(fixed.floor(), i32::from(value));
            assert_eq!(fixed.fract_bits(), 0);
            assert_eq!(Fixed::from(value), fixed);
            assert_eq!(Fixed::try_from_int(i32::from(value)), Some(fixed));
        }
    }

    #[test]
    fn out_of_range_integers_are_rejected() {
        assert_eq!(Fixed::try_from_int(32768), None);
        assert_eq!(Fixed::try_from_int(-32769), None);
        assert_eq!(Fixed::try_from_int(i32::MAX), None);
    }

    #[test]
    fn bits_round_trip() {
        for bits in [i32::MIN, -1, 0, 1, 0x0001_8000, i32::MAX] {
            assert_eq!(Fixed::from_bits(bits).to_bits(), bits);
        }
    }

    #[test]
    fn floor_rounds_towards_negative_infinity() {
        assert_eq!(Fixed::from_bits(0x0001_8000).floor(), 1); // 1.5
        assert_eq!(Fixed::from_bits(-0x0001_8000).floor(), -2); // -1.5
        assert_eq!(Fixed::from_bits(-1).floor(), -1);
        assert_eq!(Fixed::from_bits(-0x0001_8000).fract_bits(), 0x8000);
    }

    #[test]
    fn ratios_convert_and_round_down() {
        assert_eq!(Fixed::from_ratio(1, 2), Some(Fixed::HALF));
        assert_eq!(Fixed::from_ratio(3, 1), Some(Fixed::from_int(3)));
        assert_eq!(Fixed::from_ratio(1, 3), Some(Fixed::from_bits(21845)));
        assert_eq!(Fixed::from_ratio(-1, 3), Some(Fixed::from_bits(-21846)));
        assert_eq!(Fixed::from_ratio(1, -2), Some(Fixed::from_bits(-32768)));
        assert_eq!(Fixed::from_ratio(1, 0), None);
        assert_eq!(Fixed::from_ratio(32768, 1), None);
    }

    #[test]
    fn ordering_follows_the_value() {
        assert!(Fixed::from_int(-1) < Fixed::ZERO);
        assert!(Fixed::HALF < Fixed::ONE);
    }
}
