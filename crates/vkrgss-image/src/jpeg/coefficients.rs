//! Marker parsing and Huffman decoding of baseline and progressive JPEG into coefficient
//! blocks, in the order of operations of IJG libjpeg 6b (`jdmarker.c`, `jdhuff.c`,
//! `jdphuff.c`).
//!
//! Every function returns `None` where this decoder gives up: a layout it does not handle, or
//! data that does not decode the plain way. It checks nothing beyond what decoding needs.

use crate::ImageFormat;

/// The position in a block, in row order, of each coefficient in the order of the file.
const ZIGZAG: [u8; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// One component of a decoded frame.
pub(super) struct Component {
    /// Sampling factors.
    pub(super) h: usize,
    pub(super) v: usize,
    /// The size of the block grid, which covers whole MCUs.
    pub(super) blocks_wide: usize,
    pub(super) blocks_high: usize,
    /// The quantisation table, in row order.
    pub(super) quant: [u16; 64],
    /// 64 coefficients per block, in row order, not dequantised. They are 16 bits wide as in
    /// libjpeg.
    pub(super) coefficients: Vec<i16>,
}

/// The coefficients of a whole file.
pub(super) struct Frame {
    pub(super) width: u32,
    pub(super) height: u32,
    /// One component for greyscale, or Y, Cb and Cr.
    pub(super) components: Vec<Component>,
}

struct Pending {
    id: u8,
    table: usize,
    /// Whether a scan has latched the quantisation table, as libjpeg does at the first scan a
    /// component is part of.
    latched: bool,
    /// The size in samples, which a scan of this component alone covers.
    width: usize,
    height: usize,
    component: Component,
}

/// What a frame header says.
struct Header {
    coding: Coding,
    width: u32,
    height: u32,
    /// The size of the image in MCUs.
    mcus: (usize, usize),
    components: Vec<Pending>,
}

/// The bytes of the file, read as markers and segments or, inside a scan, as bits.
struct Stream<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// The bits read ahead; the low `count` bits are valid.
    acc: u64,
    count: u32,
    /// How many of the valid bits are zeros supplied after the end of the entropy-coded data.
    padding: u32,
}

impl<'a> Stream<'a> {
    fn byte(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    /// The next marker, skipping whatever precedes it as `next_marker` of libjpeg does.
    fn marker(&mut self) -> Option<u8> {
        loop {
            while self.byte()? != 0xFF {}
            let mut code = self.byte()?;
            while code == 0xFF {
                code = self.byte()?;
            }
            if code != 0 {
                return Some(code);
            }
        }
    }

    /// The contents of the segment that follows a marker.
    fn segment(&mut self) -> Option<&'a [u8]> {
        let length = usize::from(u16::from_be_bytes([
            *self.bytes.get(self.pos)?,
            *self.bytes.get(self.pos + 1)?,
        ]));
        let segment = self.bytes.get(self.pos + 2..self.pos + length)?;
        self.pos += length;
        Some(segment)
    }

    fn fill(&mut self) {
        while self.count <= 56 {
            let byte = match (self.bytes.get(self.pos), self.bytes.get(self.pos + 1)) {
                (Some(0xFF), Some(0)) => {
                    self.pos += 2;
                    0xFF
                }
                // A marker or the end of the data: the scan gets zeros from here on.
                (Some(0xFF), _) | (None, _) => {
                    self.padding += 8;
                    0
                }
                (Some(&byte), _) => {
                    self.pos += 1;
                    byte
                }
            };
            self.acc = (self.acc << 8) | u64::from(byte);
            self.count += 8;
        }
    }

    /// The next 16 bits, without taking them.
    fn peek(&mut self) -> i32 {
        if self.count < 16 {
            self.fill();
        }
        ((self.acc >> (self.count - 16)) & 0xFFFF) as i32
    }

    /// Takes bits that were looked at. Taking bits past the end of the data gives up.
    fn consume(&mut self, bits: u32) -> Option<()> {
        self.count -= bits;
        (self.count >= self.padding).then_some(())
    }

    fn bits(&mut self, bits: u32) -> Option<i32> {
        if bits > 16 {
            return None;
        }
        let value = self.peek() >> (16 - bits);
        self.consume(bits)?;
        Some(value)
    }

    /// Leaves the entropy-coded data: the bits read ahead are dropped.
    fn end_bits(&mut self) {
        (self.acc, self.count, self.padding) = (0, 0, 0);
    }
}

struct Huffman {
    /// The largest code of each length, or -1 where a length has none.
    max_code: [i32; 17],
    /// What to add to a code of each length to get the index of its value.
    offset: [i32; 17],
    values: Vec<u8>,
}

impl Huffman {
    fn new(counts: &[u8], values: &[u8]) -> Self {
        let (mut max_code, mut offset) = ([-1; 17], [0; 17]);
        let (mut code, mut index) = (0i32, 0i32);
        for (length, &count) in (1..).zip(counts) {
            offset[length] = index - code;
            code += i32::from(count);
            index += i32::from(count);
            if count != 0 {
                max_code[length] = code - 1;
            }
            code <<= 1;
        }
        Self {
            max_code,
            offset,
            values: values.to_vec(),
        }
    }

    fn decode(&self, stream: &mut Stream) -> Option<u8> {
        let peek = stream.peek();
        for length in 1..=16 {
            let code = peek >> (16 - length);
            if code <= self.max_code[length] {
                stream.consume(length as u32)?;
                let index = usize::try_from(code + self.offset[length]).ok()?;
                return self.values.get(index).copied();
            }
        }
        None
    }
}

/// `HUFF_EXTEND` of libjpeg: the value of `bits` received bits.
fn extend(value: i32, bits: u32) -> i32 {
    if bits != 0 && value < 1 << (bits - 1) {
        value - (1 << bits) + 1
    } else {
        value
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Coding {
    Baseline,
    Progressive,
}

/// What a scan header says, with the tables it names.
struct Scan<'a> {
    /// The index of each component with its DC and AC table.
    components: Vec<(usize, Option<&'a Huffman>, Option<&'a Huffman>)>,
    /// `None` for a baseline scan, else the first and last coefficient and the bit positions.
    progression: Option<(usize, usize, u32, u32)>,
}

/// The state that a restart marker resets.
#[derive(Default)]
struct Run {
    predictions: [i32; 3],
    /// The blocks still covered by an end-of-band run of a progressive scan.
    end_of_band: i32,
}

fn baseline_block(
    stream: &mut Stream,
    dc: &Huffman,
    ac: &Huffman,
    prediction: &mut i32,
    block: &mut [i16],
) -> Option<()> {
    let size = u32::from(dc.decode(stream)?);
    let difference = extend(stream.bits(size)?, size);
    *prediction = prediction.wrapping_add(difference);
    block[0] = *prediction as i16;
    let mut k = 1;
    while k < 64 {
        let symbol = ac.decode(stream)?;
        let (run, size) = (usize::from(symbol >> 4), u32::from(symbol & 15));
        if size != 0 {
            k += run;
            let value = extend(stream.bits(size)?, size);
            block[usize::from(*ZIGZAG.get(k)?)] = value as i16;
        } else if run != 15 {
            break;
        } else {
            k += 15;
        }
        k += 1;
    }
    Some(())
}

fn dc_first(
    stream: &mut Stream,
    dc: &Huffman,
    low: u32,
    prediction: &mut i32,
    block: &mut [i16],
) -> Option<()> {
    let size = u32::from(dc.decode(stream)?);
    let difference = extend(stream.bits(size)?, size);
    *prediction = prediction.wrapping_add(difference);
    block[0] = prediction.wrapping_shl(low) as i16;
    Some(())
}

fn ac_first(
    stream: &mut Stream,
    ac: &Huffman,
    (first, last, low): (usize, usize, u32),
    end_of_band: &mut i32,
    block: &mut [i16],
) -> Option<()> {
    if *end_of_band > 0 {
        *end_of_band -= 1;
        return Some(());
    }
    let mut k = first;
    while k <= last {
        let symbol = ac.decode(stream)?;
        let (run, size) = (u32::from(symbol >> 4), u32::from(symbol & 15));
        if size != 0 {
            k += run as usize;
            let value = extend(stream.bits(size)?, size);
            block[usize::from(*ZIGZAG.get(k)?)] = value.wrapping_shl(low) as i16;
        } else if run == 15 {
            k += 15;
        } else {
            *end_of_band = (1 << run) + stream.bits(run)? - 1;
            break;
        }
        k += 1;
    }
    Some(())
}

/// One more bit of a coefficient that is already non-zero.
fn refine(stream: &mut Stream, low: u32, coefficient: &mut i16) -> Option<()> {
    let bit = 1i32 << low;
    if stream.bits(1)? != 0 && i32::from(*coefficient) & bit == 0 {
        let step = if *coefficient >= 0 { bit } else { -bit };
        *coefficient = (i32::from(*coefficient) + step) as i16;
    }
    Some(())
}

fn ac_refine(
    stream: &mut Stream,
    ac: &Huffman,
    (first, last, low): (usize, usize, u32),
    end_of_band: &mut i32,
    block: &mut [i16],
) -> Option<()> {
    let mut k = first;
    if *end_of_band == 0 {
        while k <= last {
            let symbol = ac.decode(stream)?;
            let (mut run, size) = (i32::from(symbol >> 4), symbol & 15);
            let mut new = 0;
            if size != 0 {
                new = if stream.bits(1)? != 0 { 1 } else { -1 } << low;
            } else if run != 15 {
                *end_of_band = (1 << run) + stream.bits(run as u32)?;
                break;
            }
            // Pass the coefficients that are already non-zero, each of which takes a bit,
            // and `run` that are still zero.
            while k <= last {
                let coefficient = &mut block[usize::from(ZIGZAG[k])];
                if *coefficient != 0 {
                    refine(stream, low, coefficient)?;
                } else {
                    run -= 1;
                    if run < 0 {
                        break;
                    }
                }
                k += 1;
            }
            if new != 0 {
                if k > last {
                    return None;
                }
                block[usize::from(ZIGZAG[k])] = new as i16;
            }
            k += 1;
        }
    }
    if *end_of_band > 0 {
        while k <= last {
            let coefficient = &mut block[usize::from(ZIGZAG[k])];
            if *coefficient != 0 {
                refine(stream, low, coefficient)?;
            }
            k += 1;
        }
        *end_of_band -= 1;
    }
    Some(())
}

fn block(
    stream: &mut Stream,
    scan: &Scan,
    (slot, dc, ac): (usize, Option<&Huffman>, Option<&Huffman>),
    run: &mut Run,
    block: &mut [i16],
) -> Option<()> {
    let prediction = &mut run.predictions[slot];
    match scan.progression {
        None => baseline_block(stream, dc?, ac?, prediction, block),
        Some((0, _, 0, low)) => dc_first(stream, dc?, low, prediction, block),
        Some((0, _, _, low)) => {
            if stream.bits(1)? != 0 {
                block[0] |= 1i16.wrapping_shl(low);
            }
            Some(())
        }
        Some((first, last, 0, low)) => {
            ac_first(stream, ac?, (first, last, low), &mut run.end_of_band, block)
        }
        Some((first, last, _, low)) => {
            ac_refine(stream, ac?, (first, last, low), &mut run.end_of_band, block)
        }
    }
}

fn decode_scan(
    stream: &mut Stream,
    scan: &Scan,
    components: &mut [Pending],
    (mcus_wide, mcus_high): (usize, usize),
    restart_interval: usize,
) -> Option<()> {
    let interleaved = scan.components.len() > 1;
    let (wide, high) = if interleaved {
        (mcus_wide, mcus_high)
    } else {
        // A scan of one component covers the blocks that hold its samples, one per MCU.
        let pending = &components[scan.components[0].0];
        (pending.width.div_ceil(8), pending.height.div_ceil(8))
    };
    let mut run = Run::default();
    let mut restarts = 0u8;
    for mcu in 0..wide * high {
        if restart_interval != 0 && mcu != 0 && mcu % restart_interval == 0 {
            stream.end_bits();
            if stream.marker()? != 0xD0 + (restarts & 7) {
                return None;
            }
            restarts = restarts.wrapping_add(1);
            run = Run::default();
        }
        let (row, column) = (mcu / wide, mcu % wide);
        for (slot, &(index, dc, ac)) in scan.components.iter().enumerate() {
            let component = &mut components[index].component;
            let (h, v) = if interleaved {
                (component.h, component.v)
            } else {
                (1, 1)
            };
            for y in row * v..(row + 1) * v {
                for x in column * h..(column + 1) * h {
                    let start = (y * component.blocks_wide + x) * 64;
                    let coefficients = &mut component.coefficients[start..start + 64];
                    block(stream, scan, (slot, dc, ac), &mut run, coefficients)?;
                }
            }
        }
    }
    stream.end_bits();
    Some(())
}

fn read_frame(segment: &[u8], coding: Coding) -> Option<Header> {
    let (&[precision, h1, h0, w1, w0, count], rest) = segment.split_first_chunk()?;
    let (width, height) = (
        u32::from(u16::from_be_bytes([w1, w0])),
        u32::from(u16::from_be_bytes([h1, h0])),
    );
    if precision != 8 || !matches!(count, 1 | 3) {
        return None;
    }
    // The pixel limit also bounds what the coefficients need.
    crate::rgba_len(ImageFormat::Jpeg, width, height).ok()?;
    let mut factors = Vec::new();
    for entry in rest.get(..usize::from(count) * 3)?.as_chunks::<3>().0 {
        let &[id, sampling, table] = entry;
        let (h, v) = (usize::from(sampling >> 4), usize::from(sampling & 15));
        if !(1..=4).contains(&h) || !(1..=4).contains(&v) {
            return None;
        }
        factors.push((id, h, v, usize::from(table)));
    }
    // Colour is handled with chroma at full resolution or halved horizontally or in both
    // directions; every other layout is left to the library.
    if let [(_, h, v, _), chroma @ ..] = &factors[..]
        && !chroma.is_empty()
        && (!matches!((h, v), (1, 1) | (2, 1) | (2, 2))
            || chroma.iter().any(|&(_, h, v, _)| (h, v) != (1, 1)))
    {
        return None;
    }
    let (h_max, v_max) = (factors[0].1, factors[0].2);
    let (width_samples, height_samples) = (width as usize, height as usize);
    let mcus = (
        width_samples.div_ceil(8 * h_max),
        height_samples.div_ceil(8 * v_max),
    );
    let components = factors
        .into_iter()
        .map(|(id, h, v, table)| {
            let (blocks_wide, blocks_high) = (mcus.0 * h, mcus.1 * v);
            Pending {
                id,
                table,
                latched: false,
                width: (width_samples * h).div_ceil(h_max),
                height: (height_samples * v).div_ceil(v_max),
                component: Component {
                    h,
                    v,
                    blocks_wide,
                    blocks_high,
                    quant: [0; 64],
                    coefficients: vec![0; blocks_wide * blocks_high * 64],
                },
            }
        })
        .collect();
    Some(Header {
        coding,
        width,
        height,
        mcus,
        components,
    })
}

fn read_quant_tables(mut segment: &[u8], tables: &mut [Option<[u16; 64]>; 4]) -> Option<()> {
    while let Some((&head, rest)) = segment.split_first() {
        let mut table = [0; 64];
        if head >> 4 == 0 {
            let (values, rest) = rest.split_first_chunk::<64>()?;
            for (&position, &value) in ZIGZAG.iter().zip(values) {
                table[usize::from(position)] = u16::from(value);
            }
            segment = rest;
        } else {
            let (values, rest) = rest.split_first_chunk::<128>()?;
            for (&position, &value) in ZIGZAG.iter().zip(values.as_chunks::<2>().0) {
                table[usize::from(position)] = u16::from_be_bytes(value);
            }
            segment = rest;
        }
        *tables.get_mut(usize::from(head & 15))? = Some(table);
    }
    Some(())
}

fn read_huffman_tables(
    mut segment: &[u8],
    dc: &mut [Option<Huffman>; 4],
    ac: &mut [Option<Huffman>; 4],
) -> Option<()> {
    while let Some((&head, rest)) = segment.split_first() {
        let (counts, rest) = rest.split_first_chunk::<16>()?;
        let total = counts.iter().map(|&count| usize::from(count)).sum();
        let (values, rest) = rest.split_at_checked(total)?;
        let tables = if head >> 4 == 0 { &mut *dc } else { &mut *ac };
        *tables.get_mut(usize::from(head & 15))? = Some(Huffman::new(counts, values));
        segment = rest;
    }
    Some(())
}

fn read_scan<'a>(
    segment: &[u8],
    coding: Coding,
    components: &[Pending],
    dc: &'a [Option<Huffman>; 4],
    ac: &'a [Option<Huffman>; 4],
) -> Option<Scan<'a>> {
    let (&count, rest) = segment.split_first()?;
    let (entries, rest) = rest.split_at_checked(usize::from(count) * 2)?;
    let &[first, last, bits] = rest.first_chunk()?;
    if !(1..=components.len()).contains(&usize::from(count)) {
        return None;
    }
    let mut selected = Vec::new();
    for &[id, tables] in entries.as_chunks::<2>().0 {
        let index = components.iter().position(|pending| pending.id == id)?;
        let dc = dc.get(usize::from(tables >> 4)).and_then(Option::as_ref);
        let ac = ac.get(usize::from(tables & 15)).and_then(Option::as_ref);
        selected.push((index, dc, ac));
    }
    let progression = match coding {
        Coding::Baseline => None,
        Coding::Progressive => {
            let (first, last) = (usize::from(first), usize::from(last));
            // Coefficient 0 comes in scans of its own; the others one component at a time.
            if last > 63 || (first == 0) != (last == 0) || first > last || first != 0 && count != 1
            {
                return None;
            }
            Some((first, last, u32::from(bits >> 4), u32::from(bits & 15)))
        }
    };
    Some(Scan {
        components: selected,
        progression,
    })
}

/// Whether libjpeg takes three components for Y, Cb and Cr (`default_decompress_parms`).
fn is_ycbcr(jfif: bool, adobe_transform: Option<u8>, components: &[Pending]) -> bool {
    if jfif {
        true
    } else if let Some(transform) = adobe_transform {
        transform != 0
    } else {
        components.iter().map(|pending| pending.id).ne(*b"RGB")
    }
}

/// Reads a whole file. `bytes` starts with the start-of-image marker.
pub(super) fn read(bytes: &[u8]) -> Option<Frame> {
    let mut stream = Stream {
        bytes,
        pos: 2,
        acc: 0,
        count: 0,
        padding: 0,
    };
    let mut quant = [None; 4];
    let (mut dc, mut ac) = ([const { None }; 4], [const { None }; 4]);
    let mut restart_interval = 0;
    let (mut jfif, mut adobe_transform) = (false, None);
    let mut frame = None;

    // A file that ends without an end-of-image marker ends there, as in libjpeg.
    while let Some(marker) = stream.marker() {
        match marker {
            0xD9 => break,
            0x01 | 0xD0..=0xD7 => continue,
            0xD8 => return None,
            _ => {}
        }
        let segment = stream.segment()?;
        match marker {
            0xC0..=0xC2 => {
                if frame.is_some() {
                    return None;
                }
                let coding = if marker == 0xC2 {
                    Coding::Progressive
                } else {
                    Coding::Baseline
                };
                frame = Some(read_frame(segment, coding)?);
            }
            // Lossless, hierarchical and arithmetic-coded frames.
            0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF | 0xCC => return None,
            0xC4 => read_huffman_tables(segment, &mut dc, &mut ac)?,
            0xDB => read_quant_tables(segment, &mut quant)?,
            0xDD => restart_interval = usize::from(u16::from_be_bytes(*segment.first_chunk()?)),
            0xE0 => jfif |= segment.starts_with(b"JFIF\0"),
            0xEE if segment.starts_with(b"Adobe") => adobe_transform = segment.get(11).copied(),
            0xDA => {
                let Header {
                    coding,
                    mcus,
                    components,
                    ..
                } = frame.as_mut()?;
                if components.len() == 3 && !is_ycbcr(jfif, adobe_transform, components) {
                    return None;
                }
                let scan = read_scan(segment, *coding, components, &dc, &ac)?;
                for &(index, ..) in &scan.components {
                    let pending = &mut components[index];
                    if !pending.latched {
                        pending.component.quant = (*quant.get(pending.table)?)?;
                        pending.latched = true;
                    }
                }
                decode_scan(&mut stream, &scan, components, *mcus, restart_interval)?;
            }
            _ => {}
        }
    }

    let frame = frame?;
    if !frame.components.iter().all(|pending| pending.latched) {
        return None;
    }
    Some(Frame {
        width: frame.width,
        height: frame.height,
        components: frame
            .components
            .into_iter()
            .map(|pending| pending.component)
            .collect(),
    })
}
