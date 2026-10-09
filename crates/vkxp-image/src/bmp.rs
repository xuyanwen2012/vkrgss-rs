//! BMP, decoded here: uncompressed images of 1, 4, 8, 24 and 32 bits per pixel.

use crate::{DecodedImage, ImageError, ImageFormat, pixels_mut, rgba_len};

/// Size of the file header that precedes the information header.
const FILE_HEADER_LEN: usize = 14;
/// Size of the oldest information header, which has 16-bit dimensions and 3-byte palette
/// entries.
const CORE_HEADER_LEN: u32 = 12;
/// Size of the common information header. Later versions extend it.
const INFO_HEADER_LEN: u32 = 40;

const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;

fn malformed(reason: impl Into<String>) -> ImageError {
    ImageError::malformed(ImageFormat::Bmp, reason)
}

fn truncated() -> ImageError {
    malformed("the data ends early")
}

fn bytes_at<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], ImageError> {
    bytes
        .get(offset..)
        .and_then(|rest| rest.first_chunk::<N>())
        .copied()
        .ok_or_else(truncated)
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, ImageError> {
    bytes_at(bytes, offset).map(u16::from_le_bytes)
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, ImageError> {
    bytes_at(bytes, offset).map(u32::from_le_bytes)
}

fn i32_at(bytes: &[u8], offset: usize) -> Result<i32, ImageError> {
    bytes_at(bytes, offset).map(i32::from_le_bytes)
}

/// The fields of the information header that decoding needs.
struct Header {
    width: u32,
    height: u32,
    top_down: bool,
    bits: u16,
    /// Where the palette starts: the end of the information header and of the channel masks
    /// that may follow it. The pixels cannot start before this.
    palette_offset: usize,
    /// Bytes per palette entry: blue, green, red and, in all but the oldest header, a filler.
    palette_entry_len: usize,
    /// Declared number of palette entries; 0 means as many as the bit depth allows.
    colors_used: u32,
}

fn header(bytes: &[u8]) -> Result<Header, ImageError> {
    let header_len = u32_at(bytes, FILE_HEADER_LEN)?;
    // The whole header the file declares must be present, whatever part of it is read.
    let palette_offset = usize::try_from(header_len)
        .ok()
        .and_then(|len| len.checked_add(FILE_HEADER_LEN))
        .filter(|end| *end <= bytes.len())
        .ok_or_else(truncated)?;
    let check_planes = |planes: u16| match planes {
        1 => Ok(()),
        other => Err(malformed(format!("{other} colour planes"))),
    };
    if header_len == CORE_HEADER_LEN {
        check_planes(u16_at(bytes, 22)?)?;
        return Ok(Header {
            width: u32::from(u16_at(bytes, 18)?),
            height: u32::from(u16_at(bytes, 20)?),
            top_down: false,
            bits: u16_at(bytes, 24)?,
            palette_offset,
            palette_entry_len: 3,
            colors_used: 0,
        });
    }
    if header_len < INFO_HEADER_LEN {
        return Err(malformed(format!(
            "information header of {header_len} bytes"
        )));
    }
    let width = i32_at(bytes, 18)?;
    let height = i32_at(bytes, 22)?;
    check_planes(u16_at(bytes, 26)?)?;
    let bits = u16_at(bytes, 28)?;
    let compression = u32_at(bytes, 30)?;
    let colors_used = u32_at(bytes, 46)?;
    // The common header is followed by its channel masks; later headers contain them.
    let masks_len = if compression == BI_BITFIELDS && header_len == INFO_HEADER_LEN {
        12
    } else {
        0
    };
    match compression {
        BI_RGB => {}
        // The layout that image editors write for 32 bits is the uncompressed one with its
        // channel masks spelled out. Other masks would need a different decoder.
        BI_BITFIELDS if bits == 32 => {
            let masks = [u32_at(bytes, 54)?, u32_at(bytes, 58)?, u32_at(bytes, 62)?];
            if masks != [0x00FF_0000, 0x0000_FF00, 0x0000_00FF] {
                return Err(ImageError::unsupported(format!(
                    "BMP with channel masks {masks:08X?}"
                )));
            }
        }
        other => {
            return Err(ImageError::unsupported(format!(
                "BMP with compression {other}"
            )));
        }
    }
    if width < 0 {
        return Err(malformed(format!("width {width}")));
    }
    Ok(Header {
        width: width.unsigned_abs(),
        height: height.unsigned_abs(),
        top_down: height < 0,
        bits,
        palette_offset: palette_offset + masks_len,
        palette_entry_len: 4,
        colors_used,
    })
}

/// Reads the palette as red, green, blue entries.
fn palette(bytes: &[u8], header: &Header) -> Result<Vec<[u8; 3]>, ImageError> {
    let most = 1usize << header.bits;
    let count = match usize::try_from(header.colors_used) {
        Ok(0) | Err(_) => most,
        Ok(used) => used.min(most),
    };
    let entries = bytes
        .get(header.palette_offset..)
        .and_then(|rest| rest.get(..count * header.palette_entry_len))
        .ok_or_else(truncated)?;
    Ok(entries
        .chunks_exact(header.palette_entry_len)
        .map(|entry| [entry[2], entry[1], entry[0]])
        .collect())
}

pub(crate) fn decode(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
    let pixel_offset = u32_at(bytes, 10)?;
    let header = header(bytes)?;
    if !matches!(header.bits, 1 | 4 | 8 | 24 | 32) {
        return Err(ImageError::unsupported(format!(
            "BMP with {} bits per pixel",
            header.bits
        )));
    }
    let (width, height) = (header.width, header.height);
    let len = rgba_len(ImageFormat::Bmp, width, height)?;
    let palette = if header.bits <= 8 {
        palette(bytes, &header)?
    } else {
        Vec::new()
    };
    // The pixels come after the headers and the palette, never on top of them.
    let pixels_from = header.palette_offset + palette.len() * header.palette_entry_len;
    if (pixel_offset as u64) < pixels_from as u64 {
        return Err(malformed(format!(
            "pixels at offset {pixel_offset} overlap the {pixels_from} bytes of headers and palette"
        )));
    }

    // Rows are padded to a multiple of four bytes. All of them must be present before the
    // pixel buffer is allocated.
    let stride = (u64::from(width) * u64::from(header.bits)).div_ceil(32) * 4;
    let rows = usize::try_from(pixel_offset)
        .ok()
        .and_then(|offset| bytes.get(offset..))
        .zip(usize::try_from(stride * u64::from(height)).ok())
        .and_then(|(rest, needed)| rest.get(..needed))
        .ok_or_else(truncated)?;
    let stride = stride as usize;
    let row_len = width as usize * 4;

    let mut rgba = vec![0; len];
    for (y, out) in rgba.chunks_exact_mut(row_len).enumerate() {
        let stored = if header.top_down {
            y
        } else {
            height as usize - 1 - y
        };
        let row = &rows[stored * stride..][..stride];
        match header.bits {
            24 => {
                for (out, pixel) in pixels_mut(out).iter_mut().zip(row.as_chunks::<3>().0) {
                    *out = [pixel[2], pixel[1], pixel[0], 255];
                }
            }
            32 => {
                for (out, pixel) in pixels_mut(out).iter_mut().zip(row.as_chunks::<4>().0) {
                    *out = [pixel[2], pixel[1], pixel[0], pixel[3]];
                }
            }
            bits => {
                let bits = usize::from(bits);
                for (x, out) in pixels_mut(out).iter_mut().enumerate() {
                    // Pixels are packed from the most significant bit of each byte.
                    let shift = 8 - bits - (x * bits) % 8;
                    let index = (row[x * bits / 8] >> shift) & (0xFF >> (8 - bits));
                    let [red, green, blue] = *palette.get(usize::from(index)).ok_or_else(|| {
                        malformed(format!(
                            "palette index {index} with {} palette entries",
                            palette.len()
                        ))
                    })?;
                    *out = [red, green, blue, 255];
                }
            }
        }
    }
    // The fourth byte of 32-bit pixels is alpha only if some pixel uses it; files that leave
    // it zero throughout are opaque.
    if header.bits == 32 && pixels_mut(&mut rgba).iter().all(|pixel| pixel[3] == 0) {
        for pixel in pixels_mut(&mut rgba) {
            pixel[3] = 255;
        }
    }
    Ok(DecodedImage {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{Bmp, bmp_rows, pack, pattern};

    const RED: [u8; 3] = [0, 0, 255];
    const BLUE: [u8; 3] = [255, 0, 0];

    fn decode_ok(bytes: &[u8], width: u32, height: u32) -> Vec<u8> {
        let image = crate::decode(bytes).unwrap();
        assert_eq!((image.width, image.height), (width, height));
        assert_eq!(image.rgba.len(), (width * height * 4) as usize);
        image.rgba
    }

    /// Stored blue, green, red bytes as the red, green, blue, alpha pixels they decode to.
    fn opaque(bgr: &[u8]) -> Vec<u8> {
        bgr.as_chunks::<3>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[2], pixel[1], pixel[0], 255])
            .collect()
    }

    /// Reverses the order of rows of `row_len` bytes.
    fn flip(data: &[u8], row_len: usize) -> Vec<u8> {
        data.chunks(row_len).rev().flatten().copied().collect()
    }

    #[test]
    fn bottom_up_rows() {
        let stored = [RED, RED, BLUE, BLUE].concat();
        let bytes = Bmp::new(2, 2, 24, bmp_rows(&stored, 6)).build();
        let expected = opaque(&[BLUE, BLUE, RED, RED].concat());
        assert_eq!(decode_ok(&bytes, 2, 2), expected);
    }

    #[test]
    fn top_down_rows() {
        let stored = [RED, RED, BLUE, BLUE].concat();
        let bytes = Bmp::new(2, -2, 24, bmp_rows(&stored, 6)).build();
        assert_eq!(decode_ok(&bytes, 2, 2), opaque(&stored));
    }

    #[test]
    fn palette() {
        let indices = [0, 1, 1, 0, 1, 0];
        let bytes = Bmp::new(3, -2, 8, bmp_rows(&indices, 3))
            .palette(&[[10, 20, 30, 0], [40, 50, 60, 0]])
            .build();
        let expected: Vec<u8> = indices
            .iter()
            .flat_map(|&index| {
                if index == 0 {
                    [30, 20, 10, 255]
                } else {
                    [60, 50, 40, 255]
                }
            })
            .collect();
        assert_eq!(decode_ok(&bytes, 3, 2), expected);
    }

    fn is_malformed(bytes: &[u8]) -> bool {
        matches!(
            crate::decode(bytes),
            Err(ImageError::Malformed {
                format: ImageFormat::Bmp,
                ..
            })
        )
    }

    #[test]
    fn index_past_the_palette_is_malformed() {
        let two_colours = [[10, 20, 30, 0], [40, 50, 60, 0]];
        let bmp = |pixels: Vec<u8>| Bmp::new(2, 1, 8, pixels).palette(&two_colours).build();
        assert_eq!(
            decode_ok(&bmp(vec![1, 0, 0, 0]), 2, 1),
            [60, 50, 40, 255, 30, 20, 10, 255]
        );
        assert!(is_malformed(&bmp(vec![1, 200, 0, 0])));
        assert!(is_malformed(&bmp(vec![2, 0, 0, 0])));
        // Padding bytes are not pixels and may hold anything.
        assert!(crate::decode(&bmp(vec![1, 0, 200, 200])).is_ok());

        // Four bits can name sixteen colours; this file has three.
        let three = [[0; 4], [1; 4], [2; 4]];
        let bmp = |pixels: Vec<u8>| Bmp::new(2, 1, 4, pixels).palette(&three).build();
        assert!(crate::decode(&bmp(vec![0x21, 0, 0, 0])).is_ok());
        assert!(is_malformed(&bmp(vec![0x23, 0, 0, 0])));
    }

    #[test]
    fn broken_structure_is_malformed() {
        let valid = Bmp::new(1, 1, 24, vec![1, 2, 3, 0]).build();
        assert_eq!(valid.len(), 58);
        assert_eq!(decode_ok(&valid, 1, 1), [3, 2, 1, 255]);
        let with = |offset: usize, value: &[u8]| {
            let mut bytes = valid.clone();
            bytes[offset..offset + value.len()].copy_from_slice(value);
            bytes
        };
        // The number of colour planes is always 1.
        assert!(is_malformed(&with(26, &0u16.to_le_bytes())));
        assert!(is_malformed(&with(26, &2u16.to_le_bytes())));
        // Pixels that start inside the file header or the information header.
        for offset in [0u32, 13, 14, 53] {
            assert!(is_malformed(&with(10, &offset.to_le_bytes())), "{offset}");
        }
        // An information header longer than the file.
        for header_len in [124u32, 45, u32::MAX] {
            assert!(
                is_malformed(&with(14, &header_len.to_le_bytes())),
                "{header_len}"
            );
        }
        // A longer header that is present is fine, with the pixels behind it.
        let mut longer = valid.clone();
        longer.splice(54..54, [0; 68]);
        longer[14..18].copy_from_slice(&108u32.to_le_bytes());
        assert!(is_malformed(&longer));
        longer[10..14].copy_from_slice(&122u32.to_le_bytes());
        assert_eq!(decode_ok(&longer, 1, 1), [3, 2, 1, 255]);

        // Pixels that start inside the palette.
        let indexed = Bmp::new(1, 1, 8, vec![1, 0, 0, 0])
            .palette(&[[10, 20, 30, 0], [40, 50, 60, 0]])
            .build();
        assert_eq!(decode_ok(&indexed, 1, 1), [60, 50, 40, 255]);
        let mut overlapping = indexed.clone();
        overlapping[10..14].copy_from_slice(&58u32.to_le_bytes());
        assert!(is_malformed(&overlapping));

        // Pixels that start inside the channel masks behind a 40-byte header.
        let mut masked = Bmp::new(1, 1, 32, vec![1, 2, 3, 4]);
        masked.compression = BI_BITFIELDS;
        let mut masked = masked.build();
        let masks = [0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF].map(u32::to_le_bytes);
        masked.splice(54..54, masks.concat());
        assert!(is_malformed(&masked));
        masked[10..14].copy_from_slice(&66u32.to_le_bytes());
        assert_eq!(decode_ok(&masked, 1, 1), [3, 2, 1, 4]);

        // The oldest header has its plane count checked too.
        let mut core = b"BM".to_vec();
        core.extend([0; 8]);
        core.extend(32u32.to_le_bytes());
        core.extend(12u32.to_le_bytes());
        core.extend([1, 0, 1, 0, 0, 0, 1, 0]);
        core.extend([10, 20, 30, 40, 50, 60, 0x80, 0, 0, 0]);
        assert!(is_malformed(&core));
        core[22] = 1;
        assert_eq!(decode_ok(&core, 1, 1), [60, 50, 40, 255]);
    }

    #[test]
    fn low_bit_depths() {
        for bits in [1u16, 4, 8] {
            let colours = 1u32 << bits;
            let palette: Vec<[u8; 4]> = (0..colours)
                .map(|i| [i as u8, (i * 3) as u8, 255 - i as u8, 0])
                .collect();
            // Widths that end inside a byte and inside the four-byte padding.
            for (width, height) in [(11u32, 3u32), (8, 2), (1, 1), (33, 2)] {
                let indices: Vec<u8> = pattern(width, height, 1)
                    .iter()
                    .map(|value| (u32::from(*value) % colours) as u8)
                    .collect();
                let packed = pack(&indices, width, bits.into());
                let row_len = packed.len() / height as usize;
                let expected: Vec<u8> = indices
                    .iter()
                    .flat_map(|&index| {
                        let [blue, green, red, _] = palette[usize::from(index)];
                        [red, green, blue, 255]
                    })
                    .collect();

                let top_down = Bmp::new(
                    width as i32,
                    -(height as i32),
                    bits,
                    bmp_rows(&packed, row_len),
                )
                .palette(&palette)
                .build();
                assert_eq!(decode_ok(&top_down, width, height), expected, "{bits} bits");
                let flipped = flip(&packed, row_len);
                let bottom_up = Bmp::new(
                    width as i32,
                    height as i32,
                    bits,
                    bmp_rows(&flipped, row_len),
                )
                .palette(&palette)
                .build();
                assert_eq!(
                    decode_ok(&bottom_up, width, height),
                    expected,
                    "{bits} bits"
                );
            }
        }
    }

    #[test]
    fn twenty_four_bits() {
        for (width, height) in [(5u32, 3u32), (4, 2), (1, 1)] {
            let stored = pattern(width, height, 3);
            let row_len = width as usize * 3;
            let bytes = Bmp::new(
                width as i32,
                -(height as i32),
                24,
                bmp_rows(&stored, row_len),
            )
            .build();
            assert_eq!(decode_ok(&bytes, width, height), opaque(&stored));
            let flipped = flip(&stored, row_len);
            let bytes =
                Bmp::new(width as i32, height as i32, 24, bmp_rows(&flipped, row_len)).build();
            assert_eq!(decode_ok(&bytes, width, height), opaque(&stored));
        }
    }

    #[test]
    fn thirty_two_bits_without_alpha() {
        let mut stored = pattern(3, 2, 4);
        for pixel in stored.as_chunks_mut::<4>().0 {
            pixel[3] = 0;
        }
        let bytes = Bmp::new(3, -2, 32, stored.clone()).build();
        let expected: Vec<u8> = stored
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[2], pixel[1], pixel[0], 255])
            .collect();
        assert_eq!(decode_ok(&bytes, 3, 2), expected);
    }

    #[test]
    fn thirty_two_bits_with_alpha() {
        // One pixel with a non-zero fourth byte makes every fourth byte alpha, zeros included.
        let stored = vec![1, 2, 3, 0, 4, 5, 6, 0, 7, 8, 9, 128, 10, 11, 12, 0];
        let bytes = Bmp::new(2, -2, 32, stored).build();
        let expected = [3, 2, 1, 0, 6, 5, 4, 0, 9, 8, 7, 128, 12, 11, 10, 0];
        assert_eq!(decode_ok(&bytes, 2, 2), expected);
    }

    #[test]
    fn compressed_data_is_unsupported() {
        // 1 and 2 are the run-length encodings, 4 and 5 embedded JPEG and PNG.
        for (compression, bits) in [(1, 8), (2, 4), (4, 24), (5, 24), (3, 16), (6, 32)] {
            let mut bmp = Bmp::new(1, 1, bits, vec![0; 4]).palette(&[[0; 4]; 2]);
            bmp.compression = compression;
            assert!(
                matches!(
                    crate::decode(&bmp.build()),
                    Err(ImageError::UnsupportedFormat { .. })
                ),
                "compression {compression}"
            );
        }
    }

    #[test]
    fn other_bit_depths_are_unsupported() {
        for bits in [0, 2, 16, 48, 64] {
            let bytes = Bmp::new(1, 1, bits, vec![0; 8]).build();
            assert!(
                matches!(
                    crate::decode(&bytes),
                    Err(ImageError::UnsupportedFormat { .. })
                ),
                "{bits} bits"
            );
        }
    }

    #[test]
    fn spelled_out_channel_masks() {
        let stored = vec![1, 2, 3, 200, 4, 5, 6, 100];
        let mut bmp = Bmp::new(2, 1, 32, stored);
        bmp.compression = BI_BITFIELDS;
        let mut bytes = bmp.build();
        // The masks follow the 40-byte header; the pixels move behind them.
        let masks = [0x00FF_0000u32, 0x0000_FF00, 0x0000_00FF].map(u32::to_le_bytes);
        bytes.splice(54..54, masks.concat());
        bytes[10..14].copy_from_slice(&66u32.to_le_bytes());
        assert_eq!(decode_ok(&bytes, 2, 1), [3, 2, 1, 200, 6, 5, 4, 100]);

        // Any other layout is refused rather than misread.
        bytes[54..58].copy_from_slice(&0x0000_00FFu32.to_le_bytes());
        assert!(matches!(
            crate::decode(&bytes),
            Err(ImageError::UnsupportedFormat { .. })
        ));
    }

    #[test]
    fn oldest_header() {
        // 12-byte header: 16-bit dimensions, 3-byte palette entries, always bottom-up.
        let mut bytes = b"BM".to_vec();
        bytes.extend([0; 8]);
        bytes.extend(32u32.to_le_bytes());
        bytes.extend(12u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend([10, 20, 30, 40, 50, 60]);
        bytes.extend([0b0100_0000, 0, 0, 0]);
        assert_eq!(decode_ok(&bytes, 2, 1), [30, 20, 10, 255, 60, 50, 40, 255]);
    }

    #[test]
    fn broken_headers_are_malformed() {
        let valid = Bmp::new(2, 2, 24, vec![0; 16]);
        let malformed = |bytes: &[u8]| {
            matches!(
                crate::decode(bytes),
                Err(ImageError::Malformed {
                    format: ImageFormat::Bmp,
                    ..
                })
            )
        };
        assert!(malformed(&Bmp::new(0, 2, 24, vec![]).build()));
        assert!(malformed(&Bmp::new(2, 0, 24, vec![]).build()));
        assert!(malformed(&Bmp::new(-2, 2, 24, vec![0; 16]).build()));
        // An information header of an impossible size.
        let mut bytes = valid.build();
        bytes[14..18].copy_from_slice(&20u32.to_le_bytes());
        assert!(malformed(&bytes));
        // Pixels that start past the end of the file.
        let mut bytes = valid.build();
        bytes[10..14].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(malformed(&bytes));
        // A palette that the file is too short for.
        let short = Bmp::new(1, 1, 8, vec![]).build();
        assert!(malformed(&short));
    }
}
