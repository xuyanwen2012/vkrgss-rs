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
        // With a positive divisor, Euclidean division rounds towards negative infinity, so
        // move the sign of the denominator to the numerator first.
        let sign = i64::from(denominator.signum());
        let scaled = sign * (i64::from(numerator) << Self::FRAC_BITS);
        let divisor = sign * i64::from(denominator);
        i32::try_from(scaled.div_euclid(divisor)).ok().map(Fixed)
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

    /// floor(n * 65536 / d), computed independently in 128-bit arithmetic.
    fn floor_ratio_bits(numerator: i32, denominator: i32) -> i128 {
        let (n, d) = (i128::from(numerator) * 65536, i128::from(denominator));
        let quotient = n / d;
        // `/` truncates towards zero; step down when it rounded a negative result up.
        if n % d != 0 && ((n < 0) != (d < 0)) {
            quotient - 1
        } else {
            quotient
        }
    }

    #[test]
    fn inexact_ratios_round_down_for_every_sign_combination() {
        // 1/3 = 0.33333..., which is 21845.33... in units of 1/65536.
        assert_eq!(Fixed::from_ratio(1, 3), Some(Fixed::from_bits(21845)));
        assert_eq!(Fixed::from_ratio(-1, 3), Some(Fixed::from_bits(-21846)));
        assert_eq!(Fixed::from_ratio(1, -3), Some(Fixed::from_bits(-21846)));
        assert_eq!(Fixed::from_ratio(-1, -3), Some(Fixed::from_bits(21845)));
        // 7/5 = 1.4, which is 91750.4 in units of 1/65536.
        assert_eq!(Fixed::from_ratio(7, 5), Some(Fixed::from_bits(91750)));
        assert_eq!(Fixed::from_ratio(-7, 5), Some(Fixed::from_bits(-91751)));
        assert_eq!(Fixed::from_ratio(7, -5), Some(Fixed::from_bits(-91751)));
        assert_eq!(Fixed::from_ratio(-7, -5), Some(Fixed::from_bits(91750)));
    }

    #[test]
    fn exact_ratios_are_exact_for_every_sign_combination() {
        assert_eq!(Fixed::from_ratio(-1, 2), Some(Fixed::from_bits(-32768)));
        assert_eq!(Fixed::from_ratio(-1, -2), Some(Fixed::HALF));
        assert_eq!(Fixed::from_ratio(6, -2), Some(Fixed::from_int(-3)));
        assert_eq!(Fixed::from_ratio(0, -7), Some(Fixed::ZERO));
        assert_eq!(Fixed::from_ratio(0, 7), Some(Fixed::ZERO));
    }

    #[test]
    fn ratios_match_floor_division_across_signs_and_magnitudes() {
        let values = [
            i32::MIN,
            i32::MIN + 1,
            -65537,
            -65536,
            -32768,
            -1000,
            -7,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            7,
            1000,
            32767,
            65536,
            65537,
            i32::MAX - 1,
            i32::MAX,
        ];
        for numerator in values {
            for denominator in values {
                if denominator == 0 {
                    assert_eq!(Fixed::from_ratio(numerator, denominator), None);
                    continue;
                }
                let expected = i32::try_from(floor_ratio_bits(numerator, denominator))
                    .ok()
                    .map(Fixed::from_bits);
                assert_eq!(
                    Fixed::from_ratio(numerator, denominator),
                    expected,
                    "{numerator} / {denominator}"
                );
            }
        }
    }

    #[test]
    fn ratios_at_the_range_boundaries() {
        let min = Fixed::from_bits(i32::MIN);
        let max = Fixed::from_bits(i32::MAX);
        // The lowest value, -32768, is representable; +32768 is not.
        assert_eq!(Fixed::from_ratio(-32768, 1), Some(min));
        assert_eq!(Fixed::from_ratio(32768, -1), Some(min));
        assert_eq!(Fixed::from_ratio(32768, 1), None);
        assert_eq!(Fixed::from_ratio(-32768, -1), None);
        assert_eq!(Fixed::from_ratio(-32769, 1), None);
        assert_eq!(Fixed::from_ratio(32769, -1), None);
        assert_eq!(Fixed::from_ratio(32767, 1), Some(Fixed::from_int(i16::MAX)));
        assert_eq!(
            Fixed::from_ratio(-32767, -1),
            Some(Fixed::from_int(i16::MAX))
        );
        // Raw extremes, reached through a denominator of 65536.
        assert_eq!(Fixed::from_ratio(i32::MAX, 65536), Some(max));
        assert_eq!(Fixed::from_ratio(i32::MIN, 65536), Some(min));
        assert_eq!(Fixed::from_ratio(i32::MIN, -65536), None);
        assert_eq!(Fixed::from_ratio(i32::MIN + 1, -65536), Some(max));
        // Extreme operands that would overflow 32-bit negation.
        assert_eq!(Fixed::from_ratio(i32::MIN, i32::MIN), Some(Fixed::ONE));
        assert_eq!(Fixed::from_ratio(i32::MIN, -1), None);
        assert_eq!(Fixed::from_ratio(i32::MIN, 1), None);
        // 1 / i32::MIN is a tiny negative number, which rounds down to -1/65536.
        assert_eq!(Fixed::from_ratio(1, i32::MIN), Some(Fixed::from_bits(-1)));
        assert_eq!(Fixed::from_ratio(-1, i32::MIN), Some(Fixed::ZERO));
        assert_eq!(Fixed::from_ratio(1, i32::MAX), Some(Fixed::ZERO));
        assert_eq!(Fixed::from_ratio(-1, i32::MAX), Some(Fixed::from_bits(-1)));
    }

    #[test]
    fn ordering_follows_the_value() {
        assert!(Fixed::from_int(-1) < Fixed::ZERO);
        assert!(Fixed::HALF < Fixed::ONE);
    }
}
