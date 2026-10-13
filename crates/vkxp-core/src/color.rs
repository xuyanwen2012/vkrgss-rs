//! `Color` and `Tone`: floating-point components that clamp whenever they are set, at both
//! bounds, as measured on the original runtime.

use crate::DecodeError;

/// Length of a serialized colour or tone: four little-endian `f64`.
const SERIALIZED_LEN: usize = 32;

/// Clamps a component to `min..=max`. Not-a-number becomes 0, which lies in every range used
/// here, so a stored component is always a number inside its range.
fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(min, max)
    }
}

/// Four components as little-endian `f64`, in the order given.
fn components_to_bytes(components: [f64; 4]) -> [u8; SERIALIZED_LEN] {
    let mut bytes = [0; SERIALIZED_LEN];
    for (chunk, component) in bytes.as_chunks_mut::<8>().0.iter_mut().zip(components) {
        *chunk = component.to_le_bytes();
    }
    bytes
}

/// The inverse of [`components_to_bytes`]; fails on any other length.
fn components_from_bytes(what: &'static str, bytes: &[u8]) -> Result<[f64; 4], DecodeError> {
    if bytes.len() != SERIALIZED_LEN {
        return Err(DecodeError::Length {
            what,
            expected: SERIALIZED_LEN,
            found: bytes.len(),
        });
    }
    let mut components = [0.0; 4];
    for (component, chunk) in components.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *component = f64::from_le_bytes(*chunk);
    }
    Ok(components)
}

/// A colour: red, green and blue from -255 to 255 and alpha from 0 to 255, with fractions
/// kept. As in the original runtime a negative red, green or blue is kept as it is, and only
/// where the colour is used does it become a channel ([`Color::to_pixel`],
/// [`Color::to_proto`]).
///
/// Two colours are equal exactly when all four components are equal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

impl Color {
    /// The colour with all four components 0.
    pub const TRANSPARENT: Color = Color {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.0,
    };

    /// A colour from four components, red, green and blue clamped to -255 to 255 and alpha to
    /// 0 to 255.
    pub fn new(red: f64, green: f64, blue: f64, alpha: f64) -> Color {
        let mut color = Color::TRANSPARENT;
        color.set(red, green, blue, alpha);
        color
    }

    /// An opaque colour: alpha is 255.
    pub fn rgb(red: f64, green: f64, blue: f64) -> Color {
        Color::new(red, green, blue, 255.0)
    }

    pub fn red(&self) -> f64 {
        self.red
    }

    pub fn green(&self) -> f64 {
        self.green
    }

    pub fn blue(&self) -> f64 {
        self.blue
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    pub fn set_red(&mut self, red: f64) {
        self.red = clamp(red, -255.0, 255.0);
    }

    pub fn set_green(&mut self, green: f64) {
        self.green = clamp(green, -255.0, 255.0);
    }

    pub fn set_blue(&mut self, blue: f64) {
        self.blue = clamp(blue, -255.0, 255.0);
    }

    pub fn set_alpha(&mut self, alpha: f64) {
        self.alpha = clamp(alpha, 0.0, 255.0);
    }

    /// Sets all four components, clamping each.
    pub fn set(&mut self, red: f64, green: f64, blue: f64, alpha: f64) {
        self.set_red(red);
        self.set_green(green);
        self.set_blue(blue);
        self.set_alpha(alpha);
    }

    /// The colour as a stored pixel, as `fill_rect`, `set_pixel` and text drawing write it:
    /// red, green and blue truncated toward zero and taken modulo 256, alpha truncated. Red -5
    /// is stored as 251 and red -0.5 as 0.
    pub fn to_pixel(self) -> [u8; 4] {
        let wrap = |component: f64| (component as i64).rem_euclid(256) as u8;
        [
            wrap(self.red),
            wrap(self.green),
            wrap(self.blue),
            self.alpha as u8,
        ]
    }

    /// The colour as integers for the protocol: every component with its fraction discarded
    /// (toward zero), then red, green and blue taken modulo 256 and a negative alpha taken
    /// as 0, as in [`Color::to_pixel`]. Red -1 is sent as 255, red -100.7 as 156 and red -0.5
    /// as 0. This is the colour sent to the host.
    ///
    /// Provenance: oracle measurement, RGSS103J, fraction probe
    /// (`ph-scene-fraction-probe.rb`: 936 blocks of colours, tones and flashes of sprites,
    /// viewports and planes, all consistent with this rule).
    pub fn to_proto(self) -> vkxp_proto::Color {
        let [r, g, b, a] = self.to_pixel();
        vkxp_proto::Color::new(r, g, b, a)
    }

    /// The colour with the integer components of a protocol colour.
    pub fn from_proto(color: vkxp_proto::Color) -> Color {
        Color {
            red: f64::from(color.r),
            green: f64::from(color.g),
            blue: f64::from(color.b),
            alpha: f64::from(color.a),
        }
    }

    /// The serialized form: red, green, blue and alpha as little-endian `f64`, 32 bytes.
    pub fn to_bytes(&self) -> [u8; SERIALIZED_LEN] {
        components_to_bytes([self.red, self.green, self.blue, self.alpha])
    }

    /// Restores a colour from its serialized form, clamping each component.
    pub fn from_bytes(bytes: &[u8]) -> Result<Color, DecodeError> {
        let [red, green, blue, alpha] = components_from_bytes("colour", bytes)?;
        Ok(Color::new(red, green, blue, alpha))
    }
}

impl From<Color> for vkxp_proto::Color {
    fn from(color: Color) -> vkxp_proto::Color {
        color.to_proto()
    }
}

/// A tone: offsets for red, green and blue from -255 to 255 and a grey amount from 0 to 255,
/// with fractions kept.
///
/// Two tones are equal exactly when all four components are equal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    red: f64,
    green: f64,
    blue: f64,
    gray: f64,
}

impl Tone {
    /// The tone that changes nothing.
    pub const NEUTRAL: Tone = Tone {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        gray: 0.0,
    };

    /// A tone from four components, the offsets clamped to -255 to 255 and the grey amount to
    /// 0 to 255.
    pub fn new(red: f64, green: f64, blue: f64, gray: f64) -> Tone {
        let mut tone = Tone::NEUTRAL;
        tone.set(red, green, blue, gray);
        tone
    }

    /// A tone without desaturation: the grey amount is 0.
    pub fn rgb(red: f64, green: f64, blue: f64) -> Tone {
        Tone::new(red, green, blue, 0.0)
    }

    pub fn red(&self) -> f64 {
        self.red
    }

    pub fn green(&self) -> f64 {
        self.green
    }

    pub fn blue(&self) -> f64 {
        self.blue
    }

    pub fn gray(&self) -> f64 {
        self.gray
    }

    pub fn set_red(&mut self, red: f64) {
        self.red = clamp(red, -255.0, 255.0);
    }

    pub fn set_green(&mut self, green: f64) {
        self.green = clamp(green, -255.0, 255.0);
    }

    pub fn set_blue(&mut self, blue: f64) {
        self.blue = clamp(blue, -255.0, 255.0);
    }

    pub fn set_gray(&mut self, gray: f64) {
        self.gray = clamp(gray, 0.0, 255.0);
    }

    /// Sets all four components, clamping each.
    pub fn set(&mut self, red: f64, green: f64, blue: f64, gray: f64) {
        self.set_red(red);
        self.set_green(green);
        self.set_blue(blue);
        self.set_gray(gray);
    }

    /// The tone as integers, each component with its fraction discarded (toward zero), without
    /// wrapping. This is the tone sent to the host.
    ///
    /// Provenance: oracle measurement, RGSS103J, fraction probe
    /// (`ph-scene-fraction-probe.rb`: 936 blocks, all consistent with truncation).
    pub fn to_proto(self) -> vkxp_proto::Tone {
        // Components are within their ranges, where `as` discards the fraction.
        vkxp_proto::Tone::new(
            self.red as i16,
            self.green as i16,
            self.blue as i16,
            self.gray as u8,
        )
    }

    /// The serialized form: red, green, blue and grey as little-endian `f64`, 32 bytes.
    pub fn to_bytes(&self) -> [u8; SERIALIZED_LEN] {
        components_to_bytes([self.red, self.green, self.blue, self.gray])
    }

    /// Restores a tone from its serialized form, clamping each component.
    pub fn from_bytes(bytes: &[u8]) -> Result<Tone, DecodeError> {
        let [red, green, blue, gray] = components_from_bytes("tone", bytes)?;
        Ok(Tone::new(red, green, blue, gray))
    }
}

impl From<Tone> for vkxp_proto::Tone {
    fn from(tone: Tone) -> vkxp_proto::Tone {
        tone.to_proto()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn colour_components_are_clamped() {
        let color = Color::new(300.0, -300.0, 0.1, -7.0);
        assert_eq!(
            (color.red(), color.green(), color.blue(), color.alpha()),
            (255.0, -255.0, 0.1, 0.0)
        );

        let mut color = Color::rgb(1.0, 2.0, 3.0);
        color.set_red(1000.0);
        color.set_green(-1000.0);
        color.set_blue(-1e9);
        color.set_alpha(-3.5);
        assert_eq!(
            (color.red(), color.green(), color.blue(), color.alpha()),
            (255.0, -255.0, -255.0, 0.0)
        );
        color.set(256.0, -255.5, 255.5, f64::INFINITY);
        assert_eq!(
            (color.red(), color.green(), color.blue(), color.alpha()),
            (255.0, -255.0, 255.0, 255.0)
        );
    }

    /// Measured on the originals of XP and VX on 2026-10-12.
    #[test]
    fn the_lower_bound_of_a_colour_is_minus_255() {
        let mut color = Color::new(-300.0, 0.0, 0.0, -5.0);
        assert_eq!((color.red(), color.alpha()), (-255.0, 0.0));
        color.set_red(-1000.0);
        assert_eq!((color.red(), color.alpha()), (-255.0, 0.0));
        let color = Color::new(-300.0, 300.0, 7.6, -7.0);
        assert_eq!(
            (color.red(), color.green(), color.blue(), color.alpha()),
            (-255.0, 255.0, 7.6, 0.0)
        );
    }

    #[test]
    fn negative_colour_components_are_kept() {
        let mut color = Color::rgb(0.0, -5.0, 0.0);
        assert_eq!(color.green(), -5.0);
        color.set_green(-0.5);
        color.set_blue(-255.0);
        assert_eq!((color.green(), color.blue()), (-0.5, -255.0));
    }

    #[test]
    fn stored_pixel_wraps_colour_and_truncates_alpha() {
        assert_eq!(
            Color::new(-5.0, 0.0, 0.0, 255.0).to_pixel(),
            [251, 0, 0, 255]
        );
        assert_eq!(Color::new(0.0, 0.0, 0.0, -5.0).to_pixel(), [0, 0, 0, 0]);
        assert_eq!(Color::new(7.6, 7.4, 7.5, 128.5).to_pixel(), [7, 7, 7, 128]);
        assert_eq!(
            Color::new(0.9, 255.9, 256.0, 254.9).to_pixel(),
            [0, 255, 255, 254]
        );
        assert_eq!(
            Color::new(-0.5, -1.0, -0.9, -1.5).to_pixel(),
            [0, 255, 0, 0]
        );
        assert_eq!(
            Color::new(-255.0, -254.5, -511.5, 0.0).to_pixel(),
            [1, 2, 1, 0]
        );
    }

    #[test]
    fn protocol_colour_wraps_negative_red_green_and_blue() {
        let color = Color::new(-5.0, 300.0, -0.5, -1.0);
        assert_eq!(color.to_proto(), vkxp_proto::Color::new(251, 255, 0, 0));
        // The measured values.
        for (red, sent) in [(-1.0, 255), (-1.5, 255), (-5.0, 251), (-100.7, 156)] {
            assert_eq!(Color::rgb(red, 0.0, 0.0).to_proto().r, sent, "{red}");
            assert_eq!(Color::rgb(0.0, red, 0.0).to_proto().g, sent, "{red}");
            assert_eq!(Color::rgb(0.0, 0.0, red).to_proto().b, sent, "{red}");
        }
        for red in [-0.5, -0.9] {
            assert_eq!(Color::rgb(red, 0.0, 0.0).to_proto().r, 0, "{red}");
        }
        for alpha in [-1.0, -1.5, -5.0, -100.7] {
            assert_eq!(Color::new(0.0, 0.0, 0.0, alpha).to_proto().a, 0);
        }
    }

    #[test]
    fn colour_fractions_are_kept() {
        let mut color = Color::rgb(0.0, 0.0, 12.5);
        assert_eq!(color.blue(), 12.5);
        color.set_blue(200.25);
        assert_eq!(color.blue(), 200.25);
    }

    #[test]
    fn colour_default_alpha_is_255() {
        assert_eq!(Color::rgb(1.0, 2.0, 3.0).alpha(), 255.0);
        assert_eq!(Color::rgb(1.0, 2.0, 3.0), Color::new(1.0, 2.0, 3.0, 255.0));
    }

    #[test]
    fn colour_equality_compares_every_component() {
        let color = Color::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(color, Color::new(1.0, 2.0, 3.0, 4.0));
        assert_ne!(color, Color::new(1.0, 2.0, 3.0, 5.0), "alpha differs");
        assert_ne!(color, Color::new(9.0, 2.0, 3.0, 4.0), "red differs");
        assert_ne!(color, Color::new(1.0, 9.0, 3.0, 4.0), "green differs");
        assert_ne!(color, Color::new(1.0, 2.0, 9.0, 4.0), "blue differs");
    }

    #[test]
    fn colour_never_holds_not_a_number() {
        let color = Color::new(f64::NAN, 1.0, 2.0, f64::NAN);
        assert_eq!(color, Color::new(0.0, 1.0, 2.0, 0.0));
    }

    #[test]
    fn tone_components_are_clamped() {
        let tone = Tone::new(-300.0, 0.0, 400.0, -1.0);
        assert_eq!(tone.red(), -255.0);
        assert_eq!(tone.blue(), 255.0);
        assert_eq!(tone.gray(), 0.0);

        let mut tone = Tone::NEUTRAL;
        tone.set_red(-300.0);
        tone.set_green(256.0);
        tone.set_blue(400.0);
        tone.set_gray(-1.0);
        assert_eq!(
            (tone.red(), tone.green(), tone.blue(), tone.gray()),
            (-255.0, 255.0, 255.0, 0.0)
        );
        tone.set_gray(300.0);
        assert_eq!(tone.gray(), 255.0);
    }

    #[test]
    fn tone_default_grey_is_0() {
        assert_eq!(Tone::rgb(10.0, -20.0, 30.0).gray(), 0.0);
        assert_eq!(
            Tone::rgb(10.0, -20.0, 30.0),
            Tone::new(10.0, -20.0, 30.0, 0.0)
        );
    }

    #[test]
    fn tone_equality_compares_every_component() {
        let tone = Tone::new(1.0, -2.0, 3.0, 4.0);
        assert_eq!(tone, Tone::new(1.0, -2.0, 3.0, 4.0));
        assert_ne!(tone, Tone::new(1.5, -2.0, 3.0, 4.0));
        assert_ne!(tone, Tone::new(1.0, 2.0, 3.0, 4.0));
        assert_ne!(tone, Tone::new(1.0, -2.0, 3.5, 4.0));
        assert_ne!(tone, Tone::new(1.0, -2.0, 3.0, 5.0));
    }

    #[test]
    fn conversion_discards_the_fraction() {
        let color = Color::new(127.9, 0.5, 254.999, 255.0);
        assert_eq!(color.to_proto(), vkxp_proto::Color::new(127, 0, 254, 255));
        assert_eq!(vkxp_proto::Color::from(color), color.to_proto());
    }

    #[test]
    fn conversion_of_a_negative_tone_component_goes_toward_zero() {
        let tone = Tone::new(-0.9, -254.9, 100.7, 99.9);
        assert_eq!(tone.to_proto(), vkxp_proto::Tone::new(0, -254, 100, 99));
        assert_eq!(vkxp_proto::Tone::from(tone), tone.to_proto());
    }

    #[test]
    fn protocol_colour_converts_back_exactly() {
        let color = vkxp_proto::Color::new(1, 2, 3, 4);
        assert_eq!(Color::from_proto(color), Color::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(Color::from_proto(color).to_proto(), color);
    }

    #[test]
    fn known_colour_bytes() {
        let bytes = Color::new(255.0, 255.0, 255.0, 255.0).to_bytes();
        let component = [0x00, 0x00, 0x00, 0x00, 0x00, 0xE0, 0x6F, 0x40];
        assert_eq!(bytes.len(), 32);
        for chunk in bytes.as_chunks::<8>().0 {
            assert_eq!(*chunk, component);
        }
    }

    #[test]
    fn serialized_components_are_in_order() {
        let bytes = Color::new(1.0, 2.0, 3.0, 4.0).to_bytes();
        assert_eq!(bytes[..8], 1.0f64.to_le_bytes());
        assert_eq!(bytes[8..16], 2.0f64.to_le_bytes());
        assert_eq!(bytes[16..24], 3.0f64.to_le_bytes());
        assert_eq!(bytes[24..], 4.0f64.to_le_bytes());

        let bytes = Tone::new(-1.0, -2.0, -3.0, 4.0).to_bytes();
        assert_eq!(bytes[..8], (-1.0f64).to_le_bytes());
        assert_eq!(bytes[8..16], (-2.0f64).to_le_bytes());
        assert_eq!(bytes[16..24], (-3.0f64).to_le_bytes());
        assert_eq!(bytes[24..], 4.0f64.to_le_bytes());
    }

    #[test]
    fn wrong_lengths_fail() {
        for len in [0, 31, 33, 64] {
            let bytes = vec![0; len];
            assert_eq!(
                Color::from_bytes(&bytes),
                Err(DecodeError::Length {
                    what: "colour",
                    expected: 32,
                    found: len,
                })
            );
            assert_eq!(
                Tone::from_bytes(&bytes),
                Err(DecodeError::Length {
                    what: "tone",
                    expected: 32,
                    found: len,
                })
            );
        }
    }

    #[test]
    fn deserialized_components_are_clamped() {
        let mut bytes = [0; 32];
        bytes[..8].copy_from_slice(&1000.0f64.to_le_bytes());
        bytes[24..].copy_from_slice(&(-1000.0f64).to_le_bytes());
        assert_eq!(
            Color::from_bytes(&bytes),
            Ok(Color::new(255.0, 0.0, 0.0, 0.0))
        );
        assert_eq!(
            Tone::from_bytes(&bytes),
            Ok(Tone::new(255.0, 0.0, 0.0, 0.0))
        );
        bytes[8..16].copy_from_slice(&(-1000.0f64).to_le_bytes());
        assert_eq!(Color::from_bytes(&bytes).unwrap().green(), -255.0);
    }

    proptest! {
        #[test]
        fn every_colour_round_trips(
            red in -300.0f64..600.0,
            green in -300.0f64..600.0,
            blue in -300.0f64..600.0,
            alpha in -300.0f64..600.0,
        ) {
            let color = Color::new(red, green, blue, alpha);
            prop_assert_eq!(Color::from_bytes(&color.to_bytes()), Ok(color));
        }

        #[test]
        fn every_tone_round_trips(
            red in -600.0f64..600.0,
            green in -600.0f64..600.0,
            blue in -600.0f64..600.0,
            gray in -300.0f64..600.0,
        ) {
            let tone = Tone::new(red, green, blue, gray);
            prop_assert_eq!(Tone::from_bytes(&tone.to_bytes()), Ok(tone));
        }

        #[test]
        fn conversion_matches_truncation(
            red in 0.0f64..=255.0,
            offset in -255.0f64..=255.0,
        ) {
            prop_assert_eq!(i64::from(Color::rgb(red, 0.0, 0.0).to_proto().r), red.trunc() as i64);
            prop_assert_eq!(
                i64::from(Tone::rgb(offset, 0.0, 0.0).to_proto().red),
                offset.trunc() as i64
            );
        }
    }
}
