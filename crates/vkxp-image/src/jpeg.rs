//! JPEG, through `jpeg-decoder`.

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

pub(crate) fn decode(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
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
