//! Scenes of sprites of RGSS2 written as text, for the tests that compare frames with the
//! pictures of the measurement of 2026-10-12 on the original runtime of VX.
//!
//! A scene is the calls of a script on sprites, planes and viewports, on a screen of 544 by
//! 416 pixels with the rules of RGSS2, and after some of them a frame with the digest of
//! the picture the model of the measurement gives there ([`digest`]). Every bitmap is made
//! by a formula; no picture is read. One statement stands on a line; `#` begins a comment
//! line.
//!
//! ```text
//! scene <name>
//! sprite <id> <viewport id or -> [<property>=<value> ...]
//! plane <id> <viewport id or -> [<property>=<value> ...]
//! viewport <id> <x> <y> <width> <height> [<property>=<value> ...]
//! set <id> <property>=<value> ...
//! flash <id> <nil | red,green,blue,alpha> <duration>
//! update <id>
//! frame <digest>
//! end
//! ```
//!
//! A value is a number, `true`, `false`, `nil`, four numbers joined by commas (a colour, a
//! tone, a rectangle), the id of a viewport, or the name of a bitmap. A number given to a
//! property that holds an integer loses its fraction, as in the binding. A bitmap is named
//! by its formula, with (x, y) the pixel and every component taken modulo 256:
//!
//! - `pat_<width>_<height>_<o|a>_<seed>`: red `9x + 3y + 31 seed + 10`, green
//!   `13y + 2x + 17 seed + 20`, blue `5x + 7y + 11 seed + 90`; alpha 255 for `o` and
//!   `29x + 53y + 7 seed + 40` for `a`.
//! - `cb_<o|a>`: 256 by 8, a ramp of another colour in every row; alpha 255 for `o` and
//!   `5x + 37y` for `a`.
//! - `bg`: the screen, opaque, red `4x' + y'`, green `4y' + 17x'`, blue `x'y' + 3x' + 60`
//!   with `x'` and `y'` the coordinates modulo 64.
//! - `colx_<width>_<height>` and `rowy_<width>_<height>`: opaque, red the column or the row,
//!   green 255 less that, blue 128.

use std::collections::HashMap;

use vkxp_proto::{BitmapId, FramePacket};

pub use super::window_cases::{Case, SIZE, cases, digest};
use crate::{
    Bitmaps, Color, FontDefaults, PlaneId, Rect, Rules, Scene, SpriteId, Tone, ViewportId,
};

/// The pixel at (`x`, `y`) of a bitmap made by a formula.
type Formula = Box<dyn Fn(u32, u32) -> [u8; 4]>;

/// The width, the height and the formula of the bitmap `name`.
fn bitmap(name: &str) -> (u32, u32, Formula) {
    let words: Vec<&str> = name.split('_').collect();
    let number = |index: usize| -> u32 { number(words[index]) };
    match words[0] {
        "pat" => {
            let (opaque, seed) = (words[3] == "o", number(4));
            let pixel = move |x: u32, y: u32| {
                let alpha = if opaque {
                    255
                } else {
                    x * 29 + y * 53 + seed * 7 + 40
                };
                [
                    (x * 9 + y * 3 + seed * 31 + 10) as u8,
                    (y * 13 + x * 2 + seed * 17 + 20) as u8,
                    (x * 5 + y * 7 + seed * 11 + 90) as u8,
                    alpha as u8,
                ]
            };
            (number(1), number(2), Box::new(pixel))
        }
        "cb" => {
            let opaque = words[1] == "o";
            let pixel = move |x: u32, y: u32| {
                let [red, green, blue] = match y {
                    0 => [x, 0, 0],
                    1 => [0, x, 0],
                    2 => [0, 0, x],
                    3 => [x, 255 - x, 128],
                    4 => [x, x / 2, 255 - x],
                    5 => [255 - x, x, x * 3],
                    6 => [x, x, x],
                    _ => [x * 7, x * 13, x * 29],
                };
                let alpha = if opaque { 255 } else { x * 5 + y * 37 };
                [red as u8, green as u8, blue as u8, alpha as u8]
            };
            (256, 8, Box::new(pixel))
        }
        "bg" => {
            let pixel = |x: u32, y: u32| {
                let (x, y) = (x % 64, y % 64);
                [
                    (x * 4 + y) as u8,
                    (y * 4 + 17 * x) as u8,
                    (x * y + 3 * x + 60) as u8,
                    255,
                ]
            };
            (SIZE.width, SIZE.height, Box::new(pixel))
        }
        "colx" => {
            let pixel = |x: u32, _: u32| [x as u8, 255 - x as u8, 128, 255];
            (number(1), number(2), Box::new(pixel))
        }
        "rowy" => {
            let pixel = |_: u32, y: u32| [y as u8, 255 - y as u8, 128, 255];
            (number(1), number(2), Box::new(pixel))
        }
        other => panic!("unknown kind of bitmap `{other}`"),
    }
}

/// What an id of a scene names.
#[derive(Debug, Clone, Copy)]
enum Element {
    Sprite(SpriteId),
    Plane(PlaneId),
    Viewport(ViewportId),
}

/// A scene being run.
struct Run {
    scene: Scene,
    bitmaps: Bitmaps,
    defaults: FontDefaults,
    named: HashMap<String, BitmapId>,
    elements: HashMap<String, Element>,
}

fn number<T: std::str::FromStr>(word: &str) -> T {
    word.parse()
        .unwrap_or_else(|_| panic!("`{word}` is not a number"))
}

/// The integer a property takes of a number, which loses its fraction.
fn whole(word: &str) -> i32 {
    number::<f64>(word) as i32
}

fn four(word: &str) -> [f64; 4] {
    let parts: Vec<f64> = word.split(',').map(number).collect();
    parts
        .try_into()
        .unwrap_or_else(|_| panic!("`{word}` is not four numbers"))
}

fn color(word: &str) -> Color {
    let [red, green, blue, alpha] = four(word);
    Color::new(red, green, blue, alpha)
}

fn tone(word: &str) -> Tone {
    let [red, green, blue, gray] = four(word);
    Tone::new(red, green, blue, gray)
}

impl Run {
    /// The bitmap `name`, made when it is first named.
    fn bitmap(&mut self, name: &str) -> Option<BitmapId> {
        if name == "nil" {
            return None;
        }
        if let Some(id) = self.named.get(name) {
            return Some(*id);
        }
        let (width, height, pixel) = bitmap(name);
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
        self.named.insert(name.to_owned(), id);
        Some(id)
    }

    fn viewport(&self, word: &str) -> Option<ViewportId> {
        match self.elements.get(word) {
            Some(Element::Viewport(id)) => Some(*id),
            _ => None,
        }
    }

    fn create(&mut self, kind: &str, words: &[&str]) {
        let (element, properties) = match kind {
            "sprite" => {
                let viewport = self.viewport(words[1]);
                let id = self.scene.create_sprite(viewport).unwrap();
                (Element::Sprite(id), &words[2..])
            }
            "plane" => {
                let viewport = self.viewport(words[1]);
                let id = self.scene.create_plane(viewport).unwrap();
                (Element::Plane(id), &words[2..])
            }
            _ => {
                let [x, y, width, height] = std::array::from_fn(|index| number(words[1 + index]));
                let rect = Rect::new(x, y, width, height);
                let id = self.scene.create_viewport(rect).unwrap();
                (Element::Viewport(id), &words[5..])
            }
        };
        self.elements.insert(words[0].to_owned(), element);
        self.set(words[0], properties);
    }

    fn set(&mut self, id: &str, properties: &[&str]) {
        for property in properties {
            let (name, value) = property
                .split_once('=')
                .unwrap_or_else(|| panic!("`{property}` assigns nothing"));
            match self.elements[id] {
                Element::Sprite(id) => self.set_sprite(id, name, value),
                Element::Plane(id) => self.set_plane(id, name, value),
                Element::Viewport(id) => self.set_viewport(id, name, value),
            }
        }
    }

    fn set_sprite(&mut self, id: SpriteId, name: &str, value: &str) {
        let flag = value == "true";
        match name {
            "z" => return self.scene.set_sprite_z(id, whole(value)).unwrap(),
            "viewport" => {
                let viewport = self.viewport(value);
                return self.scene.set_sprite_viewport(id, viewport).unwrap();
            }
            "bitmap" => {
                let bitmap = self.bitmap(value);
                let sprite = self.scene.sprite_mut(id).unwrap();
                return sprite.set_bitmap(bitmap, &self.bitmaps);
            }
            _ => {}
        }
        let sprite = self.scene.sprite_mut(id).unwrap();
        match name {
            "x" => sprite.x = whole(value),
            "y" => sprite.y = whole(value),
            "ox" => sprite.ox = whole(value),
            "oy" => sprite.oy = whole(value),
            "zoom_x" => sprite.zoom_x = number(value),
            "zoom_y" => sprite.zoom_y = number(value),
            "angle" => sprite.angle = number(value),
            "mirror" => sprite.mirror = flag,
            "visible" => sprite.visible = flag,
            "bush_depth" => sprite.bush_depth = whole(value),
            "bush_opacity" => sprite.bush_opacity = whole(value),
            "wave_amp" => sprite.wave_amp = whole(value),
            "wave_length" => sprite.wave_length = whole(value),
            "wave_speed" => sprite.wave_speed = whole(value),
            "wave_phase" => sprite.set_wave_phase(number(value)),
            "opacity" => sprite.set_opacity(whole(value)),
            "blend_type" => sprite.blend_type = whole(value),
            "color" => sprite.color = color(value),
            "tone" => sprite.tone = tone(value),
            "src_rect" => {
                let [x, y, width, height] = four(value).map(|side| side as i32);
                sprite.src_rect = Rect::new(x, y, width, height);
            }
            other => panic!("unknown property `{other}` of a sprite"),
        }
    }

    fn set_plane(&mut self, id: PlaneId, name: &str, value: &str) {
        if name == "z" {
            return self.scene.set_plane_z(id, whole(value)).unwrap();
        }
        let bitmap = (name == "bitmap").then(|| self.bitmap(value));
        let plane = self.scene.plane_mut(id).unwrap();
        match name {
            "bitmap" => plane.bitmap = bitmap.flatten(),
            "ox" => plane.ox = whole(value),
            "oy" => plane.oy = whole(value),
            "zoom_x" => plane.zoom_x = number(value),
            "zoom_y" => plane.zoom_y = number(value),
            "opacity" => plane.set_opacity(whole(value)),
            other => panic!("unknown property `{other}` of a plane"),
        }
    }

    fn set_viewport(&mut self, id: ViewportId, name: &str, value: &str) {
        if name == "z" {
            return self.scene.set_viewport_z(id, whole(value)).unwrap();
        }
        let viewport = self.scene.viewport_mut(id).unwrap();
        match name {
            "ox" => viewport.ox = whole(value),
            "oy" => viewport.oy = whole(value),
            "visible" => viewport.visible = value == "true",
            "color" => viewport.color = color(value),
            "tone" => viewport.tone = tone(value),
            other => panic!("unknown property `{other}` of a viewport"),
        }
    }

    fn statement(&mut self, line: &str, frames: &mut Vec<(FramePacket, u64)>) {
        let words: Vec<&str> = line.split(' ').collect();
        let rest = &words[1..];
        match words[0] {
            kind @ ("sprite" | "plane" | "viewport") => self.create(kind, rest),
            "set" => self.set(rest[0], &rest[1..]),
            "flash" => {
                let color = (rest[1] != "nil").then(|| color(rest[1]));
                let duration = number(rest[2]);
                match self.elements[rest[0]] {
                    Element::Sprite(id) => {
                        let sprite = self.scene.sprite_mut(id).unwrap();
                        sprite.flash(color, duration);
                    }
                    Element::Viewport(id) => {
                        let viewport = self.scene.viewport_mut(id).unwrap();
                        viewport.flash(color, duration);
                    }
                    Element::Plane(_) => panic!("`{line}` flashes a plane"),
                }
            }
            "update" => match self.elements[rest[0]] {
                Element::Sprite(id) => self.scene.sprite_mut(id).unwrap().update(),
                Element::Viewport(id) => self.scene.viewport_mut(id).unwrap().update(),
                Element::Plane(_) => panic!("`{line}` updates a plane"),
            },
            "frame" => {
                let digest = u64::from_str_radix(rest[0], 16).expect("a digest in hexadecimal");
                frames.push((self.scene.extract(&mut self.bitmaps), digest));
            }
            other => panic!("unknown statement `{other}`"),
        }
    }
}

/// Runs a scene of sprites: the packet of every frame, in the order a compositor must get
/// them, each with the digest of the picture it must draw.
///
/// # Panics
///
/// When a statement cannot be read.
pub fn frames(case: &Case<'_>) -> Vec<(FramePacket, u64)> {
    let mut run = Run {
        scene: Scene::with_rules(Rules::Rgss2, SIZE),
        bitmaps: Bitmaps::new(),
        defaults: FontDefaults::new(Vec::new()),
        named: HashMap::new(),
        elements: HashMap::new(),
    };
    let mut frames = Vec::new();
    for line in case.statements() {
        run.statement(line, &mut frames);
    }
    frames
}
