//! JPEG.
//!
//! Baseline and progressive Huffman-coded files in greyscale, or in Y, Cb, Cr with chroma at
//! full resolution or halved horizontally or in both directions, are decoded here with the
//! arithmetic of the original runtime, which is IJG libjpeg 6b with its floating-point inverse
//! DCT. Every other file, and every file on which that decoder gives up, goes through
//! `jpeg-decoder`.

mod coefficients;
mod idct;
mod samples;

use jpeg_decoder::{Decoder, PixelFormat};

use crate::{DecodedImage, ImageError, ImageFormat, pixels_mut, rgba_len};

fn error(error: jpeg_decoder::Error) -> ImageError {
    match error {
        jpeg_decoder::Error::Unsupported(feature) => {
            ImageError::unsupported(format!("JPEG feature is not supported: {feature:?}"))
        }
        other => ImageError::malformed(ImageFormat::Jpeg, other.to_string()),
    }
}

/// Decodes with the arithmetic of the original runtime, or gives up with `None`.
fn decode_like_the_original(bytes: &[u8]) -> Option<DecodedImage> {
    let frame = coefficients::read(bytes)?;
    let (width, height) = (frame.width as usize, frame.height as usize);
    let (h_max, v_max) = (frame.components[0].h, frame.components[0].v);
    let planes: Vec<samples::Plane> = frame
        .components
        .iter()
        .map(|component| {
            let multipliers = idct::multipliers(&component.quant);
            let stride = component.blocks_wide * 8;
            let mut samples = vec![0; stride * component.blocks_high * 8];
            for (index, block) in component
                .coefficients
                .as_chunks::<64>()
                .0
                .iter()
                .enumerate()
            {
                let (row, column) = (index / component.blocks_wide, index % component.blocks_wide);
                let out = &mut samples[row * 8 * stride + column * 8..];
                idct::inverse(block, &multipliers, out, stride);
            }
            samples::Plane {
                samples,
                stride,
                width: (width * component.h).div_ceil(h_max),
                height: (height * component.v).div_ceil(v_max),
            }
        })
        .collect();

    let mut rgba = vec![0; width * height * 4];
    match <[samples::Plane; 3]>::try_from(planes) {
        Ok(planes) => samples::colour(&planes, (h_max == 2, v_max == 2), pixels_mut(&mut rgba)),
        Err(planes) => samples::grey(&planes[0], pixels_mut(&mut rgba)),
    }
    Some(DecodedImage {
        width: frame.width,
        height: frame.height,
        rgba,
    })
}

pub(crate) fn decode(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
    match decode_like_the_original(bytes) {
        Some(image) => Ok(image),
        None => decode_with_library(bytes),
    }
}

fn decode_with_library(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
    let mut decoder = Decoder::new(bytes);
    decoder.read_info().map_err(error)?;
    let info = decoder
        .info()
        .ok_or_else(|| ImageError::malformed(ImageFormat::Jpeg, "the headers describe no image"))?;
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    let len = rgba_len(ImageFormat::Jpeg, width, height)?;
    let channels = match info.pixel_format {
        PixelFormat::L8 => 1,
        PixelFormat::RGB24 => 3,
        other => {
            return Err(ImageError::unsupported(format!(
                "JPEG that decodes to {other:?}"
            )));
        }
    };

    let samples = decoder.decode().map_err(error)?;
    if samples.len() != len / 4 * channels {
        return Err(ImageError::malformed(
            ImageFormat::Jpeg,
            "the decoder returned an incomplete image",
        ));
    }
    let mut rgba = vec![0; len];
    for (out, sample) in pixels_mut(&mut rgba)
        .iter_mut()
        .zip(samples.chunks_exact(channels))
    {
        let pixel = match *sample {
            [grey] => [grey, grey, grey, 255],
            [red, green, blue] => [red, green, blue, 255],
            _ => unreachable!("one or three channels"),
        };
        *out = pixel;
    }
    Ok(DecodedImage {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use jpeg_encoder::{ColorType, SamplingFactor};

    use super::*;
    use crate::testutil;

    fn decode_ok(bytes: &[u8], width: u16, height: u16) -> Vec<u8> {
        let image = crate::decode(bytes).unwrap();
        assert_eq!((image.width, image.height), (width.into(), height.into()));
        assert_eq!(
            image.rgba.len(),
            usize::from(width) * usize::from(height) * 4
        );
        assert!(
            image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[3] == 255)
        );
        image.rgba
    }

    /// A smooth picture, which JPEG reproduces closely.
    fn gradient(width: u16, height: u16, channels: usize) -> Vec<u8> {
        let mut data = Vec::new();
        for y in 0..usize::from(height) {
            for x in 0..usize::from(width) {
                for channel in 0..channels {
                    data.push((40 + 3 * x + 2 * y + 30 * channel) as u8);
                }
            }
        }
        data
    }

    fn assert_close(decoded: &[u8], original: &[u8], channels: usize, tolerance: u8) {
        for (index, (pixel, source)) in decoded
            .as_chunks::<4>()
            .0
            .iter()
            .zip(original.chunks_exact(channels))
            .enumerate()
        {
            for channel in 0..3 {
                let expected = source[channel % channels];
                assert!(
                    pixel[channel].abs_diff(expected) <= tolerance,
                    "pixel {index}: {pixel:?} against {source:?}"
                );
            }
        }
    }

    #[test]
    fn greyscale() {
        let source = gradient(19, 11, 1);
        let bytes = testutil::jpeg(19, 11, ColorType::Luma, &source, |_| {});
        let rgba = decode_ok(&bytes, 19, 11);
        assert!(
            rgba.as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2])
        );
        assert_close(&rgba, &source, 1, 2);
    }

    #[test]
    fn colour_with_every_subsampling() {
        let source = vec![128; 21 * 13 * 3];
        for sampling in [
            SamplingFactor::F_1_1,
            SamplingFactor::F_2_1,
            SamplingFactor::F_1_2,
            SamplingFactor::F_2_2,
            SamplingFactor::F_4_1,
            SamplingFactor::F_4_2,
            SamplingFactor::F_1_4,
            SamplingFactor::F_2_4,
        ] {
            let bytes = testutil::jpeg(21, 13, ColorType::Rgb, &source, |encoder| {
                encoder.set_sampling_factor(sampling);
            });
            let rgba = decode_ok(&bytes, 21, 13);
            for (index, pixel) in rgba.as_chunks::<4>().0.iter().enumerate() {
                assert!(
                    pixel[..3].iter().all(|value| value.abs_diff(128) <= 2),
                    "{sampling:?}, pixel {index}: {pixel:?}"
                );
            }
        }
    }

    #[test]
    fn colour_is_reproduced() {
        let source = gradient(24, 16, 3);
        let bytes = testutil::jpeg(24, 16, ColorType::Rgb, &source, |encoder| {
            encoder.set_sampling_factor(SamplingFactor::F_1_1);
        });
        assert_close(&decode_ok(&bytes, 24, 16), &source, 3, 4);
    }

    #[test]
    fn progressive() {
        for (color, channels) in [(ColorType::Rgb, 3), (ColorType::Luma, 1)] {
            let source = gradient(21, 13, channels);
            let configure = |encoder: &mut jpeg_encoder::Encoder<&mut Vec<u8>>| {
                encoder.set_sampling_factor(SamplingFactor::F_1_1);
            };
            let baseline = testutil::jpeg(21, 13, color, &source, configure);
            let progressive = testutil::jpeg(21, 13, color, &source, |encoder| {
                configure(encoder);
                encoder.set_progressive(true);
            });
            // A progressive frame starts with the marker FF C2.
            assert!(progressive.windows(2).any(|marker| marker == [0xFF, 0xC2]));
            assert!(!baseline.windows(2).any(|marker| marker == [0xFF, 0xC2]));
            let rgba = decode_ok(&progressive, 21, 13);
            assert_close(&rgba, &source, channels, 4);
            assert_eq!(rgba, decode_ok(&baseline, 21, 13));
        }
    }

    /// A committed test file with the pixels measured on the original runtime for it; see
    /// `testdata/jpeg/README.md`.
    macro_rules! measured {
        ($jpeg:literal, $png:literal) => {
            (
                $jpeg,
                &include_bytes!(concat!("../testdata/jpeg/", $jpeg, ".jpg"))[..],
                &include_bytes!(concat!("../testdata/jpeg/", $png, ".png"))[..],
            )
        };
    }

    fn assert_measured(files: &[(&str, &[u8], &[u8])]) {
        for &(name, jpeg, png) in files {
            let decoded = decode_like_the_original(jpeg).expect(name);
            let measured = crate::decode(png).unwrap();
            assert_eq!(
                (decoded.width, decoded.height),
                (measured.width, measured.height),
                "{name}"
            );
            for (index, (pixel, expected)) in decoded
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .zip(measured.rgba.as_chunks::<4>().0)
                .enumerate()
            {
                let (x, y) = (index as u32 % decoded.width, index as u32 / decoded.width);
                assert_eq!(pixel, expected, "{name}, pixel ({x}, {y})");
            }
            assert_eq!(crate::decode(jpeg).unwrap(), decoded, "{name}");
        }
    }

    #[test]
    fn measured_colour_blocks() {
        assert_measured(&[measured!("flat", "flat")]);
    }

    #[test]
    fn measured_grey_samples() {
        assert_measured(&[
            measured!("grey-q90", "grey-q90"),
            measured!("grey-q1", "grey-q1"),
            measured!("grey-q50", "grey-q50"),
        ]);
    }

    #[test]
    fn measured_colour_with_chroma_at_full_resolution() {
        assert_measured(&[measured!("colour-444", "colour-444")]);
    }

    #[test]
    fn measured_subsampled_chroma() {
        assert_measured(&[
            measured!("colour-420", "colour-420"),
            measured!("colour-422", "colour-422"),
            measured!("colour-420-narrow", "colour-420-narrow"),
        ]);
    }

    #[test]
    fn measured_restart_intervals() {
        assert_measured(&[measured!("colour-420-restart", "colour-420")]);
    }

    #[test]
    fn progressive_equals_baseline() {
        // The same coefficients stored progressively: the pixels of the baseline file.
        assert_measured(&[
            measured!("grey-q90-progressive", "grey-q90"),
            measured!("colour-420-progressive", "colour-420"),
        ]);
    }

    #[test]
    fn other_layouts_decode_as_before() {
        let source = gradient(21, 13, 3);
        // Chroma halved vertically only, and layouts that are rarer still.
        for sampling in [
            SamplingFactor::F_1_2,
            SamplingFactor::F_4_1,
            SamplingFactor::F_2_4,
        ] {
            let bytes = testutil::jpeg(21, 13, ColorType::Rgb, &source, |encoder| {
                encoder.set_sampling_factor(sampling);
            });
            assert!(decode_like_the_original(&bytes).is_none(), "{sampling:?}");
            assert_eq!(crate::decode(&bytes), decode_with_library(&bytes));
        }
    }

    #[test]
    fn four_components_are_unsupported() {
        let source = vec![100; 8 * 8 * 4];
        for color in [ColorType::Cmyk, ColorType::CmykAsYcck] {
            let bytes = testutil::jpeg(8, 8, color, &source, |_| {});
            assert!(matches!(
                crate::decode(&bytes),
                Err(ImageError::UnsupportedFormat { .. })
            ));
        }
    }
}
