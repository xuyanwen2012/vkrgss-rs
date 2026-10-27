//! The check of the fuzz targets of the image decoders, which both include: a decode may fail,
//! or may return a complete image; it may not panic, and it may not hold more memory at once
//! than the bounds that `vkrgss-image` states.

use vkrgss_image::{ImageFormat, MAX_PIXELS};

use crate::counting;

/// The multiple of the RGBA output that a decode holds at most, and the allowance beyond it,
/// as the crate documentation of `vkrgss-image` states them.
const MULTIPLE: usize = 4;
const ALLOWANCE: usize = 1 << 20;

/// The most a decode of any image may hold: the multiple of the output of the largest image.
/// No image above [`MAX_PIXELS`] is allocated.
const LARGEST: usize = MULTIPLE * 4 * MAX_PIXELS as usize + ALLOWANCE;

/// Decodes `bytes` if they are of `format`, and checks the result and the memory it took.
/// `per_image` asks that a decoded image held at most [`MULTIPLE`] times its own output, which
/// the BMP decoder keeps; files that the JPEG decoder of the original leaves to `jpeg-decoder`
/// are held to [`LARGEST`] alone.
pub(crate) fn image(bytes: &[u8], format: ImageFormat, per_image: bool) {
    if ImageFormat::sniff(bytes) != Some(format) {
        return;
    }
    let (result, usage) = counting::measure(|| vkrgss_image::decode(bytes));
    let bound = match &result {
        Ok(image) => {
            let pixels = u64::from(image.width) * u64::from(image.height);
            assert!(image.width > 0 && image.height > 0, "{image:?}");
            assert!(pixels <= MAX_PIXELS, "{image:?}");
            assert_eq!(image.rgba.len() as u64, pixels * 4, "part of an image");
            if per_image {
                MULTIPLE * image.rgba.len() + ALLOWANCE
            } else {
                LARGEST
            }
        }
        Err(_) => LARGEST,
    };
    assert!(
        usage.peak <= bound,
        "the decode held {} bytes at once ({} in all), above the bound of {bound}",
        usage.peak,
        usage.total
    );
}
