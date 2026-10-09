//! Test inputs, synthesised: no image file is committed.

use png::{BitDepth, ColorType};

/// Deterministic noise: `width * height * channels` bytes.
pub(crate) fn pattern(width: u32, height: u32, channels: u32) -> Vec<u8> {
    let mut state = 0x2545_F491u32 ^ (width << 16) ^ (height << 8) ^ channels;
    (0..width * height * channels)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 11) as u8
        })
        .collect()
}

/// Encodes a PNG from samples laid out as the format stores them.
pub(crate) fn png(
    width: u32,
    height: u32,
    color: ColorType,
    depth: BitDepth,
    data: &[u8],
) -> Vec<u8> {
    png_with(width, height, color, depth, data, |_| {})
}

/// As [`png`], with a hook to set a palette, transparency or other chunks.
pub(crate) fn png_with(
    width: u32,
    height: u32,
    color: ColorType,
    depth: BitDepth,
    data: &[u8],
    configure: impl FnOnce(&mut png::Encoder<'_, &mut Vec<u8>>),
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(color);
    encoder.set_depth(depth);
    configure(&mut encoder);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(data).unwrap();
    writer.finish().unwrap();
    out
}

/// Packs samples of `depth` bits into rows, most significant bits first, each row padded to a
/// whole byte, as PNG and BMP both store low bit depths.
pub(crate) fn pack(samples: &[u8], width: u32, depth: u32) -> Vec<u8> {
    let mut out = Vec::new();
    for row in samples.chunks(width as usize) {
        let mut packed = vec![0u8; (row.len() * depth as usize).div_ceil(8)];
        for (x, sample) in row.iter().enumerate() {
            let bit = x * depth as usize;
            packed[bit / 8] |= sample << (8 - depth as usize - bit % 8);
        }
        out.extend(packed);
    }
    out
}

/// A zlib stream that stores `data` without compressing it.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut blocks = data.chunks(0xFFFF).peekable();
    while let Some(block) = blocks.next() {
        let len = block.len() as u16;
        out.push(u8::from(blocks.peek().is_none()));
        out.extend(len.to_le_bytes());
        out.extend((!len).to_le_bytes());
        out.extend(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in data {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    out.extend((b << 16 | a).to_be_bytes());
    out
}

/// Encodes 8-bit red, green, blue, alpha pixels as an Adam7-interlaced PNG. The `png` crate
/// does not write interlaced images, so the passes are laid out here.
pub(crate) fn interlaced_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    // Per pass: first column, first row, column step, row step.
    const PASSES: [(u32, u32, usize, usize); 7] = [
        (0, 0, 8, 8),
        (4, 0, 8, 8),
        (0, 4, 4, 8),
        (2, 0, 4, 4),
        (0, 2, 2, 4),
        (1, 0, 2, 2),
        (0, 1, 1, 2),
    ];
    let mut raw = Vec::new();
    for (first_x, first_y, step_x, step_y) in PASSES {
        if first_x >= width {
            continue;
        }
        for y in (first_y..height).step_by(step_y) {
            // Filter type 0: the row is stored as it is.
            raw.push(0);
            for x in (first_x..width).step_by(step_x) {
                let at = ((y * width + x) * 4) as usize;
                raw.extend(&rgba[at..at + 4]);
            }
        }
    }
    let mut info = png::Info::with_size(width, height);
    info.color_type = ColorType::Rgba;
    info.bit_depth = BitDepth::Eight;
    info.interlaced = true;
    let mut out = Vec::new();
    let mut writer = png::Encoder::with_info(&mut out, info)
        .unwrap()
        .write_header()
        .unwrap();
    writer
        .write_chunk(png::chunk::IDAT, &zlib_stored(&raw))
        .unwrap();
    writer.finish().unwrap();
    out
}

/// The CRC of a PNG chunk, over its type and data.
pub(crate) fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

/// Encodes a JPEG at the highest quality, with a hook to set subsampling or progressive mode.
pub(crate) fn jpeg(
    width: u16,
    height: u16,
    color: jpeg_encoder::ColorType,
    data: &[u8],
    configure: impl FnOnce(&mut jpeg_encoder::Encoder<&mut Vec<u8>>),
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut out, 100);
    configure(&mut encoder);
    encoder.encode(data, width, height, color).unwrap();
    out
}

/// A BMP file built byte by byte.
pub(crate) struct Bmp {
    pub(crate) width: i32,
    /// Negative for rows stored from the top.
    pub(crate) height: i32,
    pub(crate) bits: u16,
    pub(crate) compression: u32,
    /// Blue, green, red, filler.
    pub(crate) palette: Vec<[u8; 4]>,
    /// Rows in stored order, each padded to a multiple of four bytes.
    pub(crate) pixels: Vec<u8>,
}

impl Bmp {
    pub(crate) fn new(width: i32, height: i32, bits: u16, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            bits,
            compression: 0,
            palette: Vec::new(),
            pixels,
        }
    }

    pub(crate) fn palette(mut self, palette: &[[u8; 4]]) -> Self {
        self.palette = palette.to_vec();
        self
    }

    /// The file with the common 40-byte information header.
    pub(crate) fn build(&self) -> Vec<u8> {
        let pixel_offset = 14 + 40 + self.palette.len() * 4;
        let mut out = b"BM".to_vec();
        out.extend(((pixel_offset + self.pixels.len()) as u32).to_le_bytes());
        out.extend([0; 4]);
        out.extend((pixel_offset as u32).to_le_bytes());
        out.extend(40u32.to_le_bytes());
        out.extend(self.width.to_le_bytes());
        out.extend(self.height.to_le_bytes());
        out.extend(1u16.to_le_bytes());
        out.extend(self.bits.to_le_bytes());
        out.extend(self.compression.to_le_bytes());
        out.extend((self.pixels.len() as u32).to_le_bytes());
        out.extend([0; 8]);
        out.extend((self.palette.len() as u32).to_le_bytes());
        out.extend([0; 4]);
        out.extend(self.palette.iter().flatten());
        out.extend(&self.pixels);
        out
    }
}

/// Pads rows of `row_len` bytes to the four-byte multiple BMP stores.
pub(crate) fn bmp_rows(data: &[u8], row_len: usize) -> Vec<u8> {
    let mut out = Vec::new();
    for row in data.chunks(row_len) {
        out.extend(row);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    out
}
