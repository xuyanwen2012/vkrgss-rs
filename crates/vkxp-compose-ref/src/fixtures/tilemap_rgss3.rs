//! Scenes with tilemaps of RGSS3, as the measurement of the original runtime of VX Ace made
//! them.
//!
//! As the scenes of [`super::tilemap_rgss2`], whose sheets, generator and scene these use,
//! everything is rebuilt by formula: the maps of four layers and the flags tables of the
//! measurement's topics, and its random scenes with the seed used there. The expected frames
//! are committed as digests in `tests/data/tilemap-rgss3-digests.txt` of this crate. They
//! were produced by the model of the measurement, which reproduces every frame captured from
//! the original, not by a compositor of this repository.

use vkxp_proto::PlaneRules;
use vkxp_proto::tiles_rgss2 as tiles;

use super::tilemap_rgss2::{
    PyRandom, Scene, Set, Table, Topic, Variation, random_number, random_passages, random_scene,
};

/// The names of all topics, in a fixed order.
pub const TOPICS: [&str; 4] = ["tables2", "shadow3", "edge1", "rand4"];

/// The topic called `name`.
///
/// # Panics
///
/// If there is no such topic.
pub fn topic(name: &str) -> Topic {
    let (name, build): (&'static str, fn(&mut Scene)) = match name {
        "tables2" => ("tables2", tables2),
        "shadow3" => ("shadow3", shadow3),
        "edge1" => ("edge1", edge1),
        "rand4" => ("rand4", rand4),
        other => panic!("no topic `{other}`"),
    };
    let mut scene = Scene::default();
    build(&mut scene);
    Topic {
        name,
        frames: scene.frames,
    }
}

const A1: i16 = 2048;
const A2: i16 = 2816;
const A3: i16 = 4352;
const A4: i16 = 5888;
const A5: i16 = 1536;

/// An element of a map: ((x, y, layer), value).
type Cell = ((usize, usize, usize), i16);

/// A map of `width` by `height` cells and four layers from the cells given.
fn map(width: usize, height: usize, cells: impl IntoIterator<Item = Cell>) -> Table {
    let mut data = vec![0; width * height * 4];
    for ((x, y, layer), value) in cells {
        data[x + width * (y + height * layer)] = value;
    }
    Table {
        sizes: (width as u32, height as u32, 4),
        data,
    }
}

/// A map of 17 by 13 cells.
fn grid(cells: impl IntoIterator<Item = Cell>) -> Table {
    map(17, 13, cells)
}

/// A flags table of 8192 elements, all `default` except the entries (number, flags) given.
fn flags(default: i16, entries: impl IntoIterator<Item = (i16, i16)>) -> Table {
    let mut data = vec![default; 8192];
    for (number, flags) in entries {
        data[number as usize] = flags;
    }
    Table {
        sizes: (8192, 1, 1),
        data,
    }
}

/// A tilemap of RGSS3 without a viewport, with the sheets of one set, a flags table if one
/// is given, and the map `map`.
fn tilemap(scene: &mut Scene, set: Set, flags: Option<&Table>, map: Table) -> usize {
    let tilemap = scene.tilemap_of(PlaneRules::Rgss3, None, [Some(set); tiles::SHEETS]);
    if let Some(flags) = flags {
        scene.set(tilemap, |tilemap| tilemap.passages = Some(flags.clone()));
    }
    scene.set(tilemap, |tilemap| tilemap.map = Some(map));
    tilemap
}

/// Shape `shape` of the autotile `kind` of A2, from 0 to 31.
const fn a2(kind: i16, shape: i16) -> i16 {
    A2 + 48 * kind + shape
}

/// The topic `tables1` of RGSS2 with the bit 0x80 in the flags of every number: a table
/// tile of three shapes in each layer over each kind of tile in each layer.
fn tables2(scene: &mut Scene) {
    let below = [
        A1 + 20,
        a2(2, 0),
        a2(15, 20),
        A3 + 3,
        A4 + 5,
        A4 + 48 * 8 + 2,
        A5 + 9,
        37,
    ];
    let mut combos = Vec::new();
    for shape in [28, 4, 47] {
        for layer in 0..3 {
            for other in 0..3 {
                for under in below {
                    combos.push((shape, layer, other, under));
                }
            }
        }
    }
    let flags = flags(0x80, []);
    for page in combos.chunks(36) {
        let mut cells = Vec::new();
        for (i, &(shape, layer, other, under)) in page.iter().enumerate() {
            let (x, y) = (i % 9 * 2, i / 9 * 3);
            cells.push(((x, y, layer), a2(7, shape)));
            cells.push(((x, y + 1, other), under));
        }
        tilemap(scene, Set::Code, Some(&flags), grid(cells));
        scene.capture();
        scene.clear();
    }
}

/// The fourth layer: every value from 0 to 441 and values with higher bits, over a floor,
/// over nothing with a sprite under the tilemap, and under a tile of the third layer; then
/// five masks under and over tiles of each layer, a star tile and a table among them.
fn shadow3(scene: &mut Scene) {
    let high: [i32; 12] = [
        0x100, 0x10f, 0x3f05, 0x7fff, -1, -32768, -32753, 0xff0f, 0x1000, 0x10, 0xf0, 0x1f,
    ];
    let grounds = [
        (Set::Code, a2(2, 0), None),
        (Set::Flat, A5 + 9, None),
        (Set::Code, 0, Some((200, 60, 120, 255))),
        (Set::Code, 37, Some((255, 255, 255, 255))),
    ];
    for base in [0, 221, 442] {
        for (set, floor, ground) in grounds {
            let mut cells = Vec::new();
            for i in 0..221usize {
                let (x, y) = (i % 17, i / 17);
                if floor != 0 {
                    cells.push(((x, y, if floor == 37 { 2 } else { 0 }), floor));
                }
                let mask = if base == 442 {
                    // As a signed element of 16 bits.
                    high[i % 12] as i16
                } else {
                    (base + i) as i16
                };
                cells.push(((x, y, 3), mask));
            }
            if let Some(color) = ground {
                scene.sprite(None, 0, 0, -10, 544, 416, color);
            }
            tilemap(scene, set, None, grid(cells));
            scene.capture();
            scene.clear();
        }
    }

    let tiles = [a2(2, 0), A1 + 5, A3 + 3, A4 + 5, A5 + 9, 37, 300, a2(7, 28)];
    let star = flags(0, [(300, 0x10), (a2(7, 28), 0x80)]);
    for mask in [15, 5, 10, 1, 8] {
        let mut cells = Vec::new();
        let mut i = 0;
        for layer in 0..3 {
            for tile in tiles {
                let (x, y) = (i % 8 * 2, i / 8 * 4);
                cells.push(((x, y, layer), tile));
                cells.push(((x, y, 3), mask));
                // A second tile over or under it in another layer.
                cells.push(((x, y + 2, layer), tile));
                cells.push(((x, y + 2, (layer + 1) % 3), A5 + 20));
                cells.push(((x, y + 2, 3), mask));
                i += 1;
            }
        }
        scene.sprite(None, 0, 0, -10, 544, 416, (90, 160, 240, 255));
        tilemap(scene, Set::Half, Some(&star), grid(cells.clone()));
        scene.capture();
        scene.clear();
        tilemap(scene, Set::Code, Some(&star), grid(cells));
        scene.capture();
        scene.clear();
    }
}

/// A unit of the topic `edge1`: a few cells inside two by three.
type Unit = Vec<Cell>;

/// Packs units of two by three cells into maps, 32 a frame, and captures each with the
/// sprites `strips` across it, given as (y, height, colour) at z 100.
fn units(scene: &mut Scene, units: &[Unit], set: Set, flags: &Table, strips: &[Strip]) {
    for page in units.chunks(32) {
        let mut cells = Vec::new();
        for (i, unit) in page.iter().enumerate() {
            let (x, y) = (i % 8 * 2, i / 8 * 3);
            let placed = unit.iter();
            cells.extend(placed.map(|&((dx, dy, layer), value)| ((x + dx, y + dy, layer), value)));
        }
        tilemap(scene, set, Some(flags), grid(cells));
        for &(y, height, color) in strips {
            scene.sprite(None, 0, y, 100, 544, height, color);
        }
        scene.capture();
        scene.clear();
    }
}

/// A sprite across the screen: its y, its height and its colour.
type Strip = (i32, u32, (u8, u8, u8, u8));

/// The flags of the topic `edge1`: the autotiles 7, 12 and 31 of A2 are tables, the
/// autotile 20 has every bit but 0x80 and 0x10, and one tile each of A3, A4, A5, B and A1
/// carries 0x80 without being of A2.
fn edge_flags(table: i16) -> Table {
    let mut entries = Vec::new();
    for shape in 0..48 {
        entries.push((a2(7, shape), table));
        entries.push((a2(12, shape), 0x80));
        entries.push((a2(31, shape), 0x80));
        entries.push((a2(20, shape), 0x6f));
    }
    entries
        .extend([A3 + 48 * 3, A4 + 48 * 2, A5 + 30, 40, A1 + 48 * 4].map(|number| (number, 0x80)));
    flags(0, entries)
}

/// The lower edge of a table: which shapes give it, over which tiles of each layer of the
/// cell below, from a table in which layer; star tables; with a shadow in the cell; no
/// look-up across the edge of the map.
fn edge1(scene: &mut Scene) {
    let kinds: Vec<i16> = (0..128).map(|kind| A1 + 48 * kind).collect();
    let mut us: Vec<Unit> = Vec::new();
    for shape in 0..48 {
        us.push(vec![((0, 0, 1), a2(7, shape)), ((0, 1, 0), A5 + 9)]);
    }
    for &tile in kinds.iter().chain(&[A5 + 3, A5 + 30, 37, 40, 0]) {
        us.push(vec![((0, 0, 1), a2(12, 28)), ((0, 1, 0), tile)]);
    }
    for &tile in kinds.iter().chain(&[A5 + 3, A5 + 30, 37, 40]) {
        us.push(vec![((0, 0, 1), a2(31, 33)), ((0, 1, 1), tile)]);
    }
    for &tile in kinds.iter().step_by(4).chain(&[A5 + 30, 37, 40]) {
        us.push(vec![((0, 0, 1), a2(31, 46)), ((0, 1, 2), tile)]);
    }
    for layer in [0, 2] {
        for below in [0, A5 + 9, a2(2, 0), 37] {
            for other in 0..3 {
                us.push(vec![((0, 0, layer), a2(7, 28)), ((0, 1, other), below)]);
            }
        }
    }
    for table in [a2(7, 28), a2(12, 40), a2(31, 44), a2(20, 28)] {
        for wall in [A3 + 5, A4 + 5, A4 + 48 * 8 + 5, a2(7, 0), a2(3, 0)] {
            us.push(vec![
                ((0, 0, 1), table),
                ((0, 1, 0), wall),
                ((0, 1, 1), A5 + 20),
            ]);
            us.push(vec![
                ((0, 0, 1), table),
                ((0, 0, 0), wall),
                ((0, 1, 0), A5 + 20),
            ]);
        }
    }
    let plain = edge_flags(0x80);
    units(scene, &us, Set::Code, &plain, &[]);
    let mut some: Vec<Unit> = us[48..48 + 133].iter().step_by(3).cloned().collect();
    some.extend_from_slice(&us[us.len() - 40..]);
    units(scene, &some, Set::Half, &plain, &[]);

    // Star tables in each layer, with sprites at z 100 across.
    let star = edge_flags(0x90);
    let mut us = Vec::new();
    for layer in 0..3 {
        for shape in [28, 4, 47, 33] {
            for other in [0, 1] {
                for below in [A5 + 9, 0] {
                    us.push(vec![((0, 0, layer), a2(7, shape)), ((0, 1, other), below)]);
                }
            }
        }
    }
    let strips = [(40, 12, (30, 200, 90, 255)), (20, 6, (230, 20, 90, 128))];
    for set in [Set::Code, Set::Half] {
        units(scene, &us, set, &star, &strips);
    }

    // A shadow in the cell that takes the edge, and in the table's cell.
    for set in [Set::Code, Set::Half, Set::Alpha] {
        let mut cells = Vec::new();
        for i in 0..48usize {
            let (x, y) = (i % 8 * 2, i / 8 * 2);
            let below = [A5 + 9, 0, a2(2, 0)][i % 3];
            let shape = 28 + (i % 4) as i16;
            cells.push(((x, y, 1), a2(7, shape)));
            cells.push(((x, y + 1, 0), below));
            cells.push(((x, y + 1, 3), (i % 16) as i16));
            cells.push(((x, y, 3), (i * 7 % 16) as i16));
            cells.push(((x + 1, y, 0), a2(12, shape)));
            cells.push(((x + 1, y + 1, 1), below));
            cells.push(((x + 1, y + 1, 3), 15 - (i % 16) as i16));
        }
        scene.sprite(None, 0, 0, -10, 544, 416, (90, 160, 240, 255));
        tilemap(scene, set, Some(&plain), grid(cells));
        scene.capture();
        scene.clear();
    }

    // A table in the last row of the map, the first row of the map below it on the screen.
    for (width, height) in [(17, 13), (4, 3), (3, 1), (1, 1), (6, 2)] {
        let mut cells = Vec::new();
        for x in 0..width {
            for y in 0..height {
                cells.push(((x, y, 0), A5 + 16 + ((x + y) % 8) as i16));
                if (x + y) % 2 == 0 || height == 1 {
                    cells.push(((x, y, 1), a2(7, 28 + ((x * 3 + y) % 4) as i16)));
                }
            }
        }
        for (ox, oy) in [(0, 0), (0, 32), (16, 40), (-7, -13), (100, 77)] {
            let map = map(width, height, cells.iter().copied());
            let tilemap = tilemap(scene, Set::Code, Some(&plain), map);
            scene.set(tilemap, |tilemap| tilemap.ox = ox);
            scene.set(tilemap, |tilemap| tilemap.oy = oy);
            scene.capture();
            scene.clear();
        }
    }
}

/// A random element of the fourth layer: a mask, a mask under a region id, or an odd value.
fn random_mask(random: &mut PyRandom) -> i16 {
    let choice = random.random();
    if choice < 0.45 {
        0
    } else if choice < 0.8 {
        random.range(1, 16) as i16
    } else if choice < 0.95 {
        let region = random.range(1, 64);
        (region << 8 | random.range(0, 16)) as i16
    } else {
        random.choice(&[-1, -32768, 0x7fff, 0x10, 0xf0, 0xff, 0x100])
    }
}

/// Random flags as a tileset has them: one value for all shapes of most autotiles, the bit
/// 0x80 on many, stars and passage bits elsewhere, and some odd values.
fn random_flags(random: &mut PyRandom) -> Table {
    let any = [0, 0, 0, 0x0f, 0x10, 0x10, 0x40, 0x06, 0x400, 0x210];
    let mut data: Vec<i16> = (0..8192).map(|_| random.choice(&any)).collect();
    let whole = [
        0, 0, 0x80, 0x80, 0x90, 0x8f, 0xc0, 0x10, 0x1f, -1, 0x7fff, 0x100,
    ];
    for kind in 0..128 {
        let flags = random.choice(&whole);
        if random.random() < 0.8 {
            data[2048 + 48 * kind..][..48].fill(flags);
        }
    }
    Table {
        sizes: (8192, 1, 1),
        data,
    }
}

/// A random tile number of a layer: in the second layer half are of A2, most of them in a
/// shape with the lower edge.
fn random_number_of(random: &mut PyRandom, layer: usize) -> i16 {
    if layer == 1 && random.random() < 0.5 {
        let kind = random.range(0, 32);
        let any = random.range(0, 48);
        let shapes = [
            28, 29, 30, 31, 33, 38, 39, 40, 41, 43, 44, 45, 46, 47, 0, 4, 12, 20, any,
        ];
        return (2816 + 48 * kind + random.choice(&shapes)) as i16;
    }
    random_number(random)
}

/// A random map of four layers, each filled to the share given.
fn random_map(random: &mut PyRandom, width: u32, height: u32, fill: [f64; 4]) -> Table {
    let mut data = Vec::new();
    for (layer, share) in fill.into_iter().enumerate() {
        for _ in 0..width * height {
            let filled = random.random() < share;
            data.push(match (filled, layer) {
                (false, _) => 0,
                (true, 3) => random_mask(random),
                (true, _) => random_number_of(random, layer),
            });
        }
    }
    Table {
        sizes: (width, height, 4),
        data,
    }
}

/// The random scenes of the measurement of RGSS3 with the sheets of the set `Code`.
const RGSS3: Variation = Variation {
    rules: PlaneRules::Rgss3,
    sets: &[Set::Code],
    tables: |random, width, height| {
        let fill = [
            random.choice(&[1.0, 0.8, 0.3]),
            random.random(),
            random.random(),
            random.random(),
        ];
        let map = random_map(random, width, height, fill);
        let flags = if random.random() < 0.7 {
            random_flags(random)
        } else {
            random_passages(random)
        };
        (map, flags)
    },
    with_passages: 0.93,
    with_flash: 0.4,
};

/// The first forty scenes of the measurement's held-out random scenes of RGSS3, on which
/// nothing of the model was fitted.
fn rand4(scene: &mut Scene) {
    let mut random = PyRandom::new(31_337);
    for _ in 0..40 {
        random_scene(scene, &mut random, &RGSS3);
    }
}
