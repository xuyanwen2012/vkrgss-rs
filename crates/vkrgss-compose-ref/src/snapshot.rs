//! The pixels of `Graphics.snap_to_bitmap`.

use vkrgss_proto::FramePacket;

use crate::{ComposeError, Compositor, Frame};

/// Which runtime's measured rule the pixels of a snapshot follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapshotRule {
    /// RGSS2, of RPG Maker VX: the composed frame, darkened, with an alpha of 255.
    Rgss2,
    /// RGSS3, of RPG Maker VX Ace: as RGSS2, and every byte 0 at brightness 0.
    Rgss3,
}

/// The pixels of a snapshot of `packet`, which stands alone: every bitmap and table its ops
/// refer to is created and sent whole in it. The frame is composed as every frame is,
/// darkened by the packet's brightness (`(v * b) >> 8`, unchanged at 255), and every alpha
/// is 255, whatever the alpha of the composed frame. By the rule of RGSS3 every byte of
/// every pixel is 0 when the brightness is 0.
///
/// Provenance of RGSS2: oracle measurement 2026-10-12, RGSS202E versions 2.0.0.1 and
/// 2.0.2.1, `vx-v0-REPORT.md` section 6 (measured with frames of alpha 255, and not at
/// brightness 0). Provenance of RGSS3: oracle measurement 2026-10-13, versions 3.0.0.1 and
/// 3.0.1.1, `ace-v1-sprite-REPORT.md` (256 snapshots without a wrong pixel).
pub fn snapshot(packet: &FramePacket, rule: SnapshotRule) -> Result<Frame, ComposeError> {
    let mut frame = Compositor::new().render(packet)?;
    let dark = match rule {
        SnapshotRule::Rgss2 => false,
        SnapshotRule::Rgss3 => packet.brightness == 0,
    };
    if dark {
        frame.rgba.fill(0);
    } else {
        for pixel in frame.rgba.as_chunks_mut::<4>().0 {
            pixel[3] = 255;
        }
    }
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use vkrgss_pixel::Rgba;
    use vkrgss_proto::{BitmapId, Color, FrameRate, GuestToHost, Op, Rect, Size};

    use super::*;
    use crate::fixtures::{bitmap, every_level, sprite};

    const BOTH: [SnapshotRule; 2] = [SnapshotRule::Rgss2, SnapshotRule::Rgss3];

    /// A packet of 8 by 6 pixels with `ops` at `brightness`.
    fn packet(brightness: u8, ops: Vec<Op>) -> FramePacket {
        FramePacket {
            frame_index: 0,
            logical_size: Size::new(8, 6),
            frame_rate: FrameRate::DEFAULT,
            brightness,
            tables: vec![],
            bitmaps: vec![],
            ops,
        }
    }

    /// A packet whose frame is the opaque colour (200, 100, 50).
    fn coloured(brightness: u8) -> FramePacket {
        let clear = Op::Clear {
            clip: Rect::new(0, 0, 8, 6),
            color: Color::new(200, 100, 50, 255),
        };
        packet(brightness, vec![clear])
    }

    /// Whether every pixel of the snapshot of `packet` by `rule` is `pixel`.
    fn all(packet: &FramePacket, rule: SnapshotRule, pixel: Rgba) -> bool {
        let frame = snapshot(packet, rule).unwrap();
        assert_eq!(frame.size, packet.logical_size);
        frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|found| *found == pixel)
    }

    /// Scenarios "Darkened, measured on the originals" and "Full brightness".
    #[test]
    fn a_snapshot_is_darkened_by_the_brightness_and_opaque() {
        for rule in BOTH {
            assert!(all(&coloured(100), rule, [78, 39, 19, 255]), "{rule:?}");
            assert!(all(&coloured(255), rule, [200, 100, 50, 255]), "{rule:?}");
        }
    }

    /// Scenarios "Brightness 0 of RGSS3, measured on the original" and "Brightness 0 of
    /// RGSS2".
    #[test]
    fn at_brightness_0_a_snapshot_of_rgss3_is_all_zero() {
        assert!(all(&coloured(0), SnapshotRule::Rgss3, [0, 0, 0, 0]));
        assert!(all(&coloured(1), SnapshotRule::Rgss3, [0, 0, 0, 255]));
        assert!(all(&coloured(0), SnapshotRule::Rgss2, [0, 0, 0, 255]));
    }

    /// Scenario "Nothing drawn, measured on the original of RGSS3".
    #[test]
    fn a_snapshot_of_nothing_is_opaque_black_in_rgss3() {
        assert!(all(
            &packet(255, vec![]),
            SnapshotRule::Rgss3,
            [0, 0, 0, 255]
        ));
        assert!(all(&packet(0, vec![]), SnapshotRule::Rgss3, [0, 0, 0, 0]));
    }

    /// Scenario "Translucent over nothing, measured on the original of RGSS3".
    #[test]
    fn a_translucent_quad_over_nothing_is_opaque_in_a_snapshot() {
        let id = BitmapId::new(0, 0);
        let mut packet = packet(255, vec![Op::Quad(sprite(id, 4, 3, 2, 1))]);
        packet.bitmaps = bitmap(id, 4, 3, [255, 255, 255, 128].repeat(12)).into();
        let frame = snapshot(&packet, SnapshotRule::Rgss3).unwrap();
        for y in 0..6 {
            for x in 0..8 {
                let inside = (2..6).contains(&x) && (1..4).contains(&y);
                let expected = if inside { 128 } else { 0 };
                assert_eq!(frame.pixel(x, y), [expected, expected, expected, 255]);
            }
        }
    }

    /// Scenario "Every level, measured on the original of RGSS3".
    #[test]
    fn every_level_is_darkened_at_every_brightness() {
        let GuestToHost::Frame(full) = every_level(255) else {
            unreachable!("a frame message");
        };
        let full = Compositor::new().render(&full).unwrap();
        for brightness in 1..=254u8 {
            let GuestToHost::Frame(packet) = every_level(brightness) else {
                unreachable!("a frame message");
            };
            let frame = snapshot(&packet, SnapshotRule::Rgss3).unwrap();
            let pixels = frame.rgba.as_chunks::<4>().0.iter();
            for (found, composed) in pixels.zip(full.rgba.as_chunks::<4>().0) {
                let dark = |v: u8| ((u16::from(v) * u16::from(brightness)) >> 8) as u8;
                let expected = [dark(composed[0]), dark(composed[1]), dark(composed[2]), 255];
                assert_eq!(*found, expected, "brightness {brightness}");
            }
        }
        // Every channel value is among the composed ones.
        for channel in 0..3 {
            let mut levels: Vec<u8> = full
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| p[channel])
                .collect();
            levels.sort_unstable();
            levels.dedup();
            assert_eq!(levels.len(), 256, "channel {channel}");
        }
    }
}
