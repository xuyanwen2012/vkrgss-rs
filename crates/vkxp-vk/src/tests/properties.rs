//! Randomly generated packet sequences through both backends.
//!
//! A strategy generates a *plan*: what each packet should try to do, with bitmaps named by a
//! pick number rather than by id. [`build`] turns a plan into concrete messages while tracking
//! which bitmaps are live, so that every delta and op is valid, or invalid in exactly the
//! intended way, whatever the picks are. Shrinking a plan therefore always yields a plan that
//! still means something.

use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, TestRunner};
use vkxp_compose_ref as reference;
use vkxp_proto::{
    BitmapDelta, BitmapId, BlendMode, Color, Fixed, FramePacket, FrameRate, GuestToHost, Op, Quad,
    Rect, RectEffect, Sampling, Size, Tone, Transform,
};

use super::differential::compare_stream;
use super::harness::Rng;

/// Cases per property. Fixed, so the gate does the same work everywhere.
const CASES: u32 = 192;

/// Slots the plans create bitmaps in. Few, so that slots are reused within a sequence.
const SLOTS: u32 = 5;

/// A slot no plan ever creates a bitmap in.
const UNUSED_SLOT: u32 = 40;

#[derive(Debug, Clone)]
enum DeltaPlan {
    /// Creates a bitmap in `slot` if it is free, with a fresh generation.
    Create {
        slot: u32,
        width: u32,
        height: u32,
    },
    /// Replaces part of a live bitmap with pixels generated from `seed`.
    Update {
        pick: u32,
        span: [u32; 4],
        seed: u32,
    },
    Destroy {
        pick: u32,
    },
    Invalid(InvalidDelta),
}

#[derive(Debug, Clone)]
enum InvalidDelta {
    /// Creates into a slot that holds a bitmap.
    CreateOccupied {
        pick: u32,
    },
    /// More pixels than any backend accepts.
    CreateTooLarge {
        slot: u32,
    },
    /// Updates or destroys through the id of a destroyed bitmap.
    Stale {
        pick: u32,
        destroy: bool,
    },
    /// Destroys a bitmap that never existed.
    Unknown,
    UpdateOutOfBounds {
        pick: u32,
    },
    UpdateWrongLength {
        pick: u32,
    },
}

#[derive(Debug, Clone)]
enum ClipPlan {
    Screen,
    /// A rectangle near the screen: inside it, across its edges or beside it.
    Near(Rect),
    /// Any rectangle at all.
    Any(Rect),
}

#[derive(Debug, Clone)]
enum TransformPlan {
    /// Unscaled at a screen position.
    Place { x: i16, y: i16 },
    /// Scaled by a ratio per axis, either of which may be negative (a flip).
    Scale {
        x: (i32, i32),
        y: (i32, i32),
        tx: i32,
        ty: i32,
    },
    /// Six coefficients of moderate size: rotations, shears and what lies between.
    Affine { bits: [i32; 6] },
    /// Six arbitrary coefficients.
    Wild { bits: [i32; 6] },
    /// A stretch into a rectangle near the screen.
    Stretch { dest: Rect, mirror: bool },
    /// A stretch into any rectangle at all.
    WildStretch { dest: Rect, mirror: bool },
}

#[derive(Debug, Clone)]
enum OpPlan {
    Clear {
        clip: ClipPlan,
        color: [u8; 4],
    },
    Quad {
        pick: u32,
        span: [u32; 4],
        clip: ClipPlan,
        transform: TransformPlan,
        /// `None` is the neutral tone.
        tone: Option<(i16, i16, i16, u8)>,
        /// `None` is a colour of alpha 0.
        color: Option<[u8; 4]>,
        opacity: u8,
        blend: BlendMode,
    },
    RectEffect {
        clip: ClipPlan,
        tone: (i16, i16, i16, u8),
        color: [u8; 4],
    },
    Invalid(InvalidOp),
}

#[derive(Debug, Clone)]
enum InvalidOp {
    Plane,
    TilemapLayer,
    /// A quad of a destroyed bitmap, or of one that never existed when none was destroyed.
    StaleQuad {
        pick: u32,
    },
    SourceOutOfBounds {
        pick: u32,
    },
}

#[derive(Debug, Clone)]
struct PacketPlan {
    size: (u32, u32),
    /// Replaces the logical size with an invalid one.
    bad_size: bool,
    deltas: Vec<DeltaPlan>,
    ops: Vec<OpPlan>,
}

// Strategies.

fn delta_plan() -> impl Strategy<Value = DeltaPlan> {
    prop_oneof![
        4 => (0..SLOTS, 0u32..=24, 0u32..=24)
            .prop_map(|(slot, width, height)| DeltaPlan::Create { slot, width, height }),
        4 => (any::<u32>(), any::<[u32; 4]>(), any::<u32>())
            .prop_map(|(pick, span, seed)| DeltaPlan::Update { pick, span, seed }),
        2 => any::<u32>().prop_map(|pick| DeltaPlan::Destroy { pick }),
    ]
}

fn invalid_delta() -> impl Strategy<Value = InvalidDelta> {
    prop_oneof![
        any::<u32>().prop_map(|pick| InvalidDelta::CreateOccupied { pick }),
        (0..SLOTS).prop_map(|slot| InvalidDelta::CreateTooLarge { slot }),
        (any::<u32>(), any::<bool>())
            .prop_map(|(pick, destroy)| InvalidDelta::Stale { pick, destroy }),
        Just(InvalidDelta::Unknown),
        any::<u32>().prop_map(|pick| InvalidDelta::UpdateOutOfBounds { pick }),
        any::<u32>().prop_map(|pick| InvalidDelta::UpdateWrongLength { pick }),
    ]
}

fn clip_plan() -> impl Strategy<Value = ClipPlan> {
    let near = (-20i32..90, -20i32..70, 0u32..100, 0u32..80)
        .prop_map(|(x, y, width, height)| ClipPlan::Near(Rect::new(x, y, width, height)));
    let any = (any::<i32>(), any::<i32>(), any::<u32>(), any::<u32>())
        .prop_map(|(x, y, width, height)| ClipPlan::Any(Rect::new(x, y, width, height)));
    prop_oneof![3 => Just(ClipPlan::Screen), 5 => near, 1 => any]
}

fn transform_plan() -> impl Strategy<Value = TransformPlan> {
    let ratio = || (-6i32..=6, 1i32..=6);
    let moderate = -(4i32 << 16)..=(4i32 << 16);
    let offset = -(40i32 << 16)..=(80i32 << 16);
    prop_oneof![
        3 => (-30i16..80, -30i16..60).prop_map(|(x, y)| TransformPlan::Place { x, y }),
        3 => (ratio(), ratio(), offset.clone(), offset.clone())
            .prop_map(|(x, y, tx, ty)| TransformPlan::Scale { x, y, tx, ty }),
        3 => (prop::array::uniform4(moderate), offset.clone(), offset).prop_map(
            |(m, tx, ty)| TransformPlan::Affine { bits: [m[0], m[1], m[2], m[3], tx, ty] }
        ),
        1 => any::<[i32; 6]>().prop_map(|bits| TransformPlan::Wild { bits }),
        4 => (-40i32..80, -40i32..60, 0u32..160, 0u32..120, any::<bool>()).prop_map(
            |(x, y, width, height, mirror)| TransformPlan::Stretch {
                dest: Rect::new(x, y, width, height),
                mirror,
            }
        ),
        1 => (any::<(i32, i32, u32, u32)>(), any::<bool>()).prop_map(
            |((x, y, width, height), mirror)| TransformPlan::WildStretch {
                dest: Rect::new(x, y, width, height),
                mirror,
            }
        ),
    ]
}

fn blend_mode() -> impl Strategy<Value = BlendMode> {
    prop_oneof![
        Just(BlendMode::Normal),
        Just(BlendMode::Add),
        Just(BlendMode::Subtract),
        Just(BlendMode::Window)
    ]
}

/// A byte that is often one of the values where formulas have their corners.
fn channel() -> impl Strategy<Value = u8> {
    prop_oneof![
        3 => any::<u8>(),
        1 => prop::sample::select(vec![0u8, 1, 127, 128, 254, 255]),
    ]
}

fn op_plan() -> impl Strategy<Value = OpPlan> {
    let tone_offset = || prop_oneof![4 => -255i16..=255, 1 => any::<i16>()];
    prop_oneof![
        2 => (clip_plan(), any::<[u8; 4]>())
            .prop_map(|(clip, color)| OpPlan::Clear { clip, color }),
        6 => (
            any::<u32>(),
            any::<[u32; 4]>(),
            clip_plan(),
            transform_plan(),
            // Half of the quads have no tone, half no colour, a quarter neither.
            prop::option::of((tone_offset(), tone_offset(), tone_offset(), channel())),
            prop::option::of((any::<[u8; 3]>(), channel())),
            channel(),
            blend_mode()
        )
            .prop_map(|(pick, span, clip, transform, tone, color, opacity, blend)| {
                OpPlan::Quad {
                    pick,
                    span,
                    clip,
                    transform,
                    tone,
                    color: color.map(|([r, g, b], a)| [r, g, b, a]),
                    opacity,
                    blend,
                }
            }),
        2 => (
            clip_plan(),
            (tone_offset(), tone_offset(), tone_offset(), channel()),
            any::<[u8; 4]>()
        )
            .prop_map(|(clip, tone, color)| OpPlan::RectEffect { clip, tone, color }),
    ]
}

fn invalid_op() -> impl Strategy<Value = InvalidOp> {
    prop_oneof![
        Just(InvalidOp::Plane),
        Just(InvalidOp::TilemapLayer),
        any::<u32>().prop_map(|pick| InvalidOp::StaleQuad { pick }),
        any::<u32>().prop_map(|pick| InvalidOp::SourceOutOfBounds { pick }),
    ]
}

/// A logical size that is usually not a multiple of the tile size, sometimes exactly one.
fn logical_size() -> impl Strategy<Value = (u32, u32)> {
    prop_oneof![
        4 => (1u32..=70, 1u32..=50),
        1 => (1u32..=4, 1u32..=3).prop_map(|(x, y)| (x * 16, y * 16)),
    ]
}

/// A packet in which everything is valid.
fn valid_packet() -> impl Strategy<Value = PacketPlan> {
    (
        logical_size(),
        prop::collection::vec(delta_plan(), 0..=5),
        prop::collection::vec(op_plan(), 0..=10),
    )
        .prop_map(|(size, deltas, ops)| PacketPlan {
            size,
            bad_size: false,
            deltas,
            ops,
        })
}

/// A packet with invalid deltas and ops mixed in, and now and then an invalid logical size.
fn faulty_packet() -> impl Strategy<Value = PacketPlan> {
    let delta = prop_oneof![
        5 => delta_plan(),
        1 => invalid_delta().prop_map(DeltaPlan::Invalid),
    ];
    let op = prop_oneof![
        7 => op_plan(),
        1 => invalid_op().prop_map(OpPlan::Invalid),
    ];
    (
        logical_size(),
        prop::bool::weighted(0.05),
        prop::collection::vec(delta, 0..=5),
        prop::collection::vec(op, 0..=8),
    )
        .prop_map(|(size, bad_size, deltas, ops)| PacketPlan {
            size,
            bad_size,
            deltas,
            ops,
        })
}

fn valid_sequence() -> impl Strategy<Value = Vec<PacketPlan>> {
    prop::collection::vec(valid_packet(), 1..=5)
}

/// Faulty packets followed by one valid packet, which shows the state both backends kept.
fn faulty_sequence() -> impl Strategy<Value = Vec<PacketPlan>> {
    (
        prop::collection::vec(faulty_packet(), 1..=4),
        valid_packet(),
    )
        .prop_map(|(mut packets, last)| {
            packets.push(last);
            packets
        })
}

// From plans to messages.

/// The bitmaps that are live at some point of a sequence.
#[derive(Debug, Clone, Default)]
struct Model {
    /// Slot to generation and size.
    live: BTreeMap<u32, (u32, u32, u32)>,
    /// Ids of destroyed bitmaps. A generation is never reused, so these stay invalid.
    destroyed: Vec<BitmapId>,
}

impl Model {
    /// The live bitmap a pick number selects, if any bitmap is live.
    fn pick(&self, pick: u32) -> Option<(BitmapId, u32, u32)> {
        let count = self.live.len() as u32;
        let (slot, (generation, width, height)) =
            self.live.iter().nth((pick % count.max(1)) as usize)?;
        Some((BitmapId::new(*slot, *generation), *width, *height))
    }

    /// The id of a destroyed bitmap, or of one that never existed.
    fn stale(&self, pick: u32) -> BitmapId {
        match self.destroyed.len() {
            0 => BitmapId::new(UNUSED_SLOT, 0),
            count => self.destroyed[pick as usize % count],
        }
    }
}

/// A sub-range of `0..length` from two arbitrary numbers. It may be empty.
fn span(a: u32, b: u32, length: u32) -> (u32, u32) {
    let (a, b) = (a % (length + 1), b % (length + 1));
    (a.min(b), a.max(b) - a.min(b))
}

/// A rectangle inside a `width` by `height` bitmap from four arbitrary numbers.
fn sub_rect(numbers: [u32; 4], width: u32, height: u32) -> Rect {
    let (x, w) = span(numbers[0], numbers[1], width);
    let (y, h) = span(numbers[2], numbers[3], height);
    Rect::new(x as i32, y as i32, w, h)
}

/// Pixels with every kind of alpha: runs of transparent, opaque and arbitrary values.
fn pixels(seed: u32, count: u32) -> Vec<u8> {
    let mut rng = Rng::new(u64::from(seed));
    let mut rgba = Vec::with_capacity(count as usize * 4);
    for _ in 0..count {
        let alpha = match rng.below(4) {
            0 => 0,
            1 => 255,
            _ => rng.next_u32() as u8,
        };
        let color = rng.next_u32().to_le_bytes();
        rgba.extend_from_slice(&[color[0], color[1], color[2], alpha]);
    }
    rgba
}

fn clip(plan: &ClipPlan, size: Size) -> Rect {
    match plan {
        ClipPlan::Screen => Rect::from_size(size),
        ClipPlan::Near(rect) | ClipPlan::Any(rect) => *rect,
    }
}

fn sampling(plan: &TransformPlan) -> Sampling {
    let from_bits = |bits: &[i32; 6]| Transform {
        xx: Fixed::from_bits(bits[0]),
        xy: Fixed::from_bits(bits[1]),
        yx: Fixed::from_bits(bits[2]),
        yy: Fixed::from_bits(bits[3]),
        tx: Fixed::from_bits(bits[4]),
        ty: Fixed::from_bits(bits[5]),
    };
    Sampling::Affine(match plan {
        TransformPlan::Place { x, y } => Transform::place_at(*x, *y),
        TransformPlan::Scale { x, y, tx, ty } => Transform {
            xx: Fixed::from_ratio(x.0, x.1).expect("small ratio"),
            xy: Fixed::ZERO,
            yx: Fixed::ZERO,
            yy: Fixed::from_ratio(y.0, y.1).expect("small ratio"),
            tx: Fixed::from_bits(*tx),
            ty: Fixed::from_bits(*ty),
        },
        TransformPlan::Affine { bits } | TransformPlan::Wild { bits } => from_bits(bits),
        TransformPlan::Stretch { dest, mirror } | TransformPlan::WildStretch { dest, mirror } => {
            return Sampling::Stretch {
                dest: *dest,
                mirror: *mirror,
            };
        }
    })
}

/// Turns the plans into frame messages. With `faults` off, invalid plan items are left out.
fn build(plans: &[PacketPlan], faults: bool) -> Vec<GuestToHost> {
    let mut model = Model::default();
    // Generations are handed out once and never again, per slot and across failed packets.
    let mut next_generation = [0u32; SLOTS as usize];
    let mut messages = Vec::new();

    for (frame_index, plan) in plans.iter().enumerate() {
        let size = Size::new(plan.size.0, plan.size.1);
        let mut working = model.clone();
        // The model as it was at the first invalid delta: every backend stops there.
        let mut stopped_at: Option<Model> = None;
        let mut bitmaps = Vec::new();

        for delta in &plan.deltas {
            match delta {
                DeltaPlan::Create {
                    slot,
                    width,
                    height,
                } => {
                    if working.live.contains_key(slot) {
                        continue;
                    }
                    let generation = next_generation[*slot as usize];
                    next_generation[*slot as usize] += 1;
                    working.live.insert(*slot, (generation, *width, *height));
                    bitmaps.push(BitmapDelta::Create {
                        id: BitmapId::new(*slot, generation),
                        width: *width,
                        height: *height,
                    });
                }
                DeltaPlan::Update { pick, span, seed } => {
                    let Some((id, width, height)) = working.pick(*pick) else {
                        continue;
                    };
                    let rect = sub_rect(*span, width, height);
                    bitmaps.push(BitmapDelta::Update {
                        id,
                        rect,
                        rgba: pixels(*seed, rect.width * rect.height),
                    });
                }
                DeltaPlan::Destroy { pick } => {
                    let Some((id, _, _)) = working.pick(*pick) else {
                        continue;
                    };
                    working.live.remove(&id.index);
                    working.destroyed.push(id);
                    bitmaps.push(BitmapDelta::Destroy { id });
                }
                DeltaPlan::Invalid(invalid) => {
                    if !faults {
                        continue;
                    }
                    let delta = match invalid {
                        InvalidDelta::CreateOccupied { pick } => {
                            working.pick(*pick).map(|(id, _, _)| BitmapDelta::Create {
                                // The occupant's generation or a new one: both are refused.
                                id: BitmapId::new(id.index, id.generation + pick % 2),
                                width: 3,
                                height: 3,
                            })
                        }
                        InvalidDelta::CreateTooLarge { slot } => Some(BitmapDelta::Create {
                            id: BitmapId::new(*slot, u32::MAX),
                            width: 20_000,
                            height: 20_000,
                        }),
                        InvalidDelta::Stale { pick, destroy } => {
                            let id = working.stale(*pick);
                            Some(if *destroy {
                                BitmapDelta::Destroy { id }
                            } else {
                                BitmapDelta::Update {
                                    id,
                                    rect: Rect::new(0, 0, 1, 1),
                                    rgba: vec![255; 4],
                                }
                            })
                        }
                        InvalidDelta::Unknown => Some(BitmapDelta::Destroy {
                            id: BitmapId::new(UNUSED_SLOT, 7),
                        }),
                        InvalidDelta::UpdateOutOfBounds { pick } => {
                            working.pick(*pick).map(|(id, width, height)| {
                                // One pixel past the right or the bottom edge.
                                let rect = if pick % 2 == 0 {
                                    Rect::new(width as i32, 0, 1, 1)
                                } else {
                                    Rect::new(0, 0, 1, height + 1)
                                };
                                BitmapDelta::Update {
                                    id,
                                    rect,
                                    rgba: vec![9; (rect.width * rect.height * 4) as usize],
                                }
                            })
                        }
                        InvalidDelta::UpdateWrongLength { pick } => {
                            working
                                .pick(*pick)
                                .map(|(id, width, height)| BitmapDelta::Update {
                                    id,
                                    rect: Rect::new(0, 0, width, height),
                                    rgba: vec![
                                        9;
                                        (width * height * 4) as usize
                                            + 1
                                            + (pick % 3) as usize
                                    ],
                                })
                        }
                    };
                    // Without a live bitmap to misuse, a stale reference stands in.
                    bitmaps.push(delta.unwrap_or(BitmapDelta::Destroy {
                        id: working.stale(0),
                    }));
                    stopped_at.get_or_insert_with(|| working.clone());
                }
            }
        }
        // Later deltas of a failed packet were built against `working` but are never applied.
        model = stopped_at.unwrap_or(working);

        let mut ops = Vec::new();
        for op in &plan.ops {
            match op {
                OpPlan::Clear { clip: plan, color } => ops.push(Op::Clear {
                    clip: clip(plan, size),
                    color: Color::from_rgba(*color),
                }),
                OpPlan::Quad {
                    pick,
                    span,
                    clip: clip_plan,
                    transform: transform_plan,
                    tone,
                    color,
                    opacity,
                    blend,
                } => {
                    let Some((id, width, height)) = model.pick(*pick) else {
                        continue;
                    };
                    ops.push(Op::Quad(Quad {
                        clip: clip(clip_plan, size),
                        bitmap: id,
                        src: sub_rect(*span, width, height),
                        sampling: sampling(transform_plan),
                        tone: tone.map_or(Tone::NEUTRAL, |(red, green, blue, gray)| {
                            Tone::new(red, green, blue, gray)
                        }),
                        color: color.map_or(Color::new(0, 0, 0, 0), Color::from_rgba),
                        opacity: *opacity,
                        blend: *blend,
                    }));
                }
                OpPlan::RectEffect {
                    clip: plan,
                    tone,
                    color,
                } => ops.push(Op::RectEffect(RectEffect {
                    clip: clip(plan, size),
                    tone: Tone::new(tone.0, tone.1, tone.2, tone.3),
                    color: Color::from_rgba(*color),
                })),
                OpPlan::Invalid(invalid) => {
                    if !faults {
                        continue;
                    }
                    let screen = Rect::from_size(size);
                    let quad = |bitmap, src| {
                        Op::Quad(Quad {
                            clip: screen,
                            bitmap,
                            src,
                            sampling: Sampling::Affine(Transform::IDENTITY),
                            tone: Tone::NEUTRAL,
                            color: Color::new(0, 0, 0, 0),
                            opacity: 255,
                            blend: BlendMode::Normal,
                        })
                    };
                    ops.push(match invalid {
                        InvalidOp::Plane => Op::Plane { clip: screen },
                        InvalidOp::TilemapLayer => Op::TilemapLayer { clip: screen },
                        InvalidOp::StaleQuad { pick } => {
                            quad(model.stale(*pick), Rect::new(0, 0, 1, 1))
                        }
                        InvalidOp::SourceOutOfBounds { pick } => match model.pick(*pick) {
                            Some((id, width, height)) => {
                                quad(id, Rect::new(0, 0, width + 1, height.max(1)))
                            }
                            None => quad(model.stale(*pick), Rect::new(0, 0, 1, 1)),
                        },
                    });
                }
            }
        }

        let logical_size = match (faults && plan.bad_size, frame_index % 2) {
            (false, _) => size,
            (true, 0) => Size::new(0, size.height),
            (true, _) => Size::new(size.width, 16_385),
        };
        messages.push(GuestToHost::Frame(FramePacket {
            frame_index: frame_index as u64,
            logical_size,
            frame_rate: FrameRate::DEFAULT,
            bitmaps,
            ops,
        }));
    }
    messages
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    /// Valid packet sequences: every frame is byte-identical.
    #[test]
    fn valid_sequences_render_identically(plans in valid_sequence()) {
        let messages = build(&plans, false);
        if let Err(difference) = compare_stream(&messages) {
            return Err(TestCaseError::fail(difference));
        }
    }

    /// Sequences with invalid deltas and ops: both backends reject the same packets with the
    /// same error, keep the same bitmaps, and render the remaining packets identically.
    #[test]
    fn faulty_sequences_fail_and_recover_identically(plans in faulty_sequence()) {
        let messages = build(&plans, true);
        if let Err(difference) = compare_stream(&messages) {
            return Err(TestCaseError::fail(difference));
        }
    }
}

/// What a set of generated sequences exercised, according to the reference compositor.
#[derive(Debug, Default)]
struct Coverage {
    frames: u32,
    clears: u32,
    effects: u32,
    quads_by_blend: [u32; 4],
    /// Quads by whether they have a tone and a colour: neither, tone, colour, both.
    quads_by_recolouring: [u32; 4],
    transformed_quads: u32,
    stretch_quads: u32,
    clipped_ops: u32,
    quads_of_earlier_bitmaps: u32,
    reused_slots: u32,
    odd_sizes: u32,
    failed_deltas: u32,
    failed_ops: u32,
    failed_sizes: u32,
    frames_after_a_failure: u32,
}

/// Generates sequences the way the properties do and tallies what they contain.
fn coverage(strategy: impl Strategy<Value = Vec<PacketPlan>>, faults: bool) -> Coverage {
    let mut runner = TestRunner::new_with_rng(
        Config::default(),
        proptest::test_runner::TestRng::deterministic_rng(
            proptest::test_runner::RngAlgorithm::ChaCha,
        ),
    );
    let mut coverage = Coverage::default();
    for _ in 0..CASES {
        let plans = strategy.new_tree(&mut runner).unwrap().current();
        let mut compositor = reference::Compositor::new();
        let mut failed = false;
        let mut seen_slots = std::collections::BTreeSet::new();
        for message in build(&plans, faults) {
            let GuestToHost::Frame(packet) = &message else {
                unreachable!("plans only build frames");
            };
            let screen = Rect::from_size(packet.logical_size);
            let created: Vec<u32> = packet
                .bitmaps
                .iter()
                .filter_map(|delta| match delta {
                    BitmapDelta::Create { id, .. } => Some(id.index),
                    _ => None,
                })
                .collect();
            match compositor.handle(&message) {
                Ok(_) => {
                    coverage.frames += 1;
                    coverage.frames_after_a_failure += u32::from(failed);
                    let size = packet.logical_size;
                    coverage.odd_sizes += u32::from(size.width % 16 != 0 || size.height % 16 != 0);
                    for slot in &created {
                        coverage.reused_slots += u32::from(!seen_slots.insert(*slot));
                    }
                    for op in &packet.ops {
                        coverage.clipped_ops += u32::from(op.clip() != screen);
                        match op {
                            Op::Clear { .. } => coverage.clears += 1,
                            Op::RectEffect(_) => coverage.effects += 1,
                            Op::Quad(quad) => {
                                coverage.quads_by_blend[quad.blend as usize] += 1;
                                let toned = usize::from(quad.tone != Tone::NEUTRAL);
                                let coloured = usize::from(quad.color.a != 0);
                                coverage.quads_by_recolouring[toned + 2 * coloured] += 1;
                                coverage.transformed_quads += u32::from(
                                    quad.sampling != Sampling::Affine(Transform::IDENTITY),
                                );
                                coverage.stretch_quads +=
                                    u32::from(matches!(quad.sampling, Sampling::Stretch { .. }));
                                coverage.quads_of_earlier_bitmaps +=
                                    u32::from(!created.contains(&quad.bitmap.index));
                            }
                            _ => {}
                        }
                    }
                }
                Err(error) => {
                    failed = true;
                    match error {
                        reference::ComposeError::InvalidDelta { .. } => coverage.failed_deltas += 1,
                        reference::ComposeError::InvalidLogicalSize { .. } => {
                            coverage.failed_sizes += 1
                        }
                        _ => coverage.failed_ops += 1,
                    }
                }
            }
        }
    }
    coverage
}

/// The properties would pass on empty packets too. This checks that the generated sequences
/// contain what the properties claim to cover, in numbers that make the comparison meaningful.
#[test]
fn generated_sequences_cover_what_the_properties_claim() {
    let valid = coverage(valid_sequence(), false);
    eprintln!("valid sequences: {valid:?}");
    assert_eq!(
        (valid.failed_deltas, valid.failed_ops, valid.failed_sizes),
        (0, 0, 0),
        "a valid sequence must not fail"
    );
    assert!(valid.frames >= CASES);
    for count in [
        valid.clears,
        valid.effects,
        valid.quads_by_blend[0],
        valid.quads_by_blend[1],
        valid.quads_by_blend[2],
        valid.quads_by_blend[3],
        valid.quads_by_recolouring[0],
        valid.quads_by_recolouring[1],
        valid.quads_by_recolouring[2],
        valid.quads_by_recolouring[3],
        valid.transformed_quads,
        valid.stretch_quads,
        valid.clipped_ops,
        valid.quads_of_earlier_bitmaps,
        valid.odd_sizes,
    ] {
        assert!(count >= 100, "{valid:?}");
    }
    // Rarer by nature: a slot has to be created, destroyed and created again.
    assert!(valid.reused_slots >= 20, "{valid:?}");

    let faulty = coverage(faulty_sequence(), true);
    eprintln!("faulty sequences: {faulty:?}");
    assert!(faulty.failed_deltas >= 40, "{faulty:?}");
    assert!(faulty.failed_ops >= 40, "{faulty:?}");
    assert!(faulty.failed_sizes >= 5, "{faulty:?}");
    // Nearly every sequence has a failure followed by a frame that both backends render.
    assert!(faulty.frames_after_a_failure >= CASES / 2, "{faulty:?}");
}
