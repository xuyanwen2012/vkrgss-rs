//! The comparison of the captured frames: difference pictures and the summary.

use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

/// The size of a frame of the original runtime.
pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 480;

/// A difference of 1 is drawn as this much, so that small differences can be seen.
const AMPLIFICATION: u8 = 32;

/// A picture as RGBA bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Picture {
    /// Reads a PNG file of 8-bit RGB or RGBA.
    pub fn read(path: &Path) -> Result<Picture> {
        let name = || path.display().to_string();
        let file = fs::File::open(path).with_context(name)?;
        let mut reader = png::Decoder::new(BufReader::new(file))
            .read_info()
            .with_context(name)?;
        let info = reader.info();
        let (width, height, colour) = (info.width, info.height, info.color_type);
        ensure!(
            info.bit_depth == png::BitDepth::Eight,
            "{}: not an 8-bit picture",
            name()
        );
        let size = reader
            .output_buffer_size()
            .with_context(|| format!("{}: too large", name()))?;
        let mut data = vec![0; size];
        reader.next_frame(&mut data).with_context(name)?;
        let rgba = match colour {
            png::ColorType::Rgba => data,
            png::ColorType::Rgb => data
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
                .collect(),
            other => bail!("{}: colour type {other:?} is not RGB or RGBA", name()),
        };
        Ok(Picture {
            width,
            height,
            rgba,
        })
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let name = || path.display().to_string();
        let file = fs::File::create(path).with_context(name)?;
        let mut encoder = png::Encoder::new(BufWriter::new(file), self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().with_context(name)?;
        writer.write_image_data(&self.rgba).with_context(name)?;
        writer.finish().with_context(name)
    }
}

/// How two pictures of one frame differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// The number of pixels that differ in red, green or blue.
    pub differing_pixels: u64,
    /// The largest difference of any of these channels.
    pub largest_difference: u8,
    /// The absolute difference of every channel, amplified, on opaque black.
    pub picture: Picture,
}

/// Compares the colours of two pictures of the same size. Alpha is not compared: a picture
/// of a screen has none.
pub fn compare(original: &Picture, vkxp: &Picture) -> Result<Difference> {
    ensure!(
        (original.width, original.height) == (vkxp.width, vkxp.height),
        "the pictures differ in size: {} by {} in the original, {} by {} in vkxp",
        original.width,
        original.height,
        vkxp.width,
        vkxp.height
    );
    let mut difference = Difference {
        differing_pixels: 0,
        largest_difference: 0,
        picture: Picture {
            width: original.width,
            height: original.height,
            rgba: Vec::with_capacity(original.rgba.len()),
        },
    };
    for (a, b) in original
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(vkxp.rgba.as_chunks::<4>().0.iter())
    {
        let channels = [0, 1, 2].map(|channel| a[channel].abs_diff(b[channel]));
        let largest = channels.into_iter().max().unwrap_or(0);
        difference.differing_pixels += u64::from(largest > 0);
        difference.largest_difference = difference.largest_difference.max(largest);
        let amplified = channels.map(|channel| channel.saturating_mul(AMPLIFICATION));
        difference.picture.rgba.extend(amplified);
        difference.picture.rgba.push(255);
    }
    Ok(difference)
}

/// The file names of one frame, below the output directory.
pub fn original_file(frame: u64) -> String {
    format!("frames/{frame}-original.png")
}

pub fn vkxp_file(frame: u64) -> String {
    format!("frames/{frame}-vkxp.png")
}

fn diff_file(frame: u64) -> String {
    format!("frames/{frame}-diff.png")
}

/// Compares the pictures of `frames` that both runs left below `out`, writes the difference
/// pictures and `summary.json`, and prints the table. With `vkxp_only` there is nothing to
/// compare and the summary lists the pictures of vkxp.
pub fn write(out: &Path, frames: &[u64], vkxp_only: bool) -> Result<()> {
    let mut rows = Vec::new();
    if !vkxp_only {
        println!(
            "{:>8}  {:>16}  {:>18}",
            "frame", "differing pixels", "largest difference"
        );
    }
    for &frame in frames {
        let mut row = json!({ "frame": frame, "vkxp": vkxp_file(frame) });
        if !vkxp_only {
            let original = Picture::read(&out.join(original_file(frame)))?;
            let vkxp = Picture::read(&out.join(vkxp_file(frame)))?;
            let difference = compare(&original, &vkxp).with_context(|| format!("frame {frame}"))?;
            difference.picture.write(&out.join(diff_file(frame)))?;
            println!(
                "{frame:>8}  {:>16}  {:>18}",
                difference.differing_pixels, difference.largest_difference
            );
            row["original"] = Value::from(original_file(frame));
            row["diff"] = Value::from(diff_file(frame));
            row["differing_pixels"] = Value::from(difference.differing_pixels);
            row["largest_difference"] = Value::from(difference.largest_difference);
        }
        rows.push(row);
    }
    let summary = out.join("summary.json");
    let text = serde_json::to_string_pretty(&json!({ "frames": rows }))
        .context("the summary is plain data")?;
    fs::write(&summary, text + "\n").with_context(|| summary.display().to_string())?;
    println!("report: {}", summary.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gradient() -> Picture {
        let rgba = (0..8u8 * 4)
            .flat_map(|pixel| [pixel * 8, 255 - pixel, pixel, 255])
            .collect();
        Picture {
            width: 8,
            height: 4,
            rgba,
        }
    }

    #[test]
    fn equal_pictures_have_no_differing_pixel() {
        let difference = compare(&gradient(), &gradient()).unwrap();
        assert_eq!(difference.differing_pixels, 0);
        assert_eq!(difference.largest_difference, 0);
        assert!(
            difference
                .picture
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| *pixel == [0, 0, 0, 255])
        );
    }

    #[test]
    fn one_pixel_that_differs_by_3_in_red_is_counted_once() {
        let mut other = gradient();
        let at = (2 * 8 + 5) * 4;
        other.rgba[at] += 3;
        let difference = compare(&gradient(), &other).unwrap();
        assert_eq!(difference.differing_pixels, 1);
        assert_eq!(difference.largest_difference, 3);
        assert_eq!(difference.picture.rgba[at..at + 4], [96, 0, 0, 255]);
    }

    #[test]
    fn pictures_of_different_sizes_are_not_compared() {
        let mut other = gradient();
        (other.width, other.height) = (4, 8);
        assert!(compare(&gradient(), &other).is_err());
    }

    #[test]
    fn the_summary_lists_the_numbers_of_every_frame() {
        let out = tempfile::tempdir().unwrap();
        fs::create_dir(out.path().join("frames")).unwrap();
        let mut other = gradient();
        other.rgba[0] += 3;
        for frame in [10, 20] {
            gradient()
                .write(&out.path().join(original_file(frame)))
                .unwrap();
        }
        gradient().write(&out.path().join(vkxp_file(10))).unwrap();
        other.write(&out.path().join(vkxp_file(20))).unwrap();

        write(out.path(), &[10, 20], false).unwrap();

        let summary = fs::read_to_string(out.path().join("summary.json")).unwrap();
        let summary: Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(
            summary,
            json!({ "frames": [
                {
                    "frame": 10,
                    "original": "frames/10-original.png",
                    "vkxp": "frames/10-vkxp.png",
                    "diff": "frames/10-diff.png",
                    "differing_pixels": 0,
                    "largest_difference": 0,
                },
                {
                    "frame": 20,
                    "original": "frames/20-original.png",
                    "vkxp": "frames/20-vkxp.png",
                    "diff": "frames/20-diff.png",
                    "differing_pixels": 1,
                    "largest_difference": 3,
                },
            ] })
        );
        let diff = Picture::read(&out.path().join("frames/20-diff.png")).unwrap();
        assert_eq!(diff.rgba[..4], [96, 0, 0, 255]);
    }
}
