//! The scenes of `rgss2-scene-cases.txt`: programs measured on the original runtime of VX,
//! each frame compared by its digest. The file says what its lines mean.

use std::collections::HashMap;

use vkxp_compose_ref::Frame;
use vkxp_pixel::Rgba;
use vkxp_proto::BitmapId;

use super::testing::*;
use crate::{Color, PlaneId, Rect, Rules, SpriteId, Tone, ViewportId};

const CASES: &str = include_str!("../../tests/data/rgss2-scene-cases.txt");

/// The size of the frame of the measurement.
const WIDTH: u32 = 544;
const HEIGHT: u32 = 416;

#[derive(Debug, Clone, Copy)]
enum Element {
    Sprite(SpriteId),
    Plane(PlaneId),
    Viewport(ViewportId),
}

/// The digest of a frame: FNV-1a, 64 bits, over red, green and blue of every pixel.
fn digest(frame: &Frame) -> u64 {
    let pixels = frame.rgba.as_chunks::<4>().0;
    let bytes = pixels.iter().flat_map(|pixel| &pixel[..3]);
    bytes.fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The texel of a bitmap at (`x`, `y`).
type Texel = Box<dyn Fn(u32, u32) -> Rgba>;

/// The size of the bitmap `name` and its texels.
fn named(name: &str) -> (u32, u32, Texel) {
    let parts: Vec<&str> = name.split('_').collect();
    let number = |at: usize| parts[at].parse::<u32>().unwrap();
    let low = |value: u32| value as u8;
    match parts[0] {
        "bg" => {
            let texel = move |x: u32, y: u32| {
                let (x, y) = (x & 63, y & 63);
                [
                    low(x * 4 + y),
                    low(y * 4 + 17 * x),
                    low(x * y + 3 * x + 60),
                    255,
                ]
            };
            (WIDTH, HEIGHT, Box::new(texel))
        }
        "cb" => {
            let opaque = parts[1] == "o";
            let texel = move |x: u32, y: u32| {
                let [r, g, b] = [
                    [x, 0, 0],
                    [0, x, 0],
                    [0, 0, x],
                    [x, 255 - x, 128],
                    [x, x / 2, 255 - x],
                    [255 - x, x, x * 3],
                    [x, x, x],
                    [x * 7, x * 13, x * 29],
                ][y as usize];
                let alpha = if opaque { 255 } else { low(x * 5 + y * 37) };
                [low(r), low(g), low(b), alpha]
            };
            (256, 8, Box::new(texel))
        }
        "pat" => {
            let (opaque, seed) = (parts[3] == "o", number(4));
            let texel = move |x: u32, y: u32| {
                let alpha = if opaque {
                    255
                } else {
                    low(x * 29 + y * 53 + seed * 7 + 40)
                };
                [
                    low(x * 9 + y * 3 + seed * 31 + 10),
                    low(y * 13 + x * 2 + seed * 17 + 20),
                    low(x * 5 + y * 7 + seed * 11 + 90),
                    alpha,
                ]
            };
            (number(1), number(2), Box::new(texel))
        }
        "solid" => {
            let pixel = [3, 4, 5, 6].map(|at| low(number(at)));
            (number(1), number(2), Box::new(move |_, _| pixel))
        }
        "colx" => {
            let texel = move |x: u32, _| [low(x), 255 - low(x), 128, 255];
            (number(1), number(2), Box::new(texel))
        }
        "rowy" => {
            let texel = move |_, y: u32| [low(y), 255 - low(y), 128, 255];
            (number(1), number(2), Box::new(texel))
        }
        _ => panic!("no bitmap is named `{name}`"),
    }
}

/// A scene of the file as far as its program has run.
struct Run {
    rules: Rules,
    stage: Stage,
    elements: HashMap<u32, Element>,
    bitmaps: HashMap<&'static str, BitmapId>,
}

impl Run {
    fn new(rules: Rules) -> Run {
        Run {
            rules,
            stage: Stage::with_rules(rules, WIDTH, HEIGHT),
            elements: HashMap::new(),
            bitmaps: HashMap::new(),
        }
    }

    fn bitmap(&mut self, name: &'static str) -> BitmapId {
        if let Some(&id) = self.bitmaps.get(name) {
            return id;
        }
        let (width, height, texel) = named(name);
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&texel(x, y));
            }
        }
        let id = self.stage.bitmap(width, height, rgba);
        self.bitmaps.insert(name, id);
        id
    }

    fn viewport(&self, word: &str) -> Option<ViewportId> {
        (word != "-").then(|| match self.elements[&word.parse().unwrap()] {
            Element::Viewport(id) => id,
            other => panic!("{other:?} is not a viewport"),
        })
    }

    /// One line of a program other than `frame`.
    fn step(&mut self, line: &'static str) {
        let words: Vec<&'static str> = line.split(' ').collect();
        let id = || words[1].parse::<u32>().unwrap();
        let numbers = |from: usize| -> Vec<f64> {
            let number = |word: &&str| word.parse::<f64>().unwrap();
            words[from..].iter().map(number).collect()
        };
        match words[0] {
            "clear" => *self = Run::new(self.rules),
            "sprite" => {
                let viewport = self.viewport(words[2]);
                let sprite = self.stage.scene.create_sprite(viewport).unwrap();
                self.elements.insert(id(), Element::Sprite(sprite));
            }
            "plane" => {
                let viewport = self.viewport(words[2]);
                let plane = self.stage.scene.create_plane(viewport).unwrap();
                self.elements.insert(id(), Element::Plane(plane));
            }
            "viewport" => {
                let [x, y, width, height] = numbers(2)[..] else {
                    panic!("{line}");
                };
                let rect = Rect::new(x as i32, y as i32, width as i32, height as i32);
                let viewport = self.stage.scene.create_viewport(rect).unwrap();
                self.elements.insert(id(), Element::Viewport(viewport));
            }
            "flash" => {
                let color = (words[3] != "nil").then(|| {
                    let color = numbers(3);
                    Color::new(color[0], color[1], color[2], color[3])
                });
                let duration = words[2].parse().unwrap();
                let scene = &mut self.stage.scene;
                match self.elements[&id()] {
                    Element::Sprite(id) => scene.sprite_mut(id).unwrap().flash(color, duration),
                    Element::Viewport(id) => {
                        scene.viewport_mut(id).unwrap().flash(color, duration);
                    }
                    Element::Plane(_) => panic!("{line}"),
                }
            }
            "update" => {
                let scene = &mut self.stage.scene;
                match self.elements[&id()] {
                    Element::Sprite(id) => scene.sprite_mut(id).unwrap().update(),
                    Element::Viewport(id) => scene.viewport_mut(id).unwrap().update(),
                    Element::Plane(_) => panic!("{line}"),
                }
            }
            "set" => self.set(self.elements[&id()], words[2], &words[3..], line),
            _ => panic!("{line}"),
        }
    }

    fn set(&mut self, element: Element, property: &str, words: &[&'static str], line: &str) {
        let values: Vec<f64> = words.iter().flat_map(|word| word.parse()).collect();
        // What an integer property holds of a number.
        let whole = || values[0] as i32;
        let flag = || words[0] == "true";
        let color = || Color::new(values[0], values[1], values[2], values[3]);
        let tone = || Tone::new(values[0], values[1], values[2], values[3]);
        match (element, property) {
            (Element::Sprite(id), "bitmap") => {
                let bitmap = self.bitmap(words[0]);
                let sprite = self.stage.scene.sprite_mut(id).unwrap();
                sprite.set_bitmap(Some(bitmap), &self.stage.bitmaps);
            }
            (Element::Plane(id), "bitmap") => {
                let bitmap = self.bitmap(words[0]);
                self.stage.scene.plane_mut(id).unwrap().bitmap = Some(bitmap);
            }
            (Element::Sprite(id), "viewport") => {
                let viewport = self.viewport(words[0]);
                self.stage.scene.set_sprite_viewport(id, viewport).unwrap();
            }
            (Element::Sprite(id), "z") => self.stage.scene.set_sprite_z(id, whole()).unwrap(),
            (Element::Plane(id), "z") => self.stage.scene.set_plane_z(id, whole()).unwrap(),
            (Element::Viewport(id), "z") => {
                self.stage.scene.set_viewport_z(id, whole()).unwrap();
            }
            (Element::Sprite(id), _) => {
                let sprite = self.stage.scene.sprite_mut(id).unwrap();
                match property {
                    "x" => sprite.x = whole(),
                    "y" => sprite.y = whole(),
                    "ox" => sprite.ox = whole(),
                    "oy" => sprite.oy = whole(),
                    "opacity" => sprite.set_opacity(whole()),
                    "blend_type" => sprite.blend_type = whole(),
                    "visible" => sprite.visible = flag(),
                    "color" => sprite.color = color(),
                    "tone" => sprite.tone = tone(),
                    _ => panic!("{line}"),
                }
            }
            (Element::Plane(id), _) => {
                let plane = self.stage.scene.plane_mut(id).unwrap();
                match property {
                    "ox" => plane.ox = whole(),
                    "oy" => plane.oy = whole(),
                    "zoom_x" => plane.zoom_x = values[0],
                    "zoom_y" => plane.zoom_y = values[0],
                    "opacity" => plane.set_opacity(whole()),
                    "blend_type" => plane.blend_type = whole(),
                    "color" => plane.color = color(),
                    "tone" => plane.tone = tone(),
                    _ => panic!("{line}"),
                }
            }
            (Element::Viewport(id), _) => {
                let viewport = self.stage.scene.viewport_mut(id).unwrap();
                match property {
                    "ox" => viewport.ox = whole(),
                    "oy" => viewport.oy = whole(),
                    "color" => viewport.color = color(),
                    "tone" => viewport.tone = tone(),
                    "rect" => {
                        let [x, y, width, height] = values[..] else {
                            panic!("{line}");
                        };
                        viewport.rect = Rect::new(x as i32, y as i32, width as i32, height as i32);
                    }
                    _ => panic!("{line}"),
                }
            }
        }
    }
}

/// Runs the programs of `topic` by `rules` and compares every frame with its digest: the
/// first of its line for RGSS2, the second for RGSS1. Returns the number of frames.
fn check(topic: &str, rules: Rules) -> usize {
    let mut run = Run::new(rules);
    let mut inside = false;
    let mut frames = 0;
    for line in CASES.lines().filter(|line| !line.starts_with('#')) {
        if let Some(name) = line.strip_prefix("topic ") {
            inside = name == topic;
            continue;
        }
        if !inside {
            continue;
        }
        let Some(digests) = line.strip_prefix("frame ") else {
            run.step(line);
            continue;
        };
        let column = match rules {
            Rules::Rgss2 => 0,
            Rules::Rgss1 => 1,
        };
        let expected = digests.split(' ').nth(column).unwrap();
        let found = format!("{:016x}", digest(&run.stage.frame()));
        assert_eq!(found, expected, "frame {frames} of `{topic}` by {rules:?}");
        frames += 1;
    }
    frames
}

/// Scenario "Cases of the measurement".
#[test]
fn the_order_of_rgss2_is_the_measured_one() {
    assert_eq!(check("order", Rules::Rgss2), 5);
}

/// Scenario "Cases of the measurement": the same scenes keep the order of RGSS1.
#[test]
fn the_scenes_of_the_order_cases_keep_the_order_of_rgss1() {
    assert_eq!(check("order", Rules::Rgss1), 5);
}

/// Scenario "Colour and flash, measured on the original".
#[test]
fn viewports_of_rgss2_show_the_measured_frames() {
    assert_eq!(check("viewport", Rules::Rgss2), 9);
    assert_eq!(check("flash", Rules::Rgss2), 14);
}

#[test]
fn planes_of_rgss2_show_the_measured_frames() {
    assert_eq!(check("plane", Rules::Rgss2), 10);
    assert_eq!(check("size2", Rules::Rgss2), 5);
}
