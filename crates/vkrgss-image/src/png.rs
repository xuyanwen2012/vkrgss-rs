//! PNG, through the `png` crate.

use std::io::Cursor;

use png::{BitDepth, ColorType, Transformations};

use crate::{DecodedImage, ImageError, ImageFormat, pixels_mut, rgba_len};

fn malformed(error: png::DecodingError) -> ImageError {
    ImageError::malformed(ImageFormat::Png, error.to_string())
}

pub(crate) fn decode(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    // Palettes, low bit depths and `tRNS` are expanded and 16-bit samples lose their low byte.
    // The crate applies no gamma or colour-profile correction.
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let header = decoder.read_header_info().map_err(malformed)?;
    let (width, height) = header.size();
    let len = rgba_len(ImageFormat::Png, width, height)?;

    let mut reader = decoder.read_info().map_err(malformed)?;
    let (color, depth) = reader.output_color_type();
    let channels = match (color, depth) {
        (ColorType::Grayscale, BitDepth::Eight) => 1,
        (ColorType::GrayscaleAlpha, BitDepth::Eight) => 2,
        (ColorType::Rgb, BitDepth::Eight) => 3,
        (ColorType::Rgba, BitDepth::Eight) => 4,
        other => {
            return Err(ImageError::unsupported(format!(
                "PNG decodes to unexpected samples {other:?}"
            )));
        }
    };
    let mut samples = vec![0; len / 4 * channels];
    reader.next_frame(&mut samples).map_err(malformed)?;

    let rgba = match channels {
        4 => samples,
        _ => {
            let mut rgba = vec![0; len];
            for (out, sample) in pixels_mut(&mut rgba)
                .iter_mut()
                .zip(samples.chunks_exact(channels))
            {
                let pixel = match *sample {
                    [grey] => [grey, grey, grey, 255],
                    [grey, alpha] => [grey, grey, grey, alpha],
                    [red, green, blue] => [red, green, blue, 255],
                    _ => unreachable!("one to three channels"),
                };
                *out = pixel;
            }
            rgba
        }
    };
    Ok(DecodedImage {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{self, pack, pattern};

    fn decode_ok(bytes: &[u8], width: u32, height: u32) -> Vec<u8> {
        let image = crate::decode(bytes).unwrap();
        assert_eq!((image.width, image.height), (width, height));
        assert_eq!(image.rgba.len(), (width * height * 4) as usize);
        image.rgba
    }

    #[test]
    fn lossless_round_trip() {
        let pixels = pattern(7, 5, 4);
        let bytes = testutil::png(7, 5, ColorType::Rgba, BitDepth::Eight, &pixels);
        assert_eq!(decode_ok(&bytes, 7, 5), pixels);
    }

    #[test]
    fn palette_with_transparency() {
        let palette = [10, 20, 30, 40, 50, 60, 70, 80, 90];
        let indices = [0, 1, 2, 1, 0, 2];
        for (depth, bits) in [
            (BitDepth::Two, 2),
            (BitDepth::Four, 4),
            (BitDepth::Eight, 8),
        ] {
            let data = pack(&indices, 3, bits);
            let bytes = testutil::png_with(3, 2, ColorType::Indexed, depth, &data, |encoder| {
                encoder.set_palette(palette.to_vec());
                // Entry 1 is fully transparent; entry 2 has no value and is opaque.
                encoder.set_trns(vec![255, 0]);
            });
            let expected: Vec<u8> = indices
                .iter()
                .flat_map(|&index| {
                    let rgb = &palette[index as usize * 3..][..3];
                    [rgb[0], rgb[1], rgb[2], if index == 1 { 0 } else { 255 }]
                })
                .collect();
            assert_eq!(decode_ok(&bytes, 3, 2), expected, "{depth:?}");
        }
    }

    #[test]
    fn palette_without_transparency_is_opaque() {
        let data = pack(&[1, 0, 0, 1, 1, 1, 0, 0, 1, 0], 5, 1);
        let bytes = testutil::png_with(5, 2, ColorType::Indexed, BitDepth::One, &data, |encoder| {
            encoder.set_palette(vec![1, 2, 3, 250, 251, 252]);
        });
        let rgba = decode_ok(&bytes, 5, 2);
        assert_eq!(rgba[..8], [250, 251, 252, 255, 1, 2, 3, 255]);
        assert!(rgba.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn sixteen_bit_samples() {
        let bytes = testutil::png(1, 1, ColorType::Grayscale, BitDepth::Sixteen, &[0xAB, 0xCD]);
        assert_eq!(decode_ok(&bytes, 1, 1), [0xAB, 0xAB, 0xAB, 255]);
    }

    #[test]
    fn greyscale_at_every_depth() {
        for (depth, bits) in [(BitDepth::One, 1), (BitDepth::Two, 2), (BitDepth::Four, 4)] {
            let most = (1u32 << bits) - 1;
            let samples: Vec<u8> = (0..15).map(|i| (i * 7 % (most + 1)) as u8).collect();
            let bytes = testutil::png(5, 3, ColorType::Grayscale, depth, &pack(&samples, 5, bits));
            let expected: Vec<u8> = samples
                .iter()
                .flat_map(|&sample| {
                    let grey = (u32::from(sample) * 255 / most) as u8;
                    [grey, grey, grey, 255]
                })
                .collect();
            assert_eq!(decode_ok(&bytes, 5, 3), expected, "{depth:?}");
        }

        let samples = pattern(5, 3, 1);
        let bytes = testutil::png(5, 3, ColorType::Grayscale, BitDepth::Eight, &samples);
        let expected: Vec<u8> = samples.iter().flat_map(|&g| [g, g, g, 255]).collect();
        assert_eq!(decode_ok(&bytes, 5, 3), expected);

        let samples = pattern(5, 3, 2);
        let bytes = testutil::png(5, 3, ColorType::Grayscale, BitDepth::Sixteen, &samples);
        let expected: Vec<u8> = samples
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|s| [s[0], s[0], s[0], 255])
            .collect();
        assert_eq!(decode_ok(&bytes, 5, 3), expected);
    }

    #[test]
    fn greyscale_with_alpha() {
        let samples = pattern(4, 3, 2);
        let bytes = testutil::png(4, 3, ColorType::GrayscaleAlpha, BitDepth::Eight, &samples);
        let expected: Vec<u8> = samples
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|s| [s[0], s[0], s[0], s[1]])
            .collect();
        assert_eq!(decode_ok(&bytes, 4, 3), expected);

        let samples = pattern(4, 3, 4);
        let bytes = testutil::png(4, 3, ColorType::GrayscaleAlpha, BitDepth::Sixteen, &samples);
        let expected: Vec<u8> = samples
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|s| [s[0], s[0], s[0], s[2]])
            .collect();
        assert_eq!(decode_ok(&bytes, 4, 3), expected);
    }

    #[test]
    fn truecolour() {
        let samples = pattern(4, 3, 3);
        let bytes = testutil::png(4, 3, ColorType::Rgb, BitDepth::Eight, &samples);
        let expected: Vec<u8> = samples
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|s| [s[0], s[1], s[2], 255])
            .collect();
        assert_eq!(decode_ok(&bytes, 4, 3), expected);

        let samples = pattern(4, 3, 6);
        let bytes = testutil::png(4, 3, ColorType::Rgb, BitDepth::Sixteen, &samples);
        let expected: Vec<u8> = samples
            .as_chunks::<6>()
            .0
            .iter()
            .flat_map(|s| [s[0], s[2], s[4], 255])
            .collect();
        assert_eq!(decode_ok(&bytes, 4, 3), expected);
    }

    #[test]
    fn truecolour_with_alpha_at_sixteen_bits() {
        let samples = pattern(4, 3, 8);
        let bytes = testutil::png(4, 3, ColorType::Rgba, BitDepth::Sixteen, &samples);
        let expected: Vec<u8> = samples
            .as_chunks::<8>()
            .0
            .iter()
            .flat_map(|s| [s[0], s[2], s[4], s[6]])
            .collect();
        assert_eq!(decode_ok(&bytes, 4, 3), expected);
    }

    #[test]
    fn transparent_colour_of_greyscale_and_truecolour() {
        let bytes = testutil::png_with(
            3,
            1,
            ColorType::Grayscale,
            BitDepth::Eight,
            &[5, 6, 7],
            |encoder| encoder.set_trns(vec![0, 6]),
        );
        let expected = [5, 5, 5, 255, 6, 6, 6, 0, 7, 7, 7, 255];
        assert_eq!(decode_ok(&bytes, 3, 1), expected);

        let bytes = testutil::png_with(
            2,
            1,
            ColorType::Rgb,
            BitDepth::Eight,
            &[1, 2, 3, 1, 2, 4],
            |encoder| encoder.set_trns(vec![0, 1, 0, 2, 0, 3]),
        );
        assert_eq!(decode_ok(&bytes, 2, 1), [1, 2, 3, 0, 1, 2, 4, 255]);
    }

    #[test]
    fn interlaced() {
        // Sizes on both sides of the 8-pixel interlacing period, and one with empty passes.
        for (width, height) in [(11, 9), (8, 8), (1, 1), (3, 17)] {
            let pixels = pattern(width, height, 4);
            let bytes = testutil::interlaced_png(width, height, &pixels);
            assert_eq!(decode_ok(&bytes, width, height), pixels, "{width}x{height}");
        }
    }

    #[test]
    fn gamma_is_not_applied() {
        let pixels = pattern(4, 4, 4);
        let bytes =
            testutil::png_with(4, 4, ColorType::Rgba, BitDepth::Eight, &pixels, |encoder| {
                encoder.set_source_gamma(png::ScaledFloat::new(1.0));
            });
        assert!(bytes.windows(4).any(|window| window == b"gAMA"));
        assert_eq!(decode_ok(&bytes, 4, 4), pixels);
    }
}
