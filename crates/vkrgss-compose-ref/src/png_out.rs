//! PNG output of frames.

use png::{BitDepth, ColorType, Compression, Encoder, EncodingError, Filter};

use crate::Frame;

/// Encodes a frame as an 8-bit RGBA PNG.
///
/// The encoder settings are fixed, so the same frame always gives the same bytes with the same
/// version of the `png` crate.
pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, EncodingError> {
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes, frame.size.width, frame.size.height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_compression(Compression::High);
    encoder.set_filter(Filter::Paeth);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&frame.rgba)?;
    writer.finish()?;
    Ok(bytes)
}
