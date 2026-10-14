//! Scenes of windows of RGSS2 and RGSS3 written as text, for the tests that compare frames
//! with the pictures of the measurements on the original runtimes: of 2026-10-12 on VX and of
//! 2026-10-13 on VX Ace.
//!
//! A scene is the calls of a script on windows, sprites and viewports, on a screen of 544 by
//! 416 pixels, and after some of them a frame with the digest of the picture the model of
//! the measurement gives there. Every bitmap is made by a formula; no picture is read.
//! One statement stands on a line; `#` begins a comment line.
//!
//! ```text
//! scene <name>
//! runtime rgss3
//! bitmap b<n> noise <o|t|a|u> <seed> <width> <height>
//! bitmap b<n> skin <o|t|a|u> <seed> <mask> <width> <height>
//! bitmap b<n> flat <red> <green> <blue> <alpha> <width> <height>
//! bitmap b<n> exc|grid <width> <height>
//! bitmap b<n> frame2|cursor2|ramp <width> <height>
//! bitmap b<n> back4 <red> <green> <blue> <width> <height>
//! new <id> window|sprite|viewport <viewport id or -> [<x> <y> <width> <height>]
//! set <id> <property> <integer | true | false | nil | b<n> | o<id> | rect x y width height>
//! set <id> tone <red> <green> <blue> <grey>
//! update <id> <times>
//! rect <id> <x> <y> <width> <height>
//! rectx <id> <x>
//! move <id> <x> <y> <width> <height>
//! fill b<n> <x> <y> <width> <height> <red> <green> <blue> <alpha>
//! dispose <id>
//! bdispose b<n>
//! brightness <value>
//! frame <digest> <name>
//! end
//! ```
//!
//! `noise` is a pixel function of the measurement: `o` opaque noise, `a` noise with any
//! alpha, `t` noise with alphas of a short list, `u` a readable gradient. `skin` is the same
//! of which only the 64 by 64 quadrants named by the bits of the mask are kept (1 background,
//! 2 pattern, 4 frame, 8 cursor and pause sign). `exc` and `grid` are skins of noise whose
//! frame and cursor have the pairs of alphas for which `Bitmap#blt` is one weight off, and
//! the alphas of a coarse grid. `frame2` and `cursor2` are readable gradients in the frame
//! quadrant and in the lower right quadrant alone, `ramp` a readable gradient of any size,
//! and `back4` a skin whose background block alone is opaque, red rising with x over the
//! other components given. `rect` changes the cursor rectangle of a window as the methods of
//! its `Rect` do, `rectx` its x alone, and `bdispose` disposes a bitmap. A window is created
//! as the binding creates one: with contents of 1 by 1.
//!
//! `runtime rgss3`, the first statement of a scene of RGSS3, makes every window of the scene
//! one of RGSS3 and lets the scene follow the rules of RGSS2, which a game of RGSS3 gets.
//! Such a window may be created with its position and size, and has the properties
//! `padding`, `padding_bottom`, `arrows_visible` and `tone` and the statement `move`.

use std::collections::HashMap;

use vkrgss_proto::{BitmapId, FramePacket, Size};

use crate::{
    Bitmaps, Color, FontDefaults, Rect, Rules, Scene, SpriteId, Tone, ViewportId, WindowCall,
    WindowId,
};

/// The screen of every scene.
pub const SIZE: Size = Size::new(544, 416);

/// The digest of a frame: FNV-1a, 64 bits, over its width and height as little-endian
/// 32-bit numbers and its pixels.
pub fn digest(size: Size, rgba: &[u8]) -> u64 {
    let size = [size.width, size.height].map(u32::to_le_bytes);
    let bytes = size.as_flattened().iter().chain(rgba);
    bytes.fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// One scene of a text of scenes.
#[derive(Debug, Clone)]
pub struct Case<'a> {
    pub name: &'a str,
    statements: Vec<&'a str>,
}

/// The scenes of `text`.
///
/// # Panics
///
/// When a statement stands outside a scene.
pub fn cases(text: &str) -> Vec<Case<'_>> {
    let mut cases: Vec<Case<'_>> = Vec::new();
    let mut open = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix("scene ") {
            cases.push(Case {
                name,
                statements: Vec::new(),
            });
            open = true;
        } else if line == "end" {
            open = false;
        } else {
            assert!(open, "`{line}` stands outside a scene");
            cases.last_mut().unwrap().statements.push(line);
        }
    }
    cases
}

/// The hash of the measurement's pixel functions.
fn mix(x: u32, y: u32, seed: u32) -> u32 {
    let v = x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663) ^ seed.wrapping_mul(83_492_791);
    let v = (v ^ (v >> 13)).wrapping_mul(1_274_126_177);
    v ^ (v >> 16)
}

/// The alphas of the kind `t`.
const ALPHAS: [u8; 12] = [0, 0, 1, 2, 64, 127, 128, 129, 200, 254, 255, 255];

/// Pixel (`x`, `y`) of the pixel function `kind` with `seed`.
fn pattern(kind: &str, seed: u32, x: u32, y: u32) -> [u8; 4] {
    if kind == "u" {
        return [(2 * x) as u8, (2 * y) as u8, 128, 255];
    }
    let v = mix(x, y, seed);
    let other = mix(x, y, seed.wrapping_add(977));
    let alpha = match kind {
        "o" => 255,
        "a" => other as u8,
        "t" => ALPHAS[other as usize % ALPHAS.len()],
        _ => panic!("unknown pixel function `{kind}`"),
    };
    [v as u8, (v >> 8) as u8, (v >> 16) as u8, alpha]
}

/// What an id of a scene names.
#[derive(Debug, Clone, Copy)]
enum Element {
    Window(WindowId),
    Sprite(SpriteId),
    Viewport(ViewportId),
}

/// A scene being run.
struct Run {
    scene: Scene,
    bitmaps: Bitmaps,
    defaults: FontDefaults,
    named: HashMap<String, BitmapId>,
    elements: HashMap<String, Element>,
    /// Whether the windows of the scene are of RGSS3.
    rgss3: bool,
}

fn number<T: std::str::FromStr>(word: &str) -> T {
    word.parse()
        .unwrap_or_else(|_| panic!("`{word}` is not a number"))
}

fn numbers<const N: usize>(words: &[&str]) -> [i32; N] {
    std::array::from_fn(|index| number(words[index]))
}

impl Run {
    fn bitmap(&self, word: &str) -> Option<BitmapId> {
        (word != "nil").then(|| self.named[word])
    }

    fn viewport(&self, word: &str) -> Option<ViewportId> {
        let word = word.strip_prefix('o').unwrap_or(word);
        match self.elements.get(word) {
            Some(Element::Viewport(id)) => Some(*id),
            _ => None,
        }
    }

    fn create_bitmap(&mut self, words: &[&str]) {
        let (width, height): (u32, u32) = (
            number(words[words.len() - 2]),
            number(words[words.len() - 1]),
        );
        let pixel: Box<dyn Fn(u32, u32) -> [u8; 4]> = match words[1] {
            "noise" => {
                let (kind, seed) = (words[2].to_owned(), number(words[3]));
                Box::new(move |x, y| pattern(&kind, seed, x, y))
            }
            "skin" => {
                let (kind, seed) = (words[2].to_owned(), number(words[3]));
                let mask: u32 = number(words[4]);
                Box::new(move |x, y| {
                    let bit = match (x < 64, y < 64) {
                        (true, true) => 1,
                        (true, false) => 2,
                        (false, true) => 4,
                        (false, false) => 8,
                    };
                    if mask & bit == 0 {
                        [0; 4]
                    } else {
                        pattern(&kind, seed, x, y)
                    }
                })
            }
            "flat" => {
                let color: [u8; 4] = std::array::from_fn(|index| number(words[2 + index]));
                Box::new(move |_, _| color)
            }
            "exc" => Box::new(|x, y| {
                // Destination alphas left, source alphas right, in the frame and the cursor.
                const UNDER: [u8; 5] = [51, 136, 204, 153, 85];
                const OVER: [u8; 5] = [30, 30, 105, 180, 210];
                let v = mix(x, y, 77);
                let pair = ((x / 2 + y) % 5) as usize;
                let alpha = match (x, y) {
                    (64..96, 0..64) => UNDER[pair],
                    (96.., 0..64) => OVER[pair],
                    (64..80, 64..96) => UNDER[pair],
                    (80..96, 64..96) => OVER[pair],
                    _ => 255,
                };
                [v as u8, (v >> 8) as u8, (v >> 16) as u8, alpha]
            }),
            "grid" => Box::new(|x, y| {
                let v = mix(x, y, 78);
                let alpha = match x {
                    0..64 => 255,
                    64..96 => x * 17 % 256,
                    _ => (y * 37 + x) % 256,
                };
                [v as u8, (v >> 8) as u8, (v >> 16) as u8, alpha as u8]
            }),
            // The readable gradients of the measurement of RGSS3, each from its case file.
            "frame2" => Box::new(|x, y| match (x, y) {
                (64.., 0..64) => [(255 - 2 * (x - 64)) as u8, (255 - 2 * y) as u8, 60, 255],
                _ => [0; 4],
            }),
            "cursor2" => Box::new(|x, y| match (x, y) {
                (64.., 64..) => [
                    (255 - 2 * (x - 64)) as u8,
                    (255 - 2 * (y - 64)) as u8,
                    60,
                    255,
                ],
                _ => [0; 4],
            }),
            "ramp" => Box::new(|x, y| [(4 * x) as u8, (4 * y) as u8, 255, 255]),
            "back4" => {
                let [red, green, blue]: [u8; 3] = std::array::from_fn(|i| number(words[2 + i]));
                Box::new(move |x, y| match (x, y) {
                    (0..64, 0..64) => [(4 * x) as u8, green, blue, 255],
                    _ => [red, green, blue, 0],
                })
            }
            other => panic!("unknown kind of bitmap `{other}`"),
        };
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&pixel(x, y));
            }
        }
        let id = self
            .bitmaps
            .create_from_rgba(width, height, rgba, &self.defaults)
            .expect("the bitmap of a scene can be created");
        self.named.insert(words[0].to_owned(), id);
    }

    fn create(&mut self, words: &[&str]) {
        let viewport = self.viewport(words[2]);
        let element = match words[1] {
            "window" => {
                let id = if self.rgss3 {
                    self.scene.create_window_rgss3(viewport).unwrap()
                } else {
                    self.scene.create_window_rgss2(viewport).unwrap()
                };
                if words.len() > 3 {
                    let [x, y, width, height] = numbers(&words[3..]);
                    self.call(id, WindowCall::Move(x, y, width, height));
                }
                let contents = self.bitmaps.create(1, 1, &self.defaults).unwrap();
                self.call(id, WindowCall::Contents(Some(contents)));
                Element::Window(id)
            }
            "sprite" => Element::Sprite(self.scene.create_sprite(viewport).unwrap()),
            "viewport" => {
                let [x, y, width, height] = numbers(&words[3..]);
                let rect = Rect::new(x, y, width, height);
                Element::Viewport(self.scene.create_viewport(rect).unwrap())
            }
            other => panic!("unknown kind of element `{other}`"),
        };
        self.elements.insert(words[0].to_owned(), element);
    }

    fn call(&mut self, id: WindowId, call: WindowCall) {
        let window = self.scene.window_mut(id).unwrap();
        window.call(&mut self.bitmaps, call);
    }

    fn set(&mut self, words: &[&str]) {
        let (property, value) = (words[1], words[2]);
        let flag = value == "true";
        match self.elements[words[0]] {
            Element::Window(id) => {
                let call = match property {
                    "x" => WindowCall::X(number(value)),
                    "y" => WindowCall::Y(number(value)),
                    "width" => WindowCall::Width(number(value)),
                    "height" => WindowCall::Height(number(value)),
                    "ox" => WindowCall::Ox(number(value)),
                    "oy" => WindowCall::Oy(number(value)),
                    "contents_opacity" => WindowCall::ContentsOpacity(number(value)),
                    "openness" => WindowCall::Openness(number(value)),
                    "active" => WindowCall::Active(flag),
                    "pause" => WindowCall::Pause(flag),
                    "windowskin" => WindowCall::Windowskin(self.bitmap(value)),
                    "contents" => WindowCall::Contents(self.bitmap(value)),
                    "cursor_rect" => {
                        let [x, y, width, height] = numbers(&words[3..]);
                        WindowCall::CursorRect(Rect::new(x, y, width, height))
                    }
                    "padding" => WindowCall::Padding(number(value)),
                    "padding_bottom" => WindowCall::PaddingBottom(number(value)),
                    "arrows_visible" => WindowCall::ArrowsVisible(flag),
                    "tone" => {
                        let [red, green, blue, grey]: [f64; 4] =
                            std::array::from_fn(|index| number(words[2 + index]));
                        let window = self.scene.window_mut(id).unwrap();
                        return window.set_tone(Tone::new(red, green, blue, grey));
                    }
                    _ => {
                        let viewport = self.viewport(value);
                        let scene = &mut self.scene;
                        match property {
                            "z" => scene.set_window_z(id, number(value)).unwrap(),
                            "viewport" => scene.set_window_viewport(id, viewport).unwrap(),
                            "visible" => scene.window_mut(id).unwrap().visible = flag,
                            "opacity" => scene.window_mut(id).unwrap().set_opacity(number(value)),
                            "back_opacity" => {
                                let window = scene.window_mut(id).unwrap();
                                window.set_back_opacity(number(value));
                            }
                            other => panic!("unknown property `{other}` of a window"),
                        }
                        return;
                    }
                };
                self.call(id, call);
            }
            Element::Sprite(id) => {
                let bitmap = (property == "bitmap").then(|| self.bitmap(value));
                let viewport = self.viewport(value);
                let scene = &mut self.scene;
                if property == "z" {
                    return scene.set_sprite_z(id, number(value)).unwrap();
                }
                if property == "viewport" {
                    return scene.set_sprite_viewport(id, viewport).unwrap();
                }
                let sprite = scene.sprite_mut(id).unwrap();
                match property {
                    "bitmap" => sprite.set_bitmap(bitmap.flatten(), &self.bitmaps),
                    "x" => sprite.x = number(value),
                    "y" => sprite.y = number(value),
                    "ox" => sprite.ox = number(value),
                    "oy" => sprite.oy = number(value),
                    "visible" => sprite.visible = flag,
                    "opacity" => sprite.set_opacity(number(value)),
                    other => panic!("unknown property `{other}` of a sprite"),
                }
            }
            Element::Viewport(id) => {
                if property == "z" {
                    return self.scene.set_viewport_z(id, number(value)).unwrap();
                }
                let viewport = self.scene.viewport_mut(id).unwrap();
                match property {
                    "ox" => viewport.ox = number(value),
                    "oy" => viewport.oy = number(value),
                    "visible" => viewport.visible = flag,
                    other => panic!("unknown property `{other}` of a viewport"),
                }
            }
        }
    }

    fn statement(&mut self, line: &str, frames: &mut Vec<(FramePacket, u64)>) {
        let words: Vec<&str> = line.split(' ').collect();
        let rest = &words[1..];
        match words[0] {
            "bitmap" => self.create_bitmap(rest),
            "new" => self.create(rest),
            "set" => self.set(rest),
            "update" => {
                if let Element::Window(id) = self.elements[rest[0]] {
                    for _ in 0..number::<u32>(rest[1]) {
                        self.call(id, WindowCall::Update);
                    }
                }
            }
            "rect" => {
                let Element::Window(id) = self.elements[rest[0]] else {
                    panic!("`{line}` names no window");
                };
                let [x, y, width, height] = numbers(&rest[1..]);
                self.call(id, WindowCall::CursorRect(Rect::new(x, y, width, height)));
            }
            "rectx" => {
                let Element::Window(id) = self.elements[rest[0]] else {
                    panic!("`{line}` names no window");
                };
                let mut rect = self.scene.window(id).unwrap().cursor_rect;
                rect.x = number(rest[1]);
                self.call(id, WindowCall::CursorRect(rect));
            }
            "move" => {
                let Element::Window(id) = self.elements[rest[0]] else {
                    panic!("`{line}` names no window");
                };
                let [x, y, width, height] = numbers(&rest[1..]);
                self.call(id, WindowCall::Move(x, y, width, height));
            }
            "bdispose" => self.bitmaps.dispose(self.named[rest[0]]),
            "runtime" => {
                assert_eq!(rest, ["rgss3"], "`{line}` names no known runtime");
                self.rgss3 = true;
                self.scene = Scene::with_rules(Rules::Rgss2, SIZE);
            }
            "fill" => {
                let [x, y, width, height, red, green, blue, alpha] = numbers(&rest[1..]);
                let color = Color::new(red.into(), green.into(), blue.into(), alpha.into());
                self.bitmaps
                    .fill_rect(self.named[rest[0]], Rect::new(x, y, width, height), color)
                    .unwrap();
            }
            "dispose" => match self.elements[rest[0]] {
                Element::Window(id) => self.scene.dispose_window(id, &mut self.bitmaps),
                Element::Sprite(id) => self.scene.dispose_sprite(id),
                Element::Viewport(id) => {
                    self.scene.dispose_viewport(id, &mut self.bitmaps);
                }
            },
            "brightness" => self.scene.set_brightness(number(rest[0])),
            "frame" => {
                let digest = u64::from_str_radix(rest[0], 16).expect("a digest in hexadecimal");
                frames.push((self.scene.extract(&mut self.bitmaps), digest));
            }
            other => panic!("unknown statement `{other}`"),
        }
    }
}

impl<'a> Case<'a> {
    /// The statements of the scene, in order.
    pub fn statements(&self) -> &[&'a str] {
        &self.statements
    }

    /// Runs the scene: the packet of every frame, in the order a compositor must get them,
    /// each with the digest of the picture it must draw.
    ///
    /// # Panics
    ///
    /// When a statement cannot be read.
    pub fn frames(&self) -> Vec<(FramePacket, u64)> {
        let mut run = Run {
            scene: Scene::with_logical_size(SIZE),
            bitmaps: Bitmaps::new(),
            defaults: FontDefaults::new(Vec::new()),
            named: HashMap::new(),
            elements: HashMap::new(),
            rgss3: false,
        };
        let mut frames = Vec::new();
        for line in &self.statements {
            run.statement(line, &mut frames);
        }
        frames
    }
}
