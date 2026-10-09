//! Packet validation and the encoding of ops into the records the shader reads.
//!
//! Validation happens here, on the CPU, before anything is written for the device: the checks
//! and their order are those of the reference compositor, so both reject the same packets with
//! the same error. The shader then never sees an invalid slot or source rectangle.

use vkxp_proto::{BlendMode, Op, Rect, Size};

use crate::VkError;
use crate::bitmaps::BitmapTable;

/// Largest logical width or height a compositor accepts.
pub(crate) const MAX_LOGICAL_DIMENSION: u32 = 16384;

/// Number of 32-bit words in one record; `OpRecord` in `shaders/compose.comp` has this layout.
pub(crate) const RECORD_WORDS: usize = 23;

/// Values of the `kind` word. They match the `OP_*` constants in `shaders/compose.comp`.
mod op_kind {
    pub const CLEAR: u32 = 0;
    pub const QUAD: u32 = 1;
    pub const RECT_EFFECT: u32 = 2;
}

/// The `blend` word of a quad. It matches the `BLEND_*` constants in `shaders/compose.comp`.
const fn blend_mode(blend: BlendMode) -> u32 {
    match blend {
        BlendMode::Normal => 0,
        BlendMode::Add => 1,
        BlendMode::Subtract => 2,
    }
}

/// Rejects a logical size outside 1 to [`MAX_LOGICAL_DIMENSION`] per side.
pub(crate) fn check_logical_size(size: Size) -> Result<(), VkError> {
    let valid = 1..=MAX_LOGICAL_DIMENSION;
    if valid.contains(&size.width) && valid.contains(&size.height) {
        Ok(())
    } else {
        Err(VkError::InvalidLogicalSize {
            size,
            limit: MAX_LOGICAL_DIMENSION,
        })
    }
}

/// A non-empty rectangle of screen pixels: `x0 <= x < x1` and `y0 <= y < y1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClipRect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl ClipRect {
    /// The part of `clip` on a screen of `size`, or `None` when no pixel is.
    pub fn on_screen(clip: Rect, size: Size) -> Option<ClipRect> {
        let span = |start: i32, length: u32, limit: u32| {
            let start = i64::from(start);
            let low = start.max(0);
            let high = (start + i64::from(length)).min(i64::from(limit));
            (low < high).then_some((low as u32, high as u32))
        };
        let (x0, x1) = span(clip.x, clip.width, size.width)?;
        let (y0, y1) = span(clip.y, clip.height, size.height)?;
        Some(ClipRect { x0, y0, x1, y1 })
    }

    #[cfg(test)]
    pub fn contains(&self, x: u32, y: u32) -> bool {
        (self.x0..self.x1).contains(&x) && (self.y0..self.y1).contains(&y)
    }
}

/// One validated op as the shader reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpRecord {
    /// The op's clip rectangle intersected with the screen.
    pub clip: ClipRect,
    words: [u32; RECORD_WORDS],
}

impl OpRecord {
    fn new(kind: u32, clip: ClipRect) -> OpRecord {
        let mut words = [0; RECORD_WORDS];
        words[0] = kind;
        words[1..5].copy_from_slice(&[clip.x0, clip.y0, clip.x1, clip.y1]);
        OpRecord { clip, words }
    }

    /// The record's words, in the order of the fields of the GLSL `OpRecord`.
    pub fn words(&self) -> &[u32; RECORD_WORDS] {
        &self.words
    }
}

/// Checks every op, in order, and encodes those that can change a pixel.
///
/// An op whose clip misses the screen, and a quad with an empty source rectangle, is checked
/// like any other but yields no record: it covers no pixel. So a record's clip is never empty
/// and a quad record always names a slot that holds an image.
pub(crate) fn encode_ops(
    table: &BitmapTable,
    size: Size,
    ops: &[Op],
) -> Result<Vec<OpRecord>, VkError> {
    let mut records = Vec::with_capacity(ops.len());
    for (index, op) in ops.iter().enumerate() {
        let clip = ClipRect::on_screen(op.clip(), size);
        match op {
            Op::Clear { color, .. } => {
                let Some(clip) = clip else { continue };
                let mut record = OpRecord::new(op_kind::CLEAR, clip);
                record.words[18] = u32::from_le_bytes(color.to_rgba());
                records.push(record);
            }
            Op::Quad(quad) => {
                let bitmap = table
                    .get(quad.bitmap)
                    .map_err(|source| VkError::InvalidOpBitmap { index, source })?;
                if !bitmap.bounds().contains_rect(&quad.src) {
                    return Err(VkError::SourceOutOfBounds {
                        index,
                        bitmap: quad.bitmap,
                        src: quad.src,
                        width: bitmap.width,
                        height: bitmap.height,
                    });
                }
                let Some(clip) = clip else { continue };
                if quad.src.is_empty() {
                    continue;
                }
                let mut record = OpRecord::new(op_kind::QUAD, clip);
                let (src, t) = (quad.src, quad.transform);
                record.words[5] = quad.bitmap.index;
                // Inside the bitmap and not empty, so the origin is not negative and every
                // value is at most the largest image dimension.
                record.words[6..10].copy_from_slice(&[
                    src.x as u32,
                    src.y as u32,
                    src.width,
                    src.height,
                ]);
                let coefficients = [t.xx, t.xy, t.yx, t.yy, t.tx, t.ty];
                record.words[10..16].copy_from_slice(&coefficients.map(|c| c.to_bits() as u32));
                record.words[16] = u32::from(quad.opacity);
                record.words[17] = blend_mode(quad.blend);
                records.push(record);
            }
            Op::RectEffect(effect) => {
                let Some(clip) = clip else { continue };
                let mut record = OpRecord::new(op_kind::RECT_EFFECT, clip);
                let tone = effect.tone;
                record.words[18] = u32::from_le_bytes(effect.color.to_rgba());
                // Sign-extended: the shader reads these three words as signed integers.
                let offsets = [tone.red, tone.green, tone.blue];
                record.words[19..22].copy_from_slice(&offsets.map(|c| i32::from(c) as u32));
                record.words[22] = u32::from(tone.gray);
                records.push(record);
            }
            Op::Plane { .. } | Op::TilemapLayer { .. } => {
                return Err(VkError::UnsupportedOp {
                    kind: op.kind(),
                    index,
                });
            }
        }
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use vkxp_proto::{
        BitmapDelta, BitmapId, Color, Fixed, OpKind, Quad, RectEffect, Tone, Transform,
    };

    use super::*;
    use crate::{BitmapError, SlotState};

    const SIZE: Size = Size::new(40, 30);
    const SCREEN: Rect = Rect::from_size(SIZE);
    const ID: BitmapId = BitmapId::new(3, 1);

    /// A table holding an 8x6 bitmap as `ID`.
    fn table() -> BitmapTable {
        let mut table = BitmapTable::new(4096);
        table
            .apply(&BitmapDelta::Create {
                id: ID,
                width: 8,
                height: 6,
            })
            .unwrap();
        table
    }

    fn quad(bitmap: BitmapId, src: Rect) -> Op {
        Op::Quad(Quad {
            clip: SCREEN,
            bitmap,
            src,
            transform: Transform::IDENTITY,
            opacity: 255,
            blend: BlendMode::Normal,
        })
    }

    fn clear(clip: Rect) -> Op {
        Op::Clear {
            clip,
            color: Color::new(1, 2, 3, 4),
        }
    }

    #[test]
    fn logical_size_must_be_between_one_and_the_limit() {
        for size in [
            Size::new(1, 1),
            Size::new(641, 479),
            Size::new(MAX_LOGICAL_DIMENSION, MAX_LOGICAL_DIMENSION),
        ] {
            assert_eq!(check_logical_size(size), Ok(()));
        }
        for size in [
            Size::new(0, 10),
            Size::new(10, 0),
            Size::new(MAX_LOGICAL_DIMENSION + 1, 1),
            Size::new(1, u32::MAX),
        ] {
            assert_eq!(
                check_logical_size(size),
                Err(VkError::InvalidLogicalSize { size, limit: 16384 })
            );
        }
    }

    #[test]
    fn clip_is_intersected_with_the_screen() {
        let on_screen = |x, y, w, h| ClipRect::on_screen(Rect::new(x, y, w, h), SIZE);
        let rect = |x0, y0, x1, y1| Some(ClipRect { x0, y0, x1, y1 });
        assert_eq!(on_screen(0, 0, 40, 30), rect(0, 0, 40, 30));
        assert_eq!(on_screen(5, 6, 7, 8), rect(5, 6, 12, 14));
        assert_eq!(on_screen(-5, -6, 7, 8), rect(0, 0, 2, 2));
        assert_eq!(on_screen(39, 29, 100, 100), rect(39, 29, 40, 30));
        assert_eq!(
            on_screen(i32::MIN, i32::MIN, u32::MAX, u32::MAX),
            rect(0, 0, 40, 30)
        );
        assert_eq!(on_screen(i32::MAX, 0, u32::MAX, 1), None);
        // Touching the screen from outside, or empty, covers no pixel.
        assert_eq!(on_screen(40, 0, 5, 5), None);
        assert_eq!(on_screen(0, 30, 5, 5), None);
        assert_eq!(on_screen(-5, 0, 5, 5), None);
        assert_eq!(on_screen(0, -5, 5, 5), None);
        assert_eq!(on_screen(3, 3, 0, 5), None);
        assert_eq!(on_screen(3, 3, 5, 0), None);
    }

    #[test]
    fn stale_bitmap_reference_names_the_op() {
        let stale = BitmapId::new(3, 0);
        let ops = [clear(SCREEN), quad(stale, Rect::new(0, 0, 1, 1))];
        assert_eq!(
            encode_ops(&table(), SIZE, &ops),
            Err(VkError::InvalidOpBitmap {
                index: 1,
                source: BitmapError::InvalidReference {
                    id: stale,
                    slot: SlotState::Occupied { generation: 1 },
                },
            })
        );
    }

    #[test]
    fn unknown_bitmap_reference_names_the_op() {
        let unknown = BitmapId::new(9, 0);
        let ops = [quad(unknown, Rect::new(0, 0, 1, 1))];
        assert_eq!(
            encode_ops(&table(), SIZE, &ops),
            Err(VkError::InvalidOpBitmap {
                index: 0,
                source: BitmapError::InvalidReference {
                    id: unknown,
                    slot: SlotState::Empty,
                },
            })
        );
    }

    #[test]
    fn source_rectangle_outside_the_bitmap_names_the_op() {
        for src in [
            Rect::new(0, 0, 9, 6),
            Rect::new(1, 0, 8, 1),
            Rect::new(-1, 0, 2, 2),
            Rect::new(0, 5, 1, 2),
        ] {
            let ops = [clear(SCREEN), clear(SCREEN), quad(ID, src)];
            assert_eq!(
                encode_ops(&table(), SIZE, &ops),
                Err(VkError::SourceOutOfBounds {
                    index: 2,
                    bitmap: ID,
                    src,
                    width: 8,
                    height: 6,
                })
            );
        }
    }

    #[test]
    fn reserved_ops_are_unsupported_and_name_kind_and_index() {
        let cases = [
            (Op::Plane { clip: SCREEN }, OpKind::Plane),
            (Op::TilemapLayer { clip: SCREEN }, OpKind::TilemapLayer),
        ];
        for (op, kind) in cases {
            let ops = [clear(SCREEN), op];
            assert_eq!(
                encode_ops(&table(), SIZE, &ops),
                Err(VkError::UnsupportedOp { kind, index: 1 })
            );
        }
    }

    #[test]
    fn the_first_invalid_op_is_the_one_reported() {
        let ops = [
            quad(ID, Rect::new(0, 0, 9, 9)),
            Op::Plane { clip: SCREEN },
            quad(BitmapId::new(9, 0), SCREEN),
        ];
        assert!(matches!(
            encode_ops(&table(), SIZE, &ops),
            Err(VkError::SourceOutOfBounds { index: 0, .. })
        ));
    }

    /// An op that cannot touch the screen is still checked.
    #[test]
    fn off_screen_ops_are_validated_but_emit_no_record() {
        let off = Rect::new(100, 100, 5, 5);
        let valid = [
            clear(off),
            Op::Quad(Quad {
                clip: off,
                bitmap: ID,
                src: Rect::new(0, 0, 8, 6),
                transform: Transform::IDENTITY,
                opacity: 255,
                blend: BlendMode::Normal,
            }),
            Op::RectEffect(RectEffect {
                clip: off,
                tone: Tone::NEUTRAL,
                color: Color::default(),
            }),
        ];
        assert_eq!(encode_ops(&table(), SIZE, &valid), Ok(vec![]));

        let invalid = [Op::Quad(Quad {
            clip: off,
            bitmap: BitmapId::new(9, 0),
            src: Rect::new(0, 0, 1, 1),
            transform: Transform::IDENTITY,
            opacity: 255,
            blend: BlendMode::Normal,
        })];
        assert!(matches!(
            encode_ops(&table(), SIZE, &invalid),
            Err(VkError::InvalidOpBitmap { index: 0, .. })
        ));
        assert_eq!(
            encode_ops(&table(), SIZE, &[Op::Plane { clip: off }]),
            Err(VkError::UnsupportedOp {
                kind: OpKind::Plane,
                index: 0
            })
        );
    }

    /// An empty source rectangle is inside any bitmap wherever it lies, and draws nothing.
    #[test]
    fn quad_with_an_empty_source_emits_no_record() {
        let mut table = table();
        let empty = BitmapId::new(4, 0);
        table
            .apply(&BitmapDelta::Create {
                id: empty,
                width: 0,
                height: 5,
            })
            .unwrap();
        let ops = [
            quad(ID, Rect::new(-3, 900, 0, 7)),
            quad(empty, Rect::new(0, 0, 0, 5)),
        ];
        assert_eq!(encode_ops(&table, SIZE, &ops), Ok(vec![]));
        // A source with pixels is outside a bitmap that has none.
        assert!(matches!(
            encode_ops(&table, SIZE, &[quad(empty, Rect::new(0, 0, 1, 1))]),
            Err(VkError::SourceOutOfBounds { index: 0, .. })
        ));
    }

    #[test]
    fn clear_record_layout() {
        let records = encode_ops(&table(), SIZE, &[clear(Rect::new(-2, 3, 10, 100))]).unwrap();
        let mut expected = [0; RECORD_WORDS];
        expected[..5].copy_from_slice(&[0, 0, 3, 8, 30]);
        expected[18] = 0x0403_0201;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].words(), &expected);
        assert_eq!(
            records[0].clip,
            ClipRect {
                x0: 0,
                y0: 3,
                x1: 8,
                y1: 30
            }
        );
    }

    #[test]
    fn quad_record_layout() {
        let op = Op::Quad(Quad {
            clip: Rect::new(1, 2, 3, 4),
            bitmap: ID,
            src: Rect::new(2, 1, 5, 4),
            transform: Transform {
                xx: Fixed::from_bits(11),
                xy: Fixed::from_bits(-12),
                yx: Fixed::from_bits(13),
                yy: Fixed::from_bits(i32::MIN),
                tx: Fixed::from_bits(15),
                ty: Fixed::from_bits(-1),
            },
            opacity: 200,
            blend: BlendMode::Subtract,
        });
        let records = encode_ops(&table(), SIZE, &[op]).unwrap();
        #[rustfmt::skip]
        let expected = [
            1, 1, 2, 4, 6,
            3, 2, 1, 5, 4,
            11, -12i32 as u32, 13, 0x8000_0000, 15, u32::MAX,
            200, 2,
            0, 0, 0, 0, 0,
        ];
        assert_eq!(records[0].words(), &expected);
    }

    #[test]
    fn rect_effect_record_layout() {
        let op = Op::RectEffect(RectEffect {
            clip: SCREEN,
            tone: Tone::new(-255, i16::MAX, i16::MIN, 77),
            color: Color::new(255, 0, 128, 96),
        });
        let records = encode_ops(&table(), SIZE, &[op]).unwrap();
        let mut expected = [0; RECORD_WORDS];
        expected[..5].copy_from_slice(&[2, 0, 0, 40, 30]);
        expected[18] = 0x6080_00FF;
        expected[19..23].copy_from_slice(&[-255i32 as u32, 32767, -32768i32 as u32, 77]);
        assert_eq!(records[0].words(), &expected);
    }

    #[test]
    fn records_keep_op_order_and_blend_modes_are_distinct() {
        let blends = [BlendMode::Normal, BlendMode::Add, BlendMode::Subtract];
        let ops: Vec<Op> = blends
            .iter()
            .map(|&blend| {
                Op::Quad(Quad {
                    clip: SCREEN,
                    bitmap: ID,
                    src: Rect::new(0, 0, 8, 6),
                    transform: Transform::IDENTITY,
                    opacity: 255,
                    blend,
                })
            })
            .collect();
        let records = encode_ops(&table(), SIZE, &ops).unwrap();
        let modes: Vec<u32> = records.iter().map(|record| record.words()[17]).collect();
        assert_eq!(modes, [0, 1, 2]);
    }
}
