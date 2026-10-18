//! The comparison of the captured frames: difference pictures and the summary.

use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

/// A difference of 1 is drawn as this much, so that small differences can be seen.
const AMPLIFICATION: u8 = 32;

/// A picture as RGBA bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Picture {
    /// Reads a PNG file of 8-bit RGB or RGBA.
    pub(crate) fn read(path: &Path) -> Result<Picture> {
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

    pub(crate) fn write(&self, path: &Path) -> Result<()> {
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
pub(crate) struct Difference {
    /// The number of pixels that differ in red, green or blue.
    pub differing_pixels: u64,
    /// The largest difference of any of these channels.
    pub largest_difference: u8,
    /// The absolute difference of every channel, amplified, on opaque black.
    pub picture: Picture,
}

/// Compares the colours of two pictures of the same size. Alpha is not compared: a picture
/// of a screen has none.
pub(crate) fn compare(original: &Picture, vkrgss: &Picture) -> Result<Difference> {
    ensure!(
        (original.width, original.height) == (vkrgss.width, vkrgss.height),
        "the pictures differ in size: {} by {} in the original, {} by {} in vkrgss",
        original.width,
        original.height,
        vkrgss.width,
        vkrgss.height
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
        .zip(vkrgss.rgba.as_chunks::<4>().0.iter())
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
pub(super) fn original_file(frame: u64) -> String {
    format!("frames/{frame}-original.png")
}

pub(crate) fn vkrgss_file(frame: u64) -> String {
    format!("frames/{frame}-vkrgss.png")
}

fn diff_file(frame: u64) -> String {
    format!("frames/{frame}-diff.png")
}

/// Which runtimes a run of the harness ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Sides {
    Both,
    VkrgssOnly,
    OriginalOnly,
}

/// What a report says about the run besides its pictures.
pub(super) struct Run<'a> {
    pub sides: Sides,
    /// The frames whose picture of the original was taken late, after the transition they
    /// fall inside.
    pub late: &'a [u64],
    /// Whether the probe replaced `Thread.new`.
    pub no_threads: bool,
}

/// Why the picture of a late frame is not compared.
const LATE: &str = "inside a transition";

/// Compares the pictures of `frames` that both runs left below `out`, writes the difference
/// pictures and `summary.json`, and prints the table. A late frame and a frame whose two
/// pictures differ in size are listed as not comparable and counted neither as equal nor as
/// different. When only one side ran there is nothing to compare, and the summary lists the
/// pictures of that side.
pub(super) fn write(out: &Path, frames: &[u64], run: &Run) -> Result<()> {
    let both = run.sides == Sides::Both;
    if both {
        println!(
            "{:>8}  {:>16}  {:>18}",
            "frame", "differing pixels", "largest difference"
        );
    }
    let (mut equal, mut different, mut not_comparable) = (0u64, 0u64, 0u64);
    let mut rows = Vec::new();
    for &frame in frames {
        let mut row = json!({ "frame": frame });
        if run.sides != Sides::OriginalOnly {
            row["vkrgss"] = Value::from(vkrgss_file(frame));
        }
        if run.sides != Sides::VkrgssOnly {
            row["original"] = Value::from(original_file(frame));
        }
        let late = run.sides != Sides::VkrgssOnly && run.late.contains(&frame);
        let mut reason = late.then(|| LATE.to_owned());
        if both && reason.is_none() {
            let original = Picture::read(&out.join(original_file(frame)))?;
            let vkrgss = Picture::read(&out.join(vkrgss_file(frame)))?;
            match compare(&original, &vkrgss) {
                Ok(difference) => {
                    difference.picture.write(&out.join(diff_file(frame)))?;
                    println!(
                        "{frame:>8}  {:>16}  {:>18}",
                        difference.differing_pixels, difference.largest_difference
                    );
                    row["diff"] = Value::from(diff_file(frame));
                    row["differing_pixels"] = Value::from(difference.differing_pixels);
                    row["largest_difference"] = Value::from(difference.largest_difference);
                    if difference.differing_pixels == 0 {
                        equal += 1;
                    } else {
                        different += 1;
                    }
                }
                // The only failure of a comparison is that the sizes differ.
                Err(error) => reason = Some(error.to_string()),
            }
        }
        if let Some(reason) = reason {
            println!("{frame:>8}  not comparable: {reason}");
            row["not_comparable"] = Value::from(reason);
            not_comparable += 1;
        }
        rows.push(row);
    }
    let mut report = json!({ "frames": rows });
    if both {
        println!("equal: {equal}, different: {different}, not comparable: {not_comparable}");
        report["totals"] = json!({
            "equal": equal,
            "different": different,
            "not_comparable": not_comparable,
        });
    }
    if run.no_threads {
        println!("--no-threads was used: Thread.new ran its block at once in the caller");
        report["no_threads"] = Value::from(true);
    }
    let summary = out.join("summary.json");
    let text = serde_json::to_string_pretty(&report).context("the summary is plain data")?;
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
        gradient().write(&out.path().join(vkrgss_file(10))).unwrap();
        other.write(&out.path().join(vkrgss_file(20))).unwrap();

        let run = Run {
            sides: Sides::Both,
            late: &[],
            no_threads: false,
        };
        write(out.path(), &[10, 20], &run).unwrap();

        let summary = fs::read_to_string(out.path().join("summary.json")).unwrap();
        let summary: Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(
            summary,
            json!({ "totals": { "equal": 1, "different": 1, "not_comparable": 0 }, "frames": [
                {
                    "frame": 10,
                    "original": "frames/10-original.png",
                    "vkrgss": "frames/10-vkrgss.png",
                    "diff": "frames/10-diff.png",
                    "differing_pixels": 0,
                    "largest_difference": 0,
                },
                {
                    "frame": 20,
                    "original": "frames/20-original.png",
                    "vkrgss": "frames/20-vkrgss.png",
                    "diff": "frames/20-diff.png",
                    "differing_pixels": 1,
                    "largest_difference": 3,
                },
            ] })
        );
        let diff = Picture::read(&out.path().join("frames/20-diff.png")).unwrap();
        assert_eq!(diff.rgba[..4], [96, 0, 0, 255]);
    }

    /// The late-frame scenario of `oracle-harness`, and a frame whose pictures differ in size.
    #[test]
    fn a_late_frame_and_pictures_of_different_sizes_are_listed_as_not_comparable() {
        let out = tempfile::tempdir().unwrap();
        fs::create_dir(out.path().join("frames")).unwrap();
        let mut other = gradient();
        other.rgba[0] += 3;
        let mut turned = gradient();
        (turned.width, turned.height) = (4, 8);
        for (frame, vkrgss) in [(5, gradient()), (10, other), (20, turned)] {
            gradient()
                .write(&out.path().join(original_file(frame)))
                .unwrap();
            vkrgss.write(&out.path().join(vkrgss_file(frame))).unwrap();
        }

        let run = Run {
            sides: Sides::Both,
            late: &[10],
            no_threads: true,
        };
        write(out.path(), &[5, 10, 20], &run).unwrap();

        let summary = fs::read_to_string(out.path().join("summary.json")).unwrap();
        let summary: Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(
            summary["totals"],
            json!({ "equal": 1, "different": 0, "not_comparable": 2 })
        );
        assert_eq!(summary["no_threads"], json!(true));
        assert_eq!(
            summary["frames"][1],
            json!({
                "frame": 10,
                "original": "frames/10-original.png",
                "vkrgss": "frames/10-vkrgss.png",
                "not_comparable": "inside a transition",
            })
        );
        let sizes = summary["frames"][2]["not_comparable"].as_str().unwrap();
        assert!(
            sizes.contains("8 by 4") && sizes.contains("4 by 8"),
            "{sizes}"
        );
        assert!(summary["frames"][2].get("differing_pixels").is_none());
        assert!(!out.path().join("frames/10-diff.png").exists());
    }

    /// The report of `--original-only`: the pictures are listed and nothing is compared.
    #[test]
    fn the_summary_of_the_original_alone_lists_its_pictures() {
        let out = tempfile::tempdir().unwrap();
        let run = Run {
            sides: Sides::OriginalOnly,
            late: &[20],
            no_threads: false,
        };
        // No picture is read, so none has to be there.
        write(out.path(), &[10, 20], &run).unwrap();

        let summary = fs::read_to_string(out.path().join("summary.json")).unwrap();
        let summary: Value = serde_json::from_str(&summary).unwrap();
        assert_eq!(
            summary,
            json!({ "frames": [
                { "frame": 10, "original": "frames/10-original.png" },
                {
                    "frame": 20,
                    "original": "frames/20-original.png",
                    "not_comparable": "inside a transition",
                },
            ] })
        );
    }
}
