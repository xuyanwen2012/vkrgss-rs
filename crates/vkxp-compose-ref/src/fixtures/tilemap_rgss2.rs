//! Scenes with tilemaps of RGSS2, as the measurement of the original runtime of VX made them.
//!
//! Everything is rebuilt here by formula: the synthetic tile sheets, the maps of the
//! measurement's topics, and its random scenes, which come from the generator of the
//! measurement's scripts ([`PyRandom`]) with the seeds used there. The expected frames are
//! committed as digests ([`digest`]) in `tests/data/tilemap-rgss2-digests.txt` of this crate. They
//! were produced by the model of the measurement, which reproduces every frame captured from
//! the original, not by a compositor of this repository.
//!
//! A topic is a stream of frames for one compositor. Its first packets create the sheets;
//! every packet sets the tables it reads.

use vkxp_proto::tiles_rgss2 as tiles;
use vkxp_proto::{
    BitmapDelta, BitmapId, BlendMode, Color, FramePacket, FrameRate, GuestToHost, Op, Plane,
    PlaneRules, Quad, Rect, RectEffect, Sampling, Size, TableDelta, TableId, TilemapPlane, Tone,
};

/// Logical screen size of every frame: that of VX.
pub const SIZE: Size = Size::new(544, 416);

/// The sizes of the nine sheets, those of the run-time package of VX: A1 to A5, B to E.
pub const SHEET_SIZES: [(u32, u32); tiles::SHEETS] = [
    (512, 384),
    (512, 384),
    (512, 256),
    (512, 480),
    (256, 512),
    (512, 512),
    (512, 512),
    (512, 512),
    (512, 512),
];

/// A set of nine synthetic sheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Set {
    /// Opaque; every pixel of every sheet has a colour of its own that names the sheet and
    /// the position.
    Code,
    /// The colours of `Code` with an alpha that changes from pixel to pixel.
    Alpha,
    /// The colours of `Code` at alpha 128.
    Half,
    /// One opaque colour per sheet.
    Flat,
}

const SETS: [Set; 4] = [Set::Code, Set::Alpha, Set::Half, Set::Flat];

/// The pixel at (`x`, `y`) of sheet `sheet` of a set.
pub fn sheet_pixel(set: Set, sheet: usize, x: u32, y: u32) -> Color {
    let s = sheet as u32;
    if set == Set::Flat {
        return Color::new(
            (200 - 10 * s) as u8,
            (100 + 15 * s) as u8,
            (250 - 20 * s) as u8,
            255,
        );
    }
    let alpha = match set {
        Set::Alpha => (x * 29 + y * 53 + s * 7 + 40) as u8,
        Set::Half => 128,
        _ => 255,
    };
    Color::new(
        ((s + 1) << 4 | x >> 5) as u8,
        ((x & 31) << 3 | y >> 6) as u8,
        ((y & 63) << 2 | 3) as u8,
        alpha,
    )
}

/// The deltas that create sheet `sheet` of a set as the bitmap `id`.
pub fn sheet(id: BitmapId, set: Set, sheet: usize) -> [BitmapDelta; 2] {
    let (width, height) = SHEET_SIZES[sheet];
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&sheet_pixel(set, sheet, x, y).to_rgba());
        }
    }
    super::bitmap(id, width, height, rgba)
}

/// A tilemap plane op over the whole screen at the origin of a map, without sheets or other
/// tables, at animation step 0 and the flash level of a tilemap never updated.
pub fn tilemap_plane(plane: Plane, map: TableId) -> TilemapPlane {
    TilemapPlane {
        clip: Rect::from_size(SIZE),
        plane,
        rules: PlaneRules::Rgss2,
        corner: (0, 0),
        first_cell: (0, 0),
        animation_step: 0,
        flash_level: tiles::flash_level(0),
        map,
        passages: None,
        flash: None,
        bitmaps: [None; tiles::SHEETS],
    }
}

/// The digest the expected frames are committed as: FNV-1a of 64 bits over the bytes.
pub fn digest(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &byte in bytes {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The generator `random.Random` of Python, which made the random maps of the measurement:
/// MT19937 seeded with one integer below 2^32.
#[derive(Debug, Clone)]
pub struct PyRandom {
    state: [u32; 624],
    next: usize,
}

impl PyRandom {
    pub fn new(seed: u32) -> PyRandom {
        let mut state = [0u32; 624];
        state[0] = 19_650_218;
        for i in 1..624 {
            let previous = state[i - 1];
            state[i] = 1_812_433_253u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(i as u32);
        }
        // `init_by_array` with the one-element key `[seed]`.
        let mut i = 1;
        for _ in 0..624 {
            let previous = state[i - 1];
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(seed);
            i += 1;
            if i >= 624 {
                state[0] = state[623];
                i = 1;
            }
        }
        for _ in 0..623 {
            let previous = state[i - 1];
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                state[0] = state[623];
                i = 1;
            }
        }
        state[0] = 0x8000_0000;
        PyRandom { state, next: 624 }
    }

    fn word(&mut self) -> u32 {
        if self.next >= 624 {
            for i in 0..624 {
                let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
                let mut value = self.state[(i + 397) % 624] ^ (y >> 1);
                if y & 1 != 0 {
                    value ^= 0x9908_b0df;
                }
                self.state[i] = value;
            }
            self.next = 0;
        }
        let mut y = self.state[self.next];
        self.next += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// `random()`: a number from 0 to below 1 with 53 random bits.
    pub fn random(&mut self) -> f64 {
        let (a, b) = (self.word() >> 5, self.word() >> 6);
        (f64::from(a) * 67_108_864.0 + f64::from(b)) / 9_007_199_254_740_992.0
    }

    /// `randrange(n)`, for `n` from 1.
    pub fn below(&mut self, n: u32) -> u32 {
        let bits = 32 - n.leading_zeros();
        loop {
            let value = self.word() >> (32 - bits);
            if value < n {
                return value;
            }
        }
    }

    /// `randrange(start, stop)`.
    pub fn range(&mut self, start: i32, stop: i32) -> i32 {
        start + self.below((stop - start) as u32) as i32
    }

    /// `choice(items)`.
    pub fn choice<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len() as u32) as usize]
    }
}

/// A table as a scene holds it.
#[derive(Debug, Clone)]
struct Table {
    sizes: (u32, u32, u32),
    data: Vec<i16>,
}

/// A map of 17 by 13 cells and three layers from the cells given as ((x, y, layer), number).
fn grid(cells: impl IntoIterator<Item = ((usize, usize, usize), i16)>) -> Table {
    let mut data = vec![0; 17 * 13 * 3];
    for ((x, y, layer), number) in cells {
        data[x + 17 * (y + 13 * layer)] = number;
    }
    Table {
        sizes: (17, 13, 3),
        data,
    }
}

#[derive(Debug, Clone)]
struct Viewport {
    rect: Rect,
    ox: i32,
    oy: i32,
    z: i32,
    color: Color,
    tone: Tone,
}

#[derive(Debug, Clone)]
struct Sprite {
    viewport: Option<usize>,
    x: i32,
    y: i32,
    z: i32,
    width: u32,
    height: u32,
    color: Color,
}

#[derive(Debug, Clone)]
struct Tilemap {
    viewport: Option<usize>,
    sheets: [Option<Set>; tiles::SHEETS],
    map: Option<Table>,
    passages: Option<Table>,
    flash: Option<Table>,
    ox: i32,
    oy: i32,
    updates: u64,
    /// The ox and oy of the viewport as the last refresh found them.
    latched: (i32, i32),
}

#[derive(Debug, Clone)]
enum Element {
    Viewport(Viewport),
    Sprite(Sprite),
    Tilemap(Tilemap),
    Disposed,
}

/// The scene of a topic: the objects a script of the measurement made, in the order it made
/// them, and the frames captured so far.
#[derive(Debug, Default)]
struct Scene {
    elements: Vec<Element>,
    frames: Vec<GuestToHost>,
    /// The sheets created so far, by the index of their bitmap.
    sheets: Vec<u32>,
    /// The one-pixel bitmaps of the sprites of the last frame.
    colors: Vec<BitmapId>,
}

/// The index of the bitmap of a sheet of a set.
fn sheet_slot(set: Set, sheet: usize) -> u32 {
    (set as usize * tiles::SHEETS + sheet) as u32
}

/// The first index of the one-pixel bitmaps of sprites.
const COLOR_SLOTS: u32 = (SETS.len() * tiles::SHEETS) as u32;

impl Scene {
    fn add(&mut self, element: Element) -> usize {
        self.elements.push(element);
        self.elements.len() - 1
    }

    fn viewport(&mut self, rect: Rect) -> usize {
        self.add(Element::Viewport(Viewport {
            rect,
            ox: 0,
            oy: 0,
            z: 0,
            color: Color::new(0, 0, 0, 0),
            tone: Tone::NEUTRAL,
        }))
    }

    fn viewport_mut(&mut self, id: usize) -> &mut Viewport {
        match &mut self.elements[id] {
            Element::Viewport(viewport) => viewport,
            other => panic!("not a viewport: {other:?}"),
        }
    }

    #[expect(clippy::too_many_arguments, reason = "the arguments of the script")]
    fn sprite(
        &mut self,
        viewport: Option<usize>,
        x: i32,
        y: i32,
        z: i32,
        width: u32,
        height: u32,
        color: (u8, u8, u8, u8),
    ) {
        self.add(Element::Sprite(Sprite {
            viewport,
            x,
            y,
            z,
            width,
            height,
            color: Color::new(color.0, color.1, color.2, color.3),
        }));
    }

    /// Sprites side by side with the z values given.
    fn strips(&mut self, zs: &[i32], y: i32) {
        for (i, &z) in zs.iter().enumerate() {
            let i = i as i32;
            let color = ((250 - 20 * i) as u8, 250, (10 + 40 * i) as u8, 255);
            self.sprite(None, 4 + i * 88, y, z, 70, 300, color);
        }
    }

    fn tilemap(&mut self, viewport: Option<usize>, sheets: [Option<Set>; tiles::SHEETS]) -> usize {
        self.add(Element::Tilemap(Tilemap {
            viewport,
            sheets,
            map: None,
            passages: None,
            flash: None,
            ox: 0,
            oy: 0,
            updates: 0,
            latched: (0, 0),
        }))
    }

    /// Changes a tilemap as a setter or `update` does, and refreshes it: the origin of its
    /// viewport is latched, once the tilemap has a map.
    fn set(&mut self, id: usize, change: impl FnOnce(&mut Tilemap)) {
        let origin = |elements: &[Element], viewport: Option<usize>| match viewport {
            Some(viewport) => match &elements[viewport] {
                Element::Viewport(viewport) => (viewport.ox, viewport.oy),
                other => panic!("not a viewport: {other:?}"),
            },
            None => (0, 0),
        };
        let Element::Tilemap(tilemap) = &self.elements[id] else {
            panic!("not a tilemap");
        };
        let origin = origin(&self.elements, tilemap.viewport);
        let Element::Tilemap(tilemap) = &mut self.elements[id] else {
            unreachable!("checked above");
        };
        change(tilemap);
        if tilemap.map.is_some() {
            tilemap.latched = origin;
        }
    }

    fn dispose(&mut self, id: usize) {
        self.elements[id] = Element::Disposed;
    }

    fn clear(&mut self) {
        self.elements.clear();
    }

    /// A tilemap with the sheets of one set and the map `map`, as most topics begin.
    fn map_of(&mut self, set: Set, map: Table) -> usize {
        let tilemap = self.tilemap(None, [Some(set); tiles::SHEETS]);
        self.set(tilemap, |tilemap| tilemap.map = Some(map));
        tilemap
    }

    /// Shows one frame: the ops of the scene as it is, in the order of z, then y, then
    /// creation, where only a sprite has a y.
    fn capture(&mut self) {
        let mut frame = Capture {
            bitmaps: Vec::new(),
            tables: Vec::new(),
            ops: Vec::new(),
            index: self.frames.len() as u32,
            colors: Vec::new(),
        };
        for id in self.colors.drain(..) {
            frame.bitmaps.push(BitmapDelta::Destroy { id });
        }
        for element in &self.elements {
            let Element::Tilemap(tilemap) = element else {
                continue;
            };
            for (sheet, set) in tilemap.sheets.iter().enumerate() {
                let Some(set) = *set else { continue };
                let slot = sheet_slot(set, sheet);
                if !self.sheets.contains(&slot) {
                    self.sheets.push(slot);
                    frame
                        .bitmaps
                        .extend(self::sheet(BitmapId::new(slot, 0), set, sheet));
                }
            }
        }

        let screen = Rect::from_size(SIZE);
        let top = self.sorted(None);
        for (id, plane) in top {
            let Element::Viewport(viewport) = &self.elements[id] else {
                frame.draw(&self.elements[id], plane, screen, (0, 0), (0, 0));
                continue;
            };
            let rect = viewport.rect;
            let offset = (rect.x - viewport.ox, rect.y - viewport.oy);
            for (inner, plane) in self.sorted(Some(id)) {
                let corner = (rect.x, rect.y);
                frame.draw(&self.elements[inner], plane, rect, offset, corner);
            }
            if viewport.tone != Tone::NEUTRAL || viewport.color.a != 0 {
                frame.ops.push(Op::RectEffect(RectEffect {
                    clip: rect,
                    tone: viewport.tone,
                    color: viewport.color,
                }));
            }
        }
        self.colors = frame.colors;
        self.frames.push(GuestToHost::Frame(FramePacket {
            frame_index: u64::from(frame.index),
            logical_size: SIZE,
            frame_rate: FrameRate::MAX,
            brightness: 255,
            bitmaps: frame.bitmaps,
            tables: frame.tables,
            ops: frame.ops,
        }));
    }

    /// The elements of a viewport, or the viewports and the elements without one, in drawing
    /// order. A tilemap is there twice, once for each plane.
    fn sorted(&self, viewport: Option<usize>) -> Vec<(usize, Plane)> {
        let mut found = Vec::new();
        for (id, element) in self.elements.iter().enumerate() {
            match element {
                Element::Viewport(inner) if viewport.is_none() => {
                    found.push(((inner.z, 0, id), Plane::Lower));
                }
                Element::Sprite(sprite) if sprite.viewport == viewport => {
                    found.push(((sprite.z, sprite.y, id), Plane::Lower));
                }
                Element::Tilemap(tilemap) if tilemap.viewport == viewport => {
                    found.push(((0, 0, id), Plane::Lower));
                    found.push(((200, 0, id), Plane::Upper));
                }
                _ => {}
            }
        }
        found.sort_by_key(|(key, _)| *key);
        found
            .into_iter()
            .map(|((_, _, id), plane)| (id, plane))
            .collect()
    }
}

/// The packet of one frame while it is put together.
struct Capture {
    bitmaps: Vec<BitmapDelta>,
    tables: Vec<TableDelta>,
    ops: Vec<Op>,
    index: u32,
    colors: Vec<BitmapId>,
}

impl Capture {
    fn table(&mut self, table: &Table) -> TableId {
        let id = TableId::new(self.tables.len() as u32);
        self.tables
            .push(super::table(id, table.sizes, table.data.clone()));
        id
    }

    /// The op of a sprite, or of one plane of a tilemap, inside `clip`. A sprite at (0, 0) is
    /// at the screen pixel `offset`; the planes of a tilemap have their corner at the screen
    /// pixel `corner`, less what the tilemap is scrolled by within a cell.
    fn draw(
        &mut self,
        element: &Element,
        plane: Plane,
        clip: Rect,
        offset: (i32, i32),
        corner: (i32, i32),
    ) {
        match element {
            Element::Sprite(sprite) => {
                // One pixel of the sprite's colour, stretched over its rectangle.
                let slot = COLOR_SLOTS + self.colors.len() as u32;
                let id = BitmapId::new(slot, self.index);
                self.colors.push(id);
                self.bitmaps
                    .extend(super::bitmap(id, 1, 1, sprite.color.to_rgba().to_vec()));
                let dest = Rect::new(
                    sprite.x + offset.0,
                    sprite.y + offset.1,
                    sprite.width,
                    sprite.height,
                );
                self.ops.push(Op::Quad(Quad {
                    clip,
                    bitmap: id,
                    src: Rect::new(0, 0, 1, 1),
                    sampling: Sampling::Stretch {
                        dest,
                        mirror: false,
                    },
                    tone: Tone::NEUTRAL,
                    color: Color::new(0, 0, 0, 0),
                    opacity: 255,
                    blend: BlendMode::Normal,
                }));
            }
            Element::Tilemap(tilemap) => {
                let Some(map) = &tilemap.map else { return };
                if map.sizes.0 == 0 || map.sizes.1 == 0 {
                    return;
                }
                let x = tilemap.ox + tilemap.latched.0;
                let y = tilemap.oy + tilemap.latched.1;
                let first = |origin: i32, size: u32| {
                    i64::from(origin.div_euclid(32)).rem_euclid(i64::from(size)) as u32
                };
                let first_cell = (first(x, map.sizes.0), first(y, map.sizes.1));
                let map = self.table(map);
                let passages = tilemap.passages.as_ref().map(|table| self.table(table));
                let flash = tilemap.flash.as_ref().map(|table| self.table(table));
                let mut bitmaps = [None; tiles::SHEETS];
                for (sheet, set) in tilemap.sheets.iter().enumerate() {
                    bitmaps[sheet] = set.map(|set| BitmapId::new(sheet_slot(set, sheet), 0));
                }
                self.ops.push(Op::TilemapPlane(TilemapPlane {
                    clip,
                    plane,
                    rules: PlaneRules::Rgss2,
                    corner: (corner.0 - x.rem_euclid(32), corner.1 - y.rem_euclid(32)),
                    first_cell,
                    animation_step: tiles::animation_step(tilemap.updates),
                    flash_level: tiles::flash_level(tilemap.updates),
                    map,
                    passages,
                    flash,
                    bitmaps,
                }));
            }
            Element::Viewport(_) | Element::Disposed => {}
        }
    }
}

/// A named stream of frames, each of which has a committed digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topic {
    pub name: &'static str,
    pub frames: Vec<GuestToHost>,
}

/// The names of all topics, in a fixed order.
pub const TOPICS: [&str; 10] = [
    "ids", "tables1", "shadow1", "shadow2", "anim", "scroll1", "z1", "alpha1", "flash1", "rand1",
];

/// The topic called `name`.
///
/// # Panics
///
/// If there is no such topic.
pub fn topic(name: &str) -> Topic {
    let (name, build): (&'static str, fn(&mut Scene)) = match name {
        "ids" => ("ids", ids),
        "tables1" => ("tables1", tables1),
        "shadow1" => ("shadow1", shadow1),
        "shadow2" => ("shadow2", shadow2),
        "anim" => ("anim", anim),
        "scroll1" => ("scroll1", scroll1),
        "z1" => ("z1", z1),
        "alpha1" => ("alpha1", alpha1),
        "flash1" => ("flash1", flash1),
        "rand1" => ("rand1", rand1),
        other => panic!("no topic `{other}`"),
    };
    let mut scene = Scene::default();
    build(&mut scene);
    Topic {
        name,
        frames: scene.frames,
    }
}

const A2: i16 = 2816;
const A3: i16 = 4352;
const A4: i16 = 5888;
const A5: i16 = 1536;
const A1: i16 = 2048;

/// Every tile number that draws, alone among empty cells, in each of the three layers.
fn ids(scene: &mut Scene) {
    let numbers: Vec<i16> = (1..1024).chain(1536..8192).collect();
    for layer in 0..3 {
        for page in numbers.chunks(63) {
            let cells = page.iter().enumerate();
            let map = grid(cells.map(|(i, &number)| ((i % 9 * 2, i / 9 * 2, layer), number)));
            let tilemap = scene.map_of(Set::Code, map);
            scene.capture();
            scene.dispose(tilemap);
        }
    }
}

/// A table tile of three shapes in each layer over each kind of tile in each layer.
fn tables1(scene: &mut Scene) {
    let below = [
        A1 + 20,
        A2 + 48 * 2,
        A2 + 48 * 15 + 20,
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
    for page in combos.chunks(36) {
        let mut cells = Vec::new();
        for (i, &(shape, layer, other, under)) in page.iter().enumerate() {
            let (x, y) = (i % 9 * 2, i / 9 * 3);
            cells.push(((x, y, layer), A2 + 48 * 7 + shape));
            cells.push(((x, y + 1, other), under));
        }
        let tilemap = scene.map_of(Set::Code, grid(cells));
        scene.capture();
        scene.dispose(tilemap);
    }
}

/// Walls of several outlines on a floor.
fn shadow1(scene: &mut Scene) {
    let blocks = [
        (1, 1, 1, 1),
        (4, 1, 1, 2),
        (7, 1, 1, 3),
        (10, 1, 2, 2),
        (14, 1, 1, 4),
        (1, 6, 3, 1),
        (6, 6, 2, 3),
        (10, 7, 1, 1),
        (11, 8, 1, 1),
        (12, 9, 1, 1),
        (14, 7, 1, 2),
        (15, 9, 1, 2),
    ];
    for wall in [A3 + 15, A4 + 47, A4 + 48 * 8 + 15, A3, A4, A4 + 48 * 8] {
        for floor in [A2 + 48 * 2, A5 + 9, A1, 0, 37] {
            for set in [Set::Code, Set::Flat] {
                let mut cells = Vec::new();
                for y in 0..13 {
                    for x in 0..17 {
                        cells.push(((x, y, 0), floor));
                    }
                }
                for (left, top, width, height) in blocks {
                    for y in top..top + height {
                        for x in left..left + width {
                            cells.push(((x, y, 0), wall));
                        }
                    }
                }
                let tilemap = scene.map_of(set, grid(cells));
                scene.capture();
                scene.dispose(tilemap);
            }
        }
    }
}

/// Which tiles take a shadow, which neighbours give one, which layers count: units of two
/// by two cells.
fn shadow2(scene: &mut Scene) {
    type Unit = Vec<((usize, usize, usize), i16)>;
    let wall = A3 + 15;
    let floor = A2 + 96;
    let kinds: Vec<i16> = (0..128).map(|kind| A1 + 48 * kind).collect();
    let fifth: Vec<i16> = kinds.iter().copied().step_by(5).collect();
    let mut units: Vec<Unit> = Vec::new();
    for &target in kinds.iter().chain(&[A5 + 9, 37, 0, 300, 1000]) {
        units.push(vec![
            ((0, 0, 0), wall),
            ((0, 1, 0), wall),
            ((1, 1, 0), target),
        ]);
    }
    for &both in kinds.iter().chain(&[A5 + 9, 37, 0]) {
        units.push(vec![
            ((0, 0, 0), both),
            ((0, 1, 0), both),
            ((1, 1, 0), floor),
        ]);
    }
    for &left in fifth.iter().chain(&[A5 + 9, 37, 0]) {
        units.push(vec![
            ((0, 0, 0), wall),
            ((0, 1, 0), left),
            ((1, 1, 0), floor),
        ]);
    }
    for &above in fifth.iter().chain(&[A5 + 9, 37, 0]) {
        units.push(vec![
            ((0, 0, 0), above),
            ((0, 1, 0), wall),
            ((1, 1, 0), floor),
        ]);
    }
    for left in 0..3 {
        for above in 0..3 {
            for target in 0..3 {
                units.push(vec![
                    ((0, 0, above), wall),
                    ((0, 1, left), wall),
                    ((1, 1, target), floor),
                ]);
            }
        }
    }
    for layer in [1, 2] {
        for tile in [A2 + 48 * 5, 37, A5 + 9, A3 + 3] {
            let walls = [((0, 0, 0), wall), ((0, 1, 0), wall)];
            let mut unit = walls.to_vec();
            unit.extend([((1, 1, 0), floor), ((1, 1, layer), tile)]);
            units.push(unit);
            let mut unit = walls.to_vec();
            unit.push(((1, 1, layer), tile));
            units.push(unit);
        }
    }
    for layer in [1, 2] {
        for tile in [A2 + 48 * 5, 37] {
            let unit = [((0, 0, 0), wall), ((0, 1, 0), wall), ((1, 1, 0), floor)];
            let mut above = unit.to_vec();
            above.push(((0, 0, layer), tile));
            units.push(above);
            let mut left = unit.to_vec();
            left.push(((0, 1, layer), tile));
            units.push(left);
        }
    }
    for shape in 0..48 {
        units.push(vec![
            ((0, 0, 0), A3 + shape),
            ((0, 1, 0), A4 + shape),
            ((1, 1, 0), floor + shape),
        ]);
    }
    for page in units.chunks(20) {
        let mut cells = Vec::new();
        for (i, unit) in page.iter().enumerate() {
            let (x, y) = (i % 5 * 3, i / 5 * 3);
            cells.extend(
                unit.iter()
                    .map(|&((dx, dy, layer), number)| ((x + dx, y + dy, layer), number)),
            );
        }
        let tilemap = scene.map_of(Set::Code, grid(cells));
        scene.capture();
        scene.dispose(tilemap);
    }
}

/// The sixteen autotiles of A1 in several shapes and layers, at the animation steps 0 to 5.
fn anim(scene: &mut Scene) {
    let mut cells = Vec::new();
    for kind in 0..16 {
        let number = A1 + 48 * kind as i16;
        cells.push(((kind, 0, 0), number + 47));
        cells.push(((kind, 2, 0), number));
        cells.push(((kind, 4, 1), number + 1));
        cells.push(((kind, 6, 2), number + 3));
    }
    cells.extend([
        ((0, 8, 0), A2 + 5),
        ((1, 8, 0), A5 + 3),
        ((2, 8, 0), 77),
        ((3, 8, 0), A3 + 5),
        ((4, 8, 0), A4 + 5),
    ]);
    let tilemap = scene.map_of(Set::Code, grid(cells));
    scene.capture();
    for _ in 1..6 {
        scene.set(tilemap, |tilemap| tilemap.updates += 30);
        scene.capture();
    }
}

/// A random tile number, as the measurement drew them.
fn random_number(random: &mut PyRandom) -> i16 {
    let choice = random.random();
    let number = if choice < 0.10 {
        0
    } else if choice < 0.30 {
        random.range(1, 1024)
    } else if choice < 0.38 {
        random.range(1536, 1664)
    } else if choice < 0.40 {
        random.range(1664, 2048)
    } else if choice < 0.52 {
        random.range(2048, 2816)
    } else if choice < 0.72 {
        random.range(2816, 4352)
    } else if choice < 0.75 {
        let row = random.below(4) as i32;
        2816 + 48 * (8 * row + 7) + random.below(48) as i32
    } else if choice < 0.87 {
        random.range(4352, 5888)
    } else {
        random.range(5888, 8192)
    };
    number as i16
}

/// A random map of three layers, each filled to the share given.
fn random_map(random: &mut PyRandom, width: u32, height: u32, fill: [f64; 3]) -> Table {
    let mut data = Vec::new();
    for share in fill {
        for _ in 0..width * height {
            let filled = random.random() < share;
            data.push(if filled { random_number(random) } else { 0 });
        }
    }
    Table {
        sizes: (width, height, 3),
        data,
    }
}

/// A random passages table.
fn random_passages(random: &mut PyRandom) -> Table {
    let flags = [
        0, 0, 0x10, 0x10, 0x80, 0x90, 0x0f, 0x1f, 0x40, 0x20, 0xff, 0x100, -1,
    ];
    Table {
        sizes: (8192, 1, 1),
        data: (0..8192).map(|_| random.choice(&flags)).collect(),
    }
}

/// A random flash table.
fn random_flash(random: &mut PyRandom, width: u32, height: u32) -> Table {
    let mut data = Vec::new();
    for _ in 0..width * height {
        let (first, second) = (random.range(1, 4096), random.range(1, 4096));
        let entries = [
            0, 0, first, second, 0xf00, 0x0f0, 0x00f, 0xfff, 0x1234, 0x7fff, -1, -32768, 0x1000,
        ];
        data.push(random.choice(&entries) as i16);
    }
    Table {
        sizes: (width, height, 1),
        data,
    }
}

/// Maps smaller and larger than the screen, scrolled by any amount in both directions.
fn scroll1(scene: &mut Scene) {
    let mut random = PyRandom::new(101);
    let offsets = [
        (0, 0),
        (1, 0),
        (0, 1),
        (31, 33),
        (-1, -1),
        (-40, 7),
        (5, -100),
        (544, 416),
        (1000, 3000),
        (123_456, -98_765),
        (-7, 9),
    ];
    for (width, height) in [(17, 13), (5, 4), (20, 15), (1, 1), (3, 30), (40, 2)] {
        let map = random_map(&mut random, width, height, [1.0, 0.4, 0.3]);
        let tilemap = scene.map_of(Set::Code, map);
        for (ox, oy) in offsets {
            scene.set(tilemap, |tilemap| tilemap.ox = ox);
            scene.set(tilemap, |tilemap| tilemap.oy = oy);
            scene.capture();
        }
        scene.dispose(tilemap);
    }
}

/// The two planes among sprites at z -1, 0, 1, 199, 200 and 201, made before and after the
/// tilemap, at several y; random passages.
fn z1(scene: &mut Scene) {
    let mut random = PyRandom::new(202);
    let zs = [-1, 0, 1, 199, 200, 201];
    for before in [true, false] {
        for oy in [0, 5, 37] {
            for y in [0, -3, -8, 40] {
                let map = random_map(&mut random, 17, 13, [1.0, 0.6, 0.6]);
                let passages = random_passages(&mut random);
                if before {
                    scene.strips(&zs, y);
                }
                let tilemap = scene.tilemap(None, [Some(Set::Code); tiles::SHEETS]);
                scene.set(tilemap, |tilemap| tilemap.passages = Some(passages));
                scene.set(tilemap, |tilemap| tilemap.map = Some(map));
                scene.set(tilemap, |tilemap| tilemap.oy = oy);
                if !before {
                    scene.strips(&zs, y);
                }
                scene.capture();
                scene.clear();
            }
        }
    }
}

/// Translucent tiles over black, over a colour and over white, with shadows, tables and
/// tiles of the upper plane.
fn alpha1(scene: &mut Scene) {
    let mut random = PyRandom::new(303);
    for set in [Set::Alpha, Set::Half] {
        for ground in [None, Some((200, 60, 120, 255)), Some((255, 255, 255, 255))] {
            for (ox, oy) in [(0, 0), (13, 21)] {
                let map = random_map(&mut random, 17, 13, [1.0, 0.7, 0.7]);
                let passages = random_passages(&mut random);
                if let Some(color) = ground {
                    scene.sprite(None, 0, 0, -10, 544, 416, color);
                }
                let tilemap = scene.tilemap(None, [Some(set); tiles::SHEETS]);
                scene.set(tilemap, |tilemap| tilemap.passages = Some(passages));
                scene.set(tilemap, |tilemap| tilemap.map = Some(map));
                scene.set(tilemap, |tilemap| tilemap.ox = ox);
                scene.set(tilemap, |tilemap| tilemap.oy = oy);
                scene.sprite(None, 300, 100, 100, 150, 200, (30, 200, 90, 255));
                scene.capture();
                scene.clear();
            }
        }
    }
}

/// The flash: its colour and its level update by update, with sprites below, between and
/// above the planes; then flash tables of other sizes, scrolled.
fn flash1(scene: &mut Scene) {
    let mut random = PyRandom::new(404);
    let map = random_map(&mut random, 17, 13, [0.7, 0.3, 0.5]);
    let passages = random_passages(&mut random);
    let flash = random_flash(&mut random, 17, 13);
    scene.sprite(None, 0, 0, -10, 272, 416, (40, 90, 130, 255));
    let tilemap = scene.tilemap(None, [Some(Set::Code); tiles::SHEETS]);
    scene.set(tilemap, |tilemap| tilemap.passages = Some(passages));
    scene.set(tilemap, |tilemap| tilemap.map = Some(map));
    scene.capture();
    scene.set(tilemap, |tilemap| tilemap.flash = Some(flash));
    scene.sprite(None, 100, 50, 100, 300, 120, (100, 100, 100, 255));
    scene.sprite(None, 150, 200, 300, 300, 100, (10, 20, 30, 255));
    scene.capture();
    for _ in 0..40 {
        scene.set(tilemap, |tilemap| tilemap.updates += 1);
        scene.capture();
    }
    scene.set(tilemap, |tilemap| tilemap.flash = None);
    scene.capture();
    scene.clear();
    for (width, height) in [(5, 4), (30, 20), (1, 1), (17, 13)] {
        let map = random_map(&mut random, 9, 7, [0.7, 0.3, 0.5]);
        let flash = random_flash(&mut random, width, height);
        let tilemap = scene.map_of(Set::Flat, map);
        scene.set(tilemap, |tilemap| tilemap.flash = Some(flash));
        for (ox, oy, updates) in [(0, 0, 0), (40, 7, 9), (-100, -50, 5), (1000, 2000, 7)] {
            scene.set(tilemap, |tilemap| tilemap.ox = ox);
            scene.set(tilemap, |tilemap| tilemap.oy = oy);
            scene.set(tilemap, |tilemap| tilemap.updates += updates);
            scene.capture();
        }
        scene.clear();
    }
}

/// One random scene: a map of any size, perhaps in a viewport with an origin, a colour, a
/// tone and a z, over a ground or not, with sheets of any set or none, a flash table,
/// scrolled, updated, among sprites.
fn random_scene(scene: &mut Scene, random: &mut PyRandom) {
    let sizes = [
        (17, 13),
        (20, 15),
        (30, 25),
        (7, 5),
        (3, 9),
        (50, 4),
        (1, 1),
        (2, 2),
        (100, 100),
    ];
    let (width, height) = random.choice(&sizes);
    let fill = [random.choice(&[1.0, 0.8]), random.random(), random.random()];
    let map = random_map(random, width, height, fill);
    let passages = random_passages(random);
    let mut viewport = None;
    if random.random() < 0.5 {
        let rect = Rect::new(
            random.range(-60, 200),
            random.range(-60, 150),
            random.range(100, 640) as u32,
            random.range(80, 480) as u32,
        );
        let id = scene.viewport(rect);
        viewport = Some(id);
        if random.random() < 0.5 {
            scene.viewport_mut(id).ox = random.range(-70, 70);
            scene.viewport_mut(id).oy = random.range(-70, 70);
        }
        if random.random() < 0.3 {
            let mut channel = || random.below(256) as u8;
            scene.viewport_mut(id).color = Color::new(channel(), channel(), channel(), channel());
        }
        if random.random() < 0.3 {
            let mut channel = || random.range(-255, 256) as i16;
            let (red, green, blue) = (channel(), channel(), channel());
            let gray = random.choice(&[0, 0, 100, 255]);
            scene.viewport_mut(id).tone = Tone::new(red, green, blue, gray);
        }
        if random.random() < 0.3 {
            scene.viewport_mut(id).z = random.choice(&[-5, 50, 150, 250]);
        }
    }
    if random.random() < 0.6 {
        let mut channel = || random.below(256) as u8;
        let color = (channel(), channel(), channel(), 255);
        let inside = if random.random() < 0.5 {
            viewport
        } else {
            None
        };
        scene.sprite(inside, 0, 0, -20, 544, 416, color);
    }
    let mut sheets = [None; tiles::SHEETS];
    if random.random() < 0.5 {
        for sheet in &mut sheets {
            if random.random() < 0.93 {
                *sheet = Some(random.choice(&SETS));
            }
        }
    } else {
        sheets = [Some(random.choice(&SETS)); tiles::SHEETS];
    }
    let tilemap = scene.tilemap(viewport, sheets);
    if random.random() < 0.9 {
        scene.set(tilemap, |tilemap| tilemap.passages = Some(passages));
    }
    scene.set(tilemap, |tilemap| tilemap.map = Some(map));
    if random.random() < 0.5 {
        let (width, height) = random.choice(&[(width, height), (5, 4), (17, 13)]);
        let flash = random_flash(random, width, height);
        scene.set(tilemap, |tilemap| tilemap.flash = Some(flash));
    }
    let mut origin = || {
        let (far, near) = (random.range(-2000, 2000), random.range(-40, 40));
        random.choice(&[0, 0, far, near])
    };
    let (ox, oy) = (origin(), origin());
    scene.set(tilemap, |tilemap| tilemap.ox = ox);
    scene.set(tilemap, |tilemap| tilemap.oy = oy);
    if random.random() < 0.7 {
        let updates = random.range(1, 200) as u64;
        scene.set(tilemap, |tilemap| tilemap.updates += updates);
    }
    for _ in 0..random.below(5) {
        let inside = if random.random() < 0.5 {
            viewport
        } else {
            None
        };
        let (x, y) = (random.range(-50, 500), random.range(-50, 380));
        let z = random.choice(&[-1, 0, 0, 1, 100, 199, 200, 200, 201]);
        let (width, height) = (random.range(20, 200), random.range(20, 200));
        let mut channel = || random.below(256) as u8;
        let (red, green, blue) = (channel(), channel(), channel());
        let color = (red, green, blue, random.choice(&[255, 255, 128]));
        scene.sprite(inside, x, y, z, width as u32, height as u32, color);
    }
    scene.capture();
    scene.clear();
}

/// The first forty scenes of the measurement's random generator, which were held out when
/// the rules were fitted.
fn rand1(scene: &mut Scene) {
    let mut random = PyRandom::new(20_261_012);
    for _ in 0..40 {
        random_scene(scene, &mut random);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first numbers of `random.Random(101)` of Python.
    #[test]
    fn the_generator_is_that_of_python() {
        let mut random = PyRandom::new(101);
        assert_eq!(
            random.random().to_bits(),
            0.581_152_132_504_564_7_f64.to_bits()
        );
        let mut random = PyRandom::new(101);
        let drawn: Vec<i32> = (0..5).map(|_| random.range(-2000, 2000)).collect();
        assert_eq!(drawn, [380, 1469, -1203, 1720, 1953]);
        assert_eq!(random.choice(&[10, 20, 30, 40, 50]), 50);
    }

    #[test]
    fn every_pixel_of_the_code_set_names_its_place() {
        assert_eq!(sheet_pixel(Set::Code, 0, 0, 0), Color::new(16, 0, 3, 255));
        assert_eq!(
            sheet_pixel(Set::Code, 8, 511, 511),
            Color::new(159, 255, 255, 255)
        );
        assert_eq!(sheet_pixel(Set::Half, 3, 40, 70).a, 128);
        assert_eq!(
            sheet_pixel(Set::Alpha, 1, 3, 5).a,
            ((87 + 265 + 7 + 40) % 256) as u8
        );
        assert_eq!(
            sheet_pixel(Set::Flat, 4, 9, 9),
            Color::new(160, 160, 170, 255)
        );
    }
}
