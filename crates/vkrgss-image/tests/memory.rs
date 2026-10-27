//! The memory of a JPEG decode, measured with a counting allocator: nothing for a frame that no
//! scan codes, and at most four times the RGBA output at once, as the crate documentation
//! states.

mod counting;

use jpeg_encoder::{ColorType, Encoder, SamplingFactor};
use vkrgss_image::{DecodedImage, ImageError, decode};

#[global_allocator]
static ALLOCATOR: counting::Counting = counting::Counting;

/// The multiple of the output that the crate documentation states.
const MULTIPLE: usize = 4;

/// A file of exactly 200 bytes: tables and a frame header of 16384 by 16384 pixels in three
/// components, no scan, and the end-of-image marker.
fn header_without_a_scan(frame_marker: u8) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8];
    bytes.extend([0xFF, 0xDB, 0x00, 0x43, 0x00]);
    bytes.extend([1; 64]);
    bytes.extend([
        0xFF,
        frame_marker,
        0x00,
        0x11,
        0x08,
        0x40,
        0x00,
        0x40,
        0x00,
        0x03,
    ]);
    bytes.extend([1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0]);
    // A comment fills the file up to its size.
    let comment = 200 - bytes.len() - 4;
    bytes.extend([0xFF, 0xFE]);
    bytes.extend(u16::try_from(comment).unwrap().to_be_bytes());
    bytes.resize(bytes.len() + comment - 2, b'.');
    bytes.extend([0xFF, 0xD9]);
    assert_eq!(bytes.len(), 200);
    bytes
}

#[test]
fn a_header_without_a_scan_allocates_nothing_for_its_size() {
    for marker in [0xC0, 0xC1, 0xC2] {
        let bytes = header_without_a_scan(marker);
        let (result, usage) = counting::measure(|| decode(&bytes));
        assert!(
            matches!(result, Err(ImageError::Malformed { .. })),
            "{result:?}"
        );
        assert!(usage.total < 1 << 20, "{usage:?}");
    }
}

/// A smooth colour picture of `size` by `size` pixels.
fn picture(size: u16, sampling: SamplingFactor, progressive: bool) -> Vec<u8> {
    let size_ = usize::from(size);
    let mut data = Vec::with_capacity(size_ * size_ * 3);
    for y in 0..size_ {
        for x in 0..size_ {
            data.extend([(x / 4) as u8, (y / 4) as u8, ((x + y) / 8) as u8]);
        }
    }
    let mut out = Vec::new();
    let mut encoder = Encoder::new(&mut out, 90);
    encoder.set_sampling_factor(sampling);
    encoder.set_progressive(progressive);
    encoder.encode(&data, size, size, ColorType::Rgb).unwrap();
    out
}

fn assert_bounded(bytes: &[u8], size: u32) {
    let (image, usage) = counting::measure(|| decode(bytes));
    let image: DecodedImage = image.unwrap();
    assert_eq!((image.width, image.height), (size, size));
    let output = image.rgba.len();
    assert!(
        usage.peak <= MULTIPLE * output,
        "{usage:?} for {output} bytes"
    );
}

#[test]
fn a_decode_holds_at_most_four_times_its_output() {
    // Chroma at full resolution, which the decoder of the original decodes, at 1024 by 1024:
    // a 4 MiB output.
    for progressive in [false, true] {
        assert_bounded(&picture(1024, SamplingFactor::F_1_1, progressive), 1024);
    }
    // A layout that the library decodes.
    assert_bounded(&picture(1024, SamplingFactor::F_4_1, false), 1024);
}
