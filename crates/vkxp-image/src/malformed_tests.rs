//! Truncated, corrupt and oversized input for all three formats.

use proptest::prelude::*;

use crate::testutil::{self, Bmp, bmp_rows, pattern};
use crate::{ImageError, ImageFormat, MAX_PIXELS, decode};

const WIDTH: u32 = 16;
const HEIGHT: u32 = 12;

/// One valid file per format, all of [`WIDTH`] by [`HEIGHT`] pixels.
fn samples() -> [Vec<u8>; 4] {
    let rgba = pattern(WIDTH, HEIGHT, 4);
    let rgb = pattern(WIDTH, HEIGHT, 3);
    let (width, height) = (WIDTH as u16, HEIGHT as u16);
    [
        testutil::png(
            WIDTH,
            HEIGHT,
            png::ColorType::Rgba,
            png::BitDepth::Eight,
            &rgba,
        ),
        testutil::jpeg(width, height, jpeg_encoder::ColorType::Rgb, &rgb, |_| {}),
        testutil::jpeg(
            width,
            height,
            jpeg_encoder::ColorType::Rgb,
            &rgb,
            |encoder| {
                encoder.set_progressive(true);
            },
        ),
        Bmp::new(
            WIDTH as i32,
            HEIGHT as i32,
            24,
            bmp_rows(&rgb, WIDTH as usize * 3),
        )
        .build(),
    ]
}

/// A decode of damaged data may fail, or may succeed with a complete image; it may not return
/// anything else.
fn assert_error_or_complete(bytes: &[u8]) -> Result<(), TestCaseError> {
    if let Ok(image) = decode(bytes) {
        prop_assert!(image.width > 0 && image.height > 0);
        let pixels = u64::from(image.width) * u64::from(image.height);
        prop_assert!(pixels <= MAX_PIXELS);
        prop_assert_eq!(image.rgba.len() as u64, pixels * 4);
    }
    Ok(())
}

#[test]
fn truncated_files() {
    for sample in samples() {
        let whole = decode(&sample).unwrap();
        assert_eq!((whole.width, whole.height), (WIDTH, HEIGHT));
        for len in 0..sample.len() {
            if let Ok(image) = decode(&sample[..len]) {
                assert_eq!((image.width, image.height), (WIDTH, HEIGHT), "{len} bytes");
                assert_eq!(
                    image.rgba.len(),
                    (WIDTH * HEIGHT * 4) as usize,
                    "{len} bytes"
                );
            }
        }
    }
}

#[test]
fn truncated_lossless_files_are_errors() {
    // PNG and BMP have no way to complete a picture whose data is missing.
    let [png, _, _, bmp] = samples();
    for (sample, format) in [(png, ImageFormat::Png), (bmp, ImageFormat::Bmp)] {
        // The last 12 bytes of a PNG are its end marker, which carries no pixels.
        let needed = if format == ImageFormat::Png {
            sample.len() - 12
        } else {
            sample.len()
        };
        for len in 8..needed {
            match decode(&sample[..len]) {
                Err(ImageError::Malformed { format: found, .. }) => assert_eq!(found, format),
                other => panic!("{format} cut to {len} bytes: {other:?}"),
            }
        }
    }
}

/// The signature each format is recognised by, followed by nothing.
const SIGNATURES: [&[u8]; 3] = [b"\x89PNG\r\n\x1A\n", b"\xFF\xD8\xFF", b"BM"];

proptest! {
    #[test]
    fn random_bytes_behind_a_signature(
        format in 0..SIGNATURES.len(),
        tail in prop::collection::vec(any::<u8>(), 0..600),
    ) {
        let bytes = [SIGNATURES[format], &tail].concat();
        assert_error_or_complete(&bytes)?;
    }

    #[test]
    fn random_damage_to_a_valid_file(
        sample in 0..4usize,
        damage in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..6),
        cut in any::<Option<prop::sample::Index>>(),
    ) {
        let mut bytes = samples()[sample].clone();
        for (at, value) in damage {
            let at = at.index(bytes.len());
            bytes[at] = value;
        }
        if let Some(cut) = cut {
            bytes.truncate(cut.index(bytes.len()));
        }
        assert_error_or_complete(&bytes)?;
    }
}

fn assert_too_large(bytes: &[u8], width: u32, height: u32) {
    assert_eq!(
        decode(bytes),
        Err(ImageError::TooLarge {
            width,
            height,
            limit: MAX_PIXELS
        })
    );
}

#[test]
fn oversized_png() {
    // A signature and a header chunk declaring 65535 by 65535 pixels: 16 GiB if allocated.
    let mut chunk = b"IHDR".to_vec();
    chunk.extend(65535u32.to_be_bytes());
    chunk.extend(65535u32.to_be_bytes());
    chunk.extend([8, 6, 0, 0, 0]);
    let mut bytes = SIGNATURES[0].to_vec();
    bytes.extend(13u32.to_be_bytes());
    bytes.extend(&chunk);
    bytes.extend(testutil::crc32(&chunk).to_be_bytes());
    assert_too_large(&bytes, 65535, 65535);
}

#[test]
fn oversized_jpeg() {
    // A valid file whose frame header is rewritten to 65535 by 65535 pixels.
    let [_, mut bytes, _, _] = samples();
    let frame = bytes
        .windows(2)
        .position(|marker| marker == [0xFF, 0xC0])
        .unwrap();
    // Marker, segment length and sample precision come before the two dimensions.
    bytes[frame + 5..frame + 9].copy_from_slice(&[0xFF; 4]);
    assert_too_large(&bytes, 65535, 65535);
}

#[test]
fn oversized_bmp() {
    let bytes = Bmp::new(65535, 65535, 32, Vec::new()).build();
    assert_too_large(&bytes, 65535, 65535);
    let bytes = Bmp::new(i32::MAX, i32::MIN, 1, Vec::new()).build();
    assert_too_large(&bytes, i32::MAX as u32, 1 << 31);
}

#[test]
fn largest_accepted_size_is_not_refused_for_its_size() {
    // Exactly the limit: the header passes the size check and the missing pixels are what
    // fails, before anything is allocated for them.
    let bytes = Bmp::new(1 << 14, 1 << 14, 32, Vec::new()).build();
    assert!(matches!(decode(&bytes), Err(ImageError::Malformed { .. })));
    let bytes = Bmp::new((1 << 14) + 1, 1 << 14, 32, Vec::new()).build();
    assert!(matches!(decode(&bytes), Err(ImageError::TooLarge { .. })));
}
