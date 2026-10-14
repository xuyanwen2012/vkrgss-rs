//! A guest written in Rust: the stand-in for a game until the Ruby side exists.
//!
//! It speaks the protocol as a real guest would. Each frame it sends one packet and then waits
//! for the tick, taking in whatever else the host sends meanwhile:
//!
//! - the directions move a sprite, faster while SHIFT is held;
//! - C held puts a rectangle effect (a tone and a colour) over the middle of the screen;
//! - B pressed freezes the screen, swaps the background and fades over with a transition;
//! - a reset request puts everything back and restarts the pacing with a frame reset;
//! - a quit request sends the shutdown message and returns.
//!
//! Everything it draws is generated here. What it does depends only on the messages it
//! receives and the number of ticks it has seen, so two runs with the same input are alike.

use vkxp_proto::link::{Disconnected, GuestEnd};
use vkxp_proto::{
    BitmapDelta, BitmapId, BlendMode, Button, Color, FramePacket, FrameRate, FreezeRule,
    GuestToHost, HostToGuest, InputSnapshot, Op, Quad, Rect, RectEffect, Sampling, Size, Tone,
    Transform, TransitionRule,
};

/// The window title the demo sets.
pub const TITLE: &str = "vkxp demo";

/// Length in frames of the transition after B.
pub const TRANSITION_FRAMES: u32 = 20;

/// Edge of the square sprite in pixels.
pub const SPRITE_SIZE: u32 = 32;

const BACKGROUND: BitmapId = BitmapId::new(0, 0);
const OTHER_BACKGROUND: BitmapId = BitmapId::new(1, 0);
const SPRITE: BitmapId = BitmapId::new(2, 0);
const MARKER: BitmapId = BitmapId::new(3, 0);

/// Edge of the marker that runs along the top of the screen, one step per frame.
const MARKER_SIZE: u32 = 8;

/// How the demo is set up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemoOptions {
    /// Size of the logical screen, at least 64 by 64 pixels.
    pub size: Size,
    pub frame_rate: FrameRate,
}

impl Default for DemoOptions {
    fn default() -> DemoOptions {
        DemoOptions {
            size: Size::new(640, 480),
            frame_rate: FrameRate::DEFAULT,
        }
    }
}

/// What the demo remembers between frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct State {
    /// Top-left corner of the sprite.
    x: i32,
    y: i32,
    /// Whether the other background is up.
    swapped: bool,
}

impl State {
    fn initial(size: Size) -> State {
        State {
            x: (size.width as i32 - SPRITE_SIZE as i32) / 2,
            y: (size.height as i32 - SPRITE_SIZE as i32) / 2,
            swapped: false,
        }
    }
}

/// Runs the demo until the host asks it to quit or goes away.
///
/// # Panics
///
/// If the screen of `options` is smaller than 64 by 64 pixels.
pub fn run(link: &GuestEnd, options: DemoOptions) -> Result<(), Disconnected> {
    let size = options.size;
    assert!(
        size.width >= 64 && size.height >= 64,
        "the demo needs a screen of at least 64 by 64 pixels"
    );
    link.send(GuestToHost::SetTitle {
        title: TITLE.to_owned(),
    })?;

    let mut state = State::initial(size);
    let mut input = InputSnapshot::EMPTY;
    let mut previous = InputSnapshot::EMPTY;
    let mut frame_index = 0;
    loop {
        let bitmaps = match frame_index {
            0 => bitmaps(size),
            _ => Vec::new(),
        };
        link.send(GuestToHost::Frame(FramePacket {
            frame_index,
            logical_size: size,
            frame_rate: options.frame_rate,
            brightness: 255,
            tables: vec![],
            bitmaps,
            ops: ops(size, &state, frame_index, input),
        }))?;
        frame_index += 1;

        let mut reset = false;
        loop {
            match link.recv()? {
                HostToGuest::Tick => break,
                HostToGuest::Input(snapshot) => input = snapshot,
                HostToGuest::Reset => reset = true,
                HostToGuest::ModalDismissed | HostToGuest::Resume => {}
                HostToGuest::Quit => return link.send(GuestToHost::Shutdown),
                // A guest that cannot be copied.
                HostToGuest::SaveState { slot } => {
                    link.send(GuestToHost::StateFailed { slot, load: false })?;
                }
                HostToGuest::LoadState { slot } => {
                    link.send(GuestToHost::StateFailed { slot, load: true })?;
                }
            }
        }

        if reset {
            state = State::initial(size);
            previous = input;
            link.send(GuestToHost::FrameReset)?;
            continue;
        }
        let held = |button| i32::from(input.is_down(button));
        let speed = if input.is_down(Button::Shift) { 8 } else { 4 };
        let limit = |position: i32, screen: u32| position.clamp(0, (screen - SPRITE_SIZE) as i32);
        state.x = limit(
            state.x + speed * (held(Button::Right) - held(Button::Left)),
            size.width,
        );
        state.y = limit(
            state.y + speed * (held(Button::Down) - held(Button::Up)),
            size.height,
        );
        if input.is_down(Button::B) && !previous.is_down(Button::B) {
            link.send(GuestToHost::Freeze {
                rule: FreezeRule::Frame,
            })?;
            state.swapped = !state.swapped;
            link.send(GuestToHost::Transition {
                duration: TRANSITION_FRAMES,
                mask: None,
                vague: 0,
                rule: TransitionRule::Rgss1,
            })?;
        }
        previous = input;
    }
}

/// The ops of one frame: the background, the marker, the sprite, and the effect while C is
/// held.
fn ops(size: Size, state: &State, frame_index: u64, input: InputSnapshot) -> Vec<Op> {
    let screen = Rect::from_size(size);
    let quad = |bitmap, width, height, x: i32, y: i32, blend| {
        Op::Quad(Quad {
            clip: screen,
            bitmap,
            src: Rect::new(0, 0, width, height),
            sampling: Sampling::Affine(Transform::place_at(x as i16, y as i16)),
            tone: Tone::NEUTRAL,
            color: Color::new(0, 0, 0, 0),
            opacity: 255,
            blend,
        })
    };
    let background = if state.swapped {
        OTHER_BACKGROUND
    } else {
        BACKGROUND
    };
    let marker_x = (frame_index * 2 % u64::from(size.width - MARKER_SIZE)) as i32;
    let mut ops = vec![
        quad(background, size.width, size.height, 0, 0, BlendMode::Normal),
        quad(
            MARKER,
            MARKER_SIZE,
            MARKER_SIZE,
            marker_x,
            4,
            BlendMode::Add,
        ),
        quad(
            SPRITE,
            SPRITE_SIZE,
            SPRITE_SIZE,
            state.x,
            state.y,
            BlendMode::Normal,
        ),
    ];
    if input.is_down(Button::C) {
        ops.push(Op::RectEffect(RectEffect {
            clip: Rect::new(
                (size.width / 4) as i32,
                (size.height / 4) as i32,
                size.width / 2,
                size.height / 2,
            ),
            tone: Tone::new(48, -32, -32, 160),
            color: Color::new(255, 224, 96, 64),
        }));
    }
    ops
}

/// The deltas that create every bitmap the demo draws.
fn bitmaps(size: Size) -> Vec<BitmapDelta> {
    let (width, height) = (size.width, size.height);
    let checkerboard = pixels(width, height, |x, y| {
        if (x / 16 + y / 16) % 2 == 0 {
            [40, 44, 64, 255]
        } else {
            [72, 80, 104, 255]
        }
    });
    let stripes = pixels(width, height, |x, y| {
        let band = ((x + y) / 12 % 3) as u8;
        [96 + band * 40, 64, 48 + band * 24, 255]
    });
    let sprite = pixels(SPRITE_SIZE, SPRITE_SIZE, |x, y| {
        // A disc with a lighter core; the corners stay transparent.
        let centre = SPRITE_SIZE as i32 - 1;
        let (dx, dy) = (2 * x as i32 - centre, 2 * y as i32 - centre);
        let distance = dx * dx + dy * dy;
        if distance > centre * centre {
            [0, 0, 0, 0]
        } else if distance < centre * centre / 4 {
            [255, 240, 200, 255]
        } else {
            [232, 160, 48, 255]
        }
    });
    let marker = pixels(MARKER_SIZE, MARKER_SIZE, |_, _| [64, 96, 64, 255]);

    let mut deltas = Vec::new();
    for (id, width, height, rgba) in [
        (BACKGROUND, width, height, checkerboard),
        (OTHER_BACKGROUND, width, height, stripes),
        (SPRITE, SPRITE_SIZE, SPRITE_SIZE, sprite),
        (MARKER, MARKER_SIZE, MARKER_SIZE, marker),
    ] {
        deltas.push(BitmapDelta::Create { id, width, height });
        deltas.push(BitmapDelta::Update {
            id,
            rect: Rect::new(0, 0, width, height),
            rgba,
        });
    }
    deltas
}

/// The RGBA bytes of an image whose pixel at (`x`, `y`) is `pixel(x, y)`.
fn pixels(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&pixel(x, y));
        }
    }
    rgba
}
