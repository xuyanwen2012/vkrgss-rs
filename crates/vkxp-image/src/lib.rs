//! PNG, JPEG, BMP decoding.
//!
//! Side: guest.
//!
//! [`decode`] turns the contents of an image file into a [`DecodedImage`]: pixels in the bitmap
//! layout of the protocol, which is red, green, blue and straight alpha, one byte each, rows
//! from the top. The format is recognised by the leading bytes ([`ImageFormat::sniff`]), never
//! by a file name, since games name files loosely.
//!
//! - **PNG**: every colour type and bit depth, interlaced or not. Palettes and low bit depths
//!   are expanded, a `tRNS` chunk becomes alpha, 16-bit samples keep their high byte. No gamma
//!   or colour-profile correction is applied.
//! - **JPEG**: baseline and progressive, greyscale or three components, any chroma
//!   subsampling; always opaque. Greyscale, and colour with chroma at full resolution or
//!   halved horizontally or in both directions, decode to the pixels of the original runtime.
//! - **BMP**: uncompressed, 1, 4, 8, 24 or 32 bits per pixel, bottom-up or top-down. The
//!   fourth byte of 32-bit pixels is alpha, also when it is 0 in every pixel, as measured on the
//!   original runtime.
//!
//! File contents are untrusted. Every failure is an [`ImageError`]; no input makes a decoder
//! panic or return part of an image, and an image above [`MAX_PIXELS`] is refused from its
//! header, before its pixels are allocated.
//!
//! This crate does no file I/O: `vkxp-vfs` finds and reads the file.
//!
//! # Example
//!
//! ```
//! use vkxp_image::{ImageError, ImageFormat, decode};
//!
//! // A 1x2 BMP of 24 bits, stored bottom-up: a red pixel below a blue one.
//! let mut bmp = b"BM".to_vec();
//! bmp.extend([62, 0, 0, 0, 0, 0, 0, 0, 54, 0, 0, 0]); // file size, reserved, pixel offset
//! bmp.extend([40, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0]); // header size, width, height
//! bmp.extend([1, 0, 24, 0, 0, 0, 0, 0]); // planes, bits per pixel, no compression
//! bmp.extend([0; 20]);
//! bmp.extend([0, 0, 255, 0]); // blue, green, red, padding
//! bmp.extend([255, 0, 0, 0]);
//!
//! assert_eq!(ImageFormat::sniff(&bmp), Some(ImageFormat::Bmp));
//! let image = decode(&bmp)?;
//! assert_eq!((image.width, image.height), (1, 2));
//! assert_eq!(image.rgba, [0, 0, 255, 255, 255, 0, 0, 255]);
//!
//! assert!(matches!(decode(b"GIF89a"), Err(ImageError::UnsupportedFormat { .. })));
//! assert!(matches!(decode(&bmp[..60]), Err(ImageError::Malformed { .. })));
//! # Ok::<(), ImageError>(())
//! ```

#![forbid(unsafe_code)]

mod bmp;
mod error;
mod jpeg;
#[cfg(test)]
mod malformed_tests;
mod png;
#[cfg(test)]
mod testutil;

pub use error::ImageError;

/// The largest number of pixels an image may have: the limit the compositors put on a bitmap.
pub const MAX_PIXELS: u64 = 1 << 28;

/// An image file format this crate decodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Bmp,
}

impl std::fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ImageFormat::Png => "PNG",
            ImageFormat::Jpeg => "JPEG",
            ImageFormat::Bmp => "BMP",
        })
    }
}

impl ImageFormat {
    /// Recognises a format by the leading bytes of the data, whatever the file was named.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
            Some(ImageFormat::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(ImageFormat::Jpeg)
        } else if bytes.starts_with(b"BM") {
            Some(ImageFormat::Bmp)
        } else {
            None
        }
    }
}

/// Decoded pixels in the bitmap layout of the protocol.
#[derive(Clone, PartialEq, Eq)]
pub struct DecodedImage {
    /// Width in pixels, at least 1.
    pub width: u32,
    /// Height in pixels, at least 1.
    pub height: u32,
    /// `width * height * 4` bytes: red, green, blue and straight alpha per pixel, rows from top
    /// to bottom.
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for DecodedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodedImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("rgba", &format_args!("[{} bytes]", self.rgba.len()))
            .finish()
    }
}

/// Decodes the contents of a PNG, JPEG or BMP file.
///
/// Truncated or corrupt data is an error, never a panic or a partial image.
pub fn decode(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
    match ImageFormat::sniff(bytes) {
        Some(ImageFormat::Png) => png::decode(bytes),
        Some(ImageFormat::Jpeg) => jpeg::decode(bytes),
        Some(ImageFormat::Bmp) => bmp::decode(bytes),
        None => Err(ImageError::unsupported("the data is not PNG, JPEG or BMP")),
    }
}

/// Checks declared dimensions and returns the length of the pixel buffer they need. Decoders
/// call this before allocating anything that grows with the image.
fn rgba_len(format: ImageFormat, width: u32, height: u32) -> Result<usize, ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::malformed(
            format,
            format!("the image is {width}x{height} pixels"),
        ));
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_PIXELS {
        return Err(ImageError::TooLarge {
            width,
            height,
            limit: MAX_PIXELS,
        });
    }
    usize::try_from(pixels * 4).map_err(|_| ImageError::TooLarge {
        width,
        height,
        limit: MAX_PIXELS,
    })
}

/// The pixels of a red, green, blue, alpha buffer.
fn pixels_mut(rgba: &mut [u8]) -> &mut [[u8; 4]] {
    rgba.as_chunks_mut().0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil;

    #[test]
    fn wrong_extension() {
        // The decoder never sees a file name: PNG data read from `picture.jpg` is PNG.
        let pixels = testutil::pattern(3, 2, 4);
        let bytes = testutil::png(
            3,
            2,
            ::png::ColorType::Rgba,
            ::png::BitDepth::Eight,
            &pixels,
        );
        assert_eq!(ImageFormat::sniff(&bytes), Some(ImageFormat::Png));
        let image = decode(&bytes).unwrap();
        assert_eq!((image.width, image.height), (3, 2));
        assert_eq!(image.rgba.len(), 24);
        assert_eq!(image.rgba, pixels);
    }

    #[test]
    fn every_format_is_recognised() {
        let jpeg = testutil::jpeg(8, 8, jpeg_encoder::ColorType::Luma, &[128; 64], |_| {});
        assert_eq!(ImageFormat::sniff(&jpeg), Some(ImageFormat::Jpeg));
        let bmp = testutil::Bmp::new(1, 1, 24, vec![0, 0, 0, 0]).build();
        assert_eq!(ImageFormat::sniff(&bmp), Some(ImageFormat::Bmp));
    }

    #[test]
    fn unknown_format() {
        let inputs: [&[u8]; 6] = [
            b"",
            b"GIF89a\x01\x00\x01\x00",
            b"\x89PNG\r\n",
            b"\xFF\xD8",
            b"B",
            b"RIFF\0\0\0\0WEBP",
        ];
        for input in inputs {
            assert_eq!(ImageFormat::sniff(input), None);
            assert!(matches!(
                decode(input),
                Err(ImageError::UnsupportedFormat { .. })
            ));
        }
    }
}
