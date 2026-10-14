//! The format of a sound, read from its header, and the position that `Audio.bgm_pos` and
//! `Audio.bgs_pos` of RGSS3 report for it, counted in frames shown.
//!
//! The guest computes the position itself and never asks the host: the audio device runs
//! on a clock of its own, a headless run has none, and what the guest does depends on its
//! ticks alone. The rule is the one measured on the original runtime, whose time is real
//! time; here one step is one frame shown, at the frame rate of that frame.
//!
//! Provenance: oracle measurement 2026-10-13, RGSS301 version 3.0.1.1 and RGSS300 version
//! 3.0.0.1, `ace-v1-api-REPORT.md` sections 6.2 and 6.3.

/// The sound of at most this many bytes of 16-bit PCM is not streamed by the original: its
/// position is a cursor in a buffer of the whole file. 529200 bytes were a whole buffer and
/// 530964 were streamed; the limit was not pinned between the two.
pub const STREAM_LIMIT: u64 = 529_200;

/// The chunks the original reads ahead when a streamed sound starts. Its buffer holds one
/// more, and the position is that of the decoder less the buffer.
const CHUNKS_AT_START: u64 = 15;
const CHUNKS_IN_BUFFER: u64 = 16;

/// What the position of a sound needs to know of its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundFormat {
    /// Sample frames per second.
    pub rate: u32,
    /// The channel count.
    pub channels: u16,
    /// The length in sample frames.
    pub frames: u64,
    /// The comment `LOOPSTART` of an Ogg Vorbis file, in sample frames.
    pub loop_start: Option<u64>,
    /// The comment `LOOPLENGTH` of an Ogg Vorbis file, in sample frames.
    pub loop_length: Option<u64>,
}

impl SoundFormat {
    /// The format of the file whose contents are `bytes`, from its header alone: a WAV file
    /// of 8-bit or 16-bit PCM, or an Ogg Vorbis file. `None` for anything else, for a header
    /// that is cut short and for a rate or channel count of 0.
    ///
    /// The frames of a WAV file are those of the bytes of its data chunk that the file
    /// holds. Those of an Ogg Vorbis file are the granule position of the last page that
    /// begins in the file (0 when it is negative); the checksums of pages are not checked.
    /// A comment named exactly `LOOPSTART` or `LOOPLENGTH`, in upper case, gives a tag when
    /// its value is decimal digits; the first comment of each name decides.
    pub fn probe(bytes: &[u8]) -> Option<SoundFormat> {
        let format = if bytes.starts_with(b"RIFF") {
            wav(bytes)?
        } else if bytes.starts_with(b"OggS") {
            ogg(bytes)?
        } else {
            return None;
        };
        (format.rate > 0 && format.channels > 0).then_some(format)
    }

    /// The bytes of one sample frame of 16-bit PCM, the unit of a position.
    pub fn frame_bytes(&self) -> u64 {
        u64::from(self.channels) * 2
    }

    /// The frames shown while the whole sound plays once at `pitch` percent, at `frame_rate`
    /// frames per second, rounded up.
    pub fn frames_shown(&self, pitch: u16, frame_rate: u32) -> u64 {
        let time = u128::from(self.frames) * 100 * u128::from(frame_rate);
        let rate = u128::from(self.rate) * u128::from(pitch.max(1));
        u64::try_from(time.div_ceil(rate)).unwrap_or(u64::MAX)
    }
}

/// Reads a little-endian word of `N` bytes at `at`.
fn le<const N: usize>(bytes: &[u8], at: usize) -> Option<[u8; N]> {
    bytes.get(at..at.checked_add(N)?)?.try_into().ok()
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    le(bytes, at).map(u16::from_le_bytes)
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    le(bytes, at).map(u32::from_le_bytes)
}

/// The format of a RIFF file: the `fmt ` chunk of PCM of 8 or 16 bits, then the `data`
/// chunk.
fn wav(bytes: &[u8]) -> Option<SoundFormat> {
    if bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut at = 12usize;
    let mut format = None;
    loop {
        let id = bytes.get(at..at + 4)?;
        let size = u32_at(bytes, at + 4)? as usize;
        let body = at + 8;
        match id {
            b"fmt " => {
                let tag = u16_at(bytes, body)?;
                let channels = u16_at(bytes, body + 2)?;
                let rate = u32_at(bytes, body + 4)?;
                let bits = u16_at(bytes, body + 14)?;
                if tag != 1 || !(bits == 8 || bits == 16) {
                    return None;
                }
                format = Some((
                    rate,
                    channels,
                    usize::from(channels) * usize::from(bits / 8),
                ));
            }
            b"data" => {
                let (rate, channels, block) = format?;
                let held = size.min(bytes.len() - body);
                return Some(SoundFormat {
                    rate,
                    channels,
                    frames: held.checked_div(block)? as u64,
                    loop_start: None,
                    loop_length: None,
                });
            }
            _ => {}
        }
        // Chunks are padded to an even size.
        at = body.checked_add(size)?.checked_add(size & 1)?;
    }
}

/// The fixed part of the header of an Ogg page, up to its segment table.
const PAGE_HEADER: usize = 27;

/// The format of an Ogg Vorbis file: the identification header and the comment header, the
/// first two packets of the first stream, and the granule position of the last page.
fn ogg(bytes: &[u8]) -> Option<SoundFormat> {
    let packets = first_packets(bytes)?;
    let [identification, comments] = &packets;
    if identification.get(..7)? != b"\x01vorbis" {
        return None;
    }
    let channels = *identification.get(11)?;
    let rate = u32_at(identification, 12)?;
    let (loop_start, loop_length) = loop_tags(comments)?;
    Some(SoundFormat {
        rate,
        channels: u16::from(channels),
        frames: last_granule(bytes)?,
        loop_start,
        loop_length,
    })
}

/// The first two packets of the pages that begin at the start of `bytes`, put together from
/// their segments across pages. Pages of another stream are passed over.
fn first_packets(bytes: &[u8]) -> Option<[Vec<u8>; 2]> {
    let serial = u32_at(bytes, 14)?;
    let mut done: Vec<Vec<u8>> = Vec::new();
    let mut packet = Vec::new();
    let mut at = 0usize;
    loop {
        if bytes.get(at..at + 4)? != b"OggS" {
            return None;
        }
        let segments = usize::from(*bytes.get(at + 26)?);
        let table = bytes.get(at + PAGE_HEADER..at + PAGE_HEADER + segments)?;
        let mut body = at + PAGE_HEADER + segments;
        let own = u32_at(bytes, at + 14)? == serial;
        for &lacing in table {
            let lacing = usize::from(lacing);
            if own {
                packet.extend_from_slice(bytes.get(body..body + lacing)?);
                if lacing < 255 {
                    done.push(std::mem::take(&mut packet));
                    if let [identification, comments] = &mut done[..] {
                        return Some([std::mem::take(identification), std::mem::take(comments)]);
                    }
                }
            }
            body += lacing;
        }
        at = body;
    }
}

/// The tags `LOOPSTART` and `LOOPLENGTH` of a Vorbis comment header. `None` for a header
/// that is not one or is cut short.
fn loop_tags(header: &[u8]) -> Option<(Option<u64>, Option<u64>)> {
    if header.get(..7)? != b"\x03vorbis" {
        return None;
    }
    let vendor = u32_at(header, 7)? as usize;
    let mut at = 11usize.checked_add(vendor)?;
    let count = u32_at(header, at)?;
    at += 4;
    let mut start = None;
    let mut length = None;
    for _ in 0..count {
        let size = u32_at(header, at)? as usize;
        let comment = header.get(at + 4..(at + 4).checked_add(size)?)?;
        at += 4 + size;
        let Some(split) = comment.iter().position(|&byte| byte == b'=') else {
            continue;
        };
        let (name, value) = (&comment[..split], &comment[split + 1..]);
        let slot = match name {
            b"LOOPSTART" => &mut start,
            b"LOOPLENGTH" => &mut length,
            _ => continue,
        };
        if slot.is_none() {
            *slot = Some(decimal(value));
        }
    }
    Some((start.flatten(), length.flatten()))
}

/// The value of decimal digits, `None` for anything else.
fn decimal(value: &[u8]) -> Option<u64> {
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(value).ok()?.parse().ok()
}

/// The granule position of the last page that begins in `bytes`, found from the end by its
/// capture pattern; 0 for a negative one.
fn last_granule(bytes: &[u8]) -> Option<u64> {
    let start = bytes
        .windows(14)
        .rposition(|window| window.starts_with(b"OggS"))?;
    let granule = i64::from_le_bytes(le(bytes, start + 6)?);
    Some(u64::try_from(granule).unwrap_or(0))
}

/// The position of a sound that plays, in bytes of 16-bit PCM from the start of its file,
/// as `Audio.bgm_pos` of RGSS3 reports it. It is started at the `pos` argument and stepped
/// once for every frame shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// Sample frames per second.
    rate: u64,
    /// The pitch in percent.
    pitch: u64,
    /// What a step adds up to a chunk or a sample frame, carried from step to step.
    clock: u64,
    kind: Kind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    /// A sound that is read in chunks.
    Streamed(Stream),
    /// A sound of at most [`STREAM_LIMIT`] bytes: a cursor that runs through it and wraps.
    Short {
        /// The start, a whole sample frame within the sound, in bytes.
        start: u64,
        /// The length in bytes.
        length: u64,
        /// The bytes of a sample frame.
        frame_bytes: u64,
        /// The sample frames counted since the start, less whole lengths.
        frames: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stream {
    /// One chunk: 187.5 ms of sound, rounded up to a whole sample frame.
    chunk: u64,
    /// The length in bytes.
    length: u64,
    /// Where the decoder jumps to at a limit.
    loop_start: u64,
    /// The first limit, behind which only the end of the file is one.
    loop_end: u64,
    /// Whether the decoder started at or behind the end of the file: then it only goes on.
    past_end: bool,
    /// The place of the decoder.
    decoder: u64,
}

impl Stream {
    /// Reads `bytes` bytes: on to the next limit, from there on at the loop start.
    fn read(&mut self, mut bytes: u64) {
        if self.past_end {
            self.decoder = self.decoder.saturating_add(bytes);
            return;
        }
        while bytes > 0 {
            let limit = if self.decoder < self.loop_end {
                self.loop_end
            } else {
                self.length
            };
            let taken = bytes.min(limit - self.decoder);
            self.decoder += taken;
            bytes -= taken;
            if self.decoder == limit {
                self.decoder = self.loop_start;
                // Each further round of the loop comes back to its start.
                bytes %= self.loop_end - self.loop_start;
            }
        }
    }
}

impl Position {
    /// A sound of `format` started at the byte `pos` of its 16-bit PCM, played at `pitch`
    /// percent.
    pub fn start(format: &SoundFormat, pos: u64, pitch: u16) -> Position {
        let frame_bytes = format.frame_bytes();
        let length = format.frames.saturating_mul(frame_bytes);
        let kind = if length <= STREAM_LIMIT {
            Kind::Short {
                start: if length == 0 {
                    0
                } else {
                    pos / frame_bytes * frame_bytes % length
                },
                length,
                frame_bytes,
                frames: 0,
            }
        } else {
            let per_second = u64::from(format.rate) * frame_bytes;
            let chunk = (per_second * 3 / 16).next_multiple_of(frame_bytes);
            let mut loop_start = format.loop_start.map_or(0, |start| start * frame_bytes);
            let mut loop_end = match format.loop_length {
                Some(loop_length) => format
                    .loop_start
                    .unwrap_or(0)
                    .saturating_add(loop_length)
                    .saturating_mul(frame_bytes)
                    .min(length),
                None => length,
            };
            // Tags that make no region are no tags (not measured).
            if loop_start >= loop_end {
                (loop_start, loop_end) = (0, length);
            }
            let mut stream = Stream {
                chunk,
                length,
                loop_start,
                loop_end,
                past_end: pos >= length,
                decoder: pos,
            };
            stream.read(CHUNKS_AT_START * chunk);
            Kind::Streamed(stream)
        };
        Position {
            rate: u64::from(format.rate),
            pitch: u64::from(pitch),
            clock: 0,
            kind,
        }
    }

    /// Steps the sound by one frame shown at `frame_rate` frames per second.
    ///
    /// A streamed sound reads one chunk for every 187.5 ms at pitch 100, which are 75 / 400
    /// seconds: a step adds 4 times the pitch to the clock, and a chunk is read for every
    /// 75 times the frame rate it holds. A short sound counts one sample frame for every 100
    /// times the frame rate of the clock, to which a step adds the rate times the pitch.
    pub fn step(&mut self, frame_rate: u32) {
        let frame_rate = u64::from(frame_rate.max(1));
        match &mut self.kind {
            Kind::Streamed(stream) => {
                let per_chunk = 75 * frame_rate;
                self.clock += 4 * self.pitch;
                let chunks = self.clock / per_chunk;
                self.clock %= per_chunk;
                stream.read(chunks.saturating_mul(stream.chunk));
            }
            Kind::Short {
                length,
                frame_bytes,
                frames,
                ..
            } => {
                let per_frame = 100 * frame_rate;
                self.clock += self.rate * self.pitch;
                let counted = self.clock / per_frame;
                self.clock %= per_frame;
                let total = *length / *frame_bytes;
                if total > 0 {
                    *frames = (*frames + counted % total) % total;
                }
            }
        }
    }

    /// The position in bytes of 16-bit PCM.
    pub fn bytes(&self) -> u64 {
        match &self.kind {
            Kind::Streamed(stream) => stream
                .decoder
                .saturating_sub(CHUNKS_IN_BUFFER * stream.chunk),
            Kind::Short {
                start,
                length,
                frame_bytes,
                frames,
            } => {
                if *length == 0 {
                    0
                } else {
                    (start + frame_bytes * frames) % length
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A WAV file of PCM whose data chunk declares `declared` bytes and holds `held`, after
    /// a chunk of another kind.
    fn wav(tag: u16, rate: u32, channels: u16, bits: u16, declared: u32, held: usize) -> Vec<u8> {
        let mut bytes = b"RIFF\0\0\0\0WAVE".to_vec();
        bytes.extend_from_slice(b"LIST\x03\0\0\0abc\0");
        bytes.extend_from_slice(b"fmt \x10\0\0\0");
        bytes.extend_from_slice(&tag.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        let block = channels * bits / 8;
        bytes.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
        bytes.extend_from_slice(&block.to_le_bytes());
        bytes.extend_from_slice(&bits.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&declared.to_le_bytes());
        bytes.resize(bytes.len() + held, 0);
        let riff = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&riff.to_le_bytes());
        bytes
    }

    /// The pages of an Ogg stream that hold `packets`, as many segments to a page as an
    /// Ogg page holds, then a page of its own with `granule`.
    fn ogg_pages(packets: &[Vec<u8>], granule: i64) -> Vec<u8> {
        let mut lacing = Vec::new();
        let mut data = Vec::new();
        for packet in packets {
            let mut rest = packet.len();
            loop {
                let segment = rest.min(255);
                lacing.push(segment as u8);
                rest -= segment;
                if segment < 255 {
                    break;
                }
            }
            data.extend_from_slice(packet);
        }
        let mut bytes = Vec::new();
        let mut at = 0;
        let page = |bytes: &mut Vec<u8>, granule: i64, table: &[u8], body: &[u8]| {
            bytes.extend_from_slice(b"OggS\0\0");
            bytes.extend_from_slice(&granule.to_le_bytes());
            bytes.extend_from_slice(&7u32.to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes.push(table.len() as u8);
            bytes.extend_from_slice(table);
            bytes.extend_from_slice(body);
        };
        for table in lacing.chunks(255) {
            let size: usize = table.iter().map(|&segment| usize::from(segment)).sum();
            page(&mut bytes, 0, table, &data[at..at + size]);
            at += size;
        }
        page(&mut bytes, granule, &[4], b"\0\0\0\0");
        bytes
    }

    fn identification(rate: u32, channels: u8) -> Vec<u8> {
        let mut packet = b"\x01vorbis\0\0\0\0".to_vec();
        packet.push(channels);
        packet.extend_from_slice(&rate.to_le_bytes());
        packet.extend_from_slice(&[0; 13]);
        packet.push(1);
        packet
    }

    fn comment_header(comments: &[&str]) -> Vec<u8> {
        let mut packet = b"\x03vorbis".to_vec();
        packet.extend_from_slice(&4u32.to_le_bytes());
        packet.extend_from_slice(b"test");
        packet.extend_from_slice(&(comments.len() as u32).to_le_bytes());
        for comment in comments {
            packet.extend_from_slice(&(comment.len() as u32).to_le_bytes());
            packet.extend_from_slice(comment.as_bytes());
        }
        packet.push(1);
        packet
    }

    fn ogg(rate: u32, channels: u8, comments: &[&str], granule: i64) -> Vec<u8> {
        let setup = b"\x05vorbis".to_vec();
        ogg_pages(
            &[
                identification(rate, channels),
                comment_header(comments),
                setup,
            ],
            granule,
        )
    }

    fn format(rate: u32, channels: u16, frames: u64) -> SoundFormat {
        SoundFormat {
            rate,
            channels,
            frames,
            loop_start: None,
            loop_length: None,
        }
    }

    fn tags(bytes: &[u8]) -> (Option<u64>, Option<u64>) {
        let format = SoundFormat::probe(bytes).unwrap();
        (format.loop_start, format.loop_length)
    }

    #[test]
    fn probe_of_wav() {
        let stereo = wav(1, 44100, 2, 16, 3_528_000, 3_528_000);
        assert_eq!(SoundFormat::probe(&stereo), Some(format(44100, 2, 882_000)));
        let mono = wav(1, 22050, 1, 8, 441_000, 441_000);
        assert_eq!(SoundFormat::probe(&mono), Some(format(22050, 1, 441_000)));
    }

    #[test]
    fn probe_of_wav_that_was_cut_short() {
        let cut = wav(1, 44100, 2, 16, 4000, 1000);
        assert_eq!(SoundFormat::probe(&cut), Some(format(44100, 2, 250)));
    }

    #[test]
    fn probe_of_ogg_vorbis() {
        let bytes = ogg(48000, 2, &["TITLE=tone"], 960_000);
        assert_eq!(SoundFormat::probe(&bytes), Some(format(48000, 2, 960_000)));
    }

    #[test]
    fn tags_in_upper_case_only() {
        let both = ogg(44100, 2, &["LOOPSTART=88200", "LOOPLENGTH=176400"], 1);
        assert_eq!(tags(&both), (Some(88200), Some(176_400)));
        let start = ogg(44100, 2, &["LOOPSTART=88200"], 1);
        assert_eq!(tags(&start), (Some(88200), None));
        let length = ogg(44100, 2, &["LOOPLENGTH=176400"], 1);
        assert_eq!(tags(&length), (None, Some(176_400)));
        let lower = ogg(44100, 2, &["loopstart=88200", "looplength=176400"], 1);
        assert_eq!(tags(&lower), (None, None));
    }

    #[test]
    fn comment_header_across_pages() {
        let long = "COMMENT=".to_owned() + &"x".repeat(240);
        let mut comments = vec![long.as_str(); 300];
        comments.push("LOOPSTART=5");
        let bytes = ogg(44100, 2, &comments, 1);
        // The comment header alone is longer than a page holds.
        assert!(comment_header(&comments).len() > 255 * 255);
        assert_eq!(tags(&bytes), (Some(5), None));
    }

    #[test]
    fn values_that_are_no_numbers() {
        let bytes = ogg(44100, 2, &["LOOPSTART=abc", "LOOPLENGTH="], 44100);
        assert_eq!(SoundFormat::probe(&bytes), Some(format(44100, 2, 44100)));
    }

    #[test]
    fn other_bytes() {
        let midi = b"MThd\0\0\0\x06\0\0\0\x01\x01\xe0MTrk\0\0\0\x04\0\xff\x2f\0".to_vec();
        let mp3 = b"ID3\x03\0\0\0\0\0\0\xff\xfb\x90\x64\0\0\0\0".to_vec();
        let asf = vec![
            0x30, 0x26, 0xb2, 0x75, 0x8e, 0x66, 0xcf, 0x11, 0xa6, 0xd9, 0x00, 0xaa, 0x00, 0x62,
            0xce, 0x6c, 0, 0, 0, 0,
        ];
        let text = b"This is no sound.\r\n".to_vec();
        let float = wav(3, 44100, 2, 16, 400, 400);
        for bytes in [midi, mp3, asf, text, Vec::new(), float] {
            assert_eq!(SoundFormat::probe(&bytes), None, "{bytes:?}");
        }
        // Neither a rate nor a channel count of 0.
        assert_eq!(SoundFormat::probe(&wav(1, 0, 2, 16, 4, 4)), None);
        assert_eq!(SoundFormat::probe(&ogg(44100, 0, &[], 1)), None);
    }

    #[test]
    fn any_prefix_and_any_changed_byte() {
        let wave = wav(1, 44100, 2, 16, 400, 400);
        let vorbis = ogg(44100, 2, &["LOOPSTART=5", "LOOPLENGTH=7"], 44100);
        for end in 0..=64 {
            let _ = SoundFormat::probe(&wave[..end]);
        }
        for end in 0..=vorbis.len() {
            let _ = SoundFormat::probe(&vorbis[..end]);
        }
        for (bytes, header) in [(&wave, 44), (&vorbis, vorbis.len())] {
            for at in 0..header {
                for value in [0x00, 0x01, 0x7f, 0x80, 0xfe, 0xff, bytes[at] ^ 0x5a] {
                    let mut changed = bytes.clone();
                    changed[at] = value;
                    let _ = SoundFormat::probe(&changed);
                }
            }
        }
    }

    #[test]
    fn frames_shown() {
        let effect = format(44100, 2, 88200);
        assert_eq!(effect.frames_shown(100, 60), 120);
        assert_eq!(effect.frames_shown(50, 60), 240);
        assert_eq!(format(44100, 2, 1).frames_shown(100, 60), 1);
    }

    /// The positions of `format` started at `pos` and read after each count of frames.
    fn readings(format: &SoundFormat, pos: u64, pitch: u16, rate: u32, at: &[u64]) -> Vec<u64> {
        let mut position = Position::start(format, pos, pitch);
        let mut shown = 0;
        at.iter()
            .map(|&frames| {
                while shown < frames {
                    position.step(rate);
                    shown += 1;
                }
                position.bytes()
            })
            .collect()
    }

    fn at_once(format: &SoundFormat, pos: u64) -> u64 {
        Position::start(format, pos, 100).bytes()
    }

    fn tagged(seconds: u64, start: Option<u64>, length: Option<u64>) -> SoundFormat {
        SoundFormat {
            loop_start: start,
            loop_length: length,
            ..format(44100, 2, 44100 * seconds)
        }
    }

    #[test]
    fn chunk() {
        let chunk = |rate, channels| {
            let position = Position::start(&format(rate, channels, u64::from(rate) * 20), 0, 100);
            match position.kind {
                Kind::Streamed(stream) => stream.chunk,
                Kind::Short { .. } => unreachable!(),
            }
        };
        assert_eq!(
            [
                chunk(44100, 2),
                chunk(44100, 1),
                chunk(22050, 2),
                chunk(22050, 1),
                chunk(48000, 2)
            ],
            [33076, 16538, 16540, 8268, 36000]
        );
    }

    #[test]
    fn start_of_a_streamed_sound() {
        let found: Vec<u64> = [(44100, 2), (44100, 1), (22050, 2), (22050, 1), (48000, 2)]
            .into_iter()
            .map(|(rate, channels)| at_once(&format(rate, channels, u64::from(rate) * 20), 44100))
            .collect();
        assert_eq!(found, [11024, 27562, 27560, 35832, 8100]);
        let s44 = tagged(20, None, None);
        assert_eq!(at_once(&s44, 100_001), 66925);
        assert_eq!(at_once(&s44, 2_147_483_647), 2_147_450_571);
        assert_eq!(at_once(&s44, 0), 0);
    }

    #[test]
    fn steps_at_60_frames_per_second() {
        let found = readings(
            &tagged(20, None, None),
            0,
            100,
            60,
            &[0, 12, 18, 24, 36, 48, 60, 72, 84],
        );
        assert_eq!(
            found,
            [0, 0, 0, 33076, 66152, 99228, 132_304, 165_380, 198_456]
        );
    }

    #[test]
    fn another_frame_rate() {
        let found = readings(&tagged(20, None, None), 0, 100, 40, &[8, 15, 40]);
        assert_eq!(found, [0, 33076, 132_304]);
    }

    #[test]
    fn pitch_of_a_streamed_sound() {
        let s44 = tagged(20, None, None);
        let after = |pitch, frames| readings(&s44, 0, pitch, 60, &[frames])[0];
        let found = [1, 50, 100, 150, 200].map(|pitch| after(pitch, 60));
        assert_eq!(found, [0, 33076, 132_304, 231_532, 297_684]);
        let found = [49, 50, 453].map(|pitch| after(pitch, 180));
        assert_eq!(found, [198_456, 231_532, 2_348_396]);
    }

    #[test]
    fn end_of_the_file() {
        let found = readings(&tagged(4, None, None), 0, 100, 60, &[75, 90]);
        assert_eq!(found, [165_380, 0]);
    }

    #[test]
    fn both_tags() {
        let tag20 = tagged(20, Some(88200), Some(176_400));
        assert_eq!(readings(&tag20, 600_000, 100, 60, &[0, 60]), [0, 26704]);
        assert_eq!(readings(&tag20, 0, 100, 60, &[191, 192]), [496_140, 0]);
    }

    #[test]
    fn only_the_start_tag() {
        let tag20s = tagged(20, Some(88200), None);
        assert_eq!(readings(&tag20s, 0, 100, 60, &[300]), [826_900]);
    }

    #[test]
    fn a_music_of_the_run_time_package() {
        let battle = SoundFormat {
            loop_start: Some(280_083),
            loop_length: Some(2_739_337),
            ..format(44100, 2, 3_020_850)
        };
        assert_eq!(at_once(&battle, 11_900_000), 909_576);
    }

    #[test]
    fn start_behind_the_end() {
        let found = readings(&tagged(20, None, None), 4_000_000, 100, 60, &[0, 24]);
        assert_eq!(found, [3_966_924, 4_033_076]);
    }

    #[test]
    fn just_above_the_limit() {
        let t301 = format(44100, 2, 132_741);
        assert_eq!(t301.frames * 4, 530_964);
        assert_eq!(at_once(&t301, 100_001), 0);
    }

    #[test]
    fn start_of_a_short_sound() {
        let short = tagged(3, None, None);
        assert_eq!(short.frames * 4, STREAM_LIMIT);
        assert_eq!(at_once(&short, 100_001), 100_000);
    }

    #[test]
    fn frames_of_a_short_sound() {
        let short = tagged(3, None, None);
        assert_eq!(readings(&short, 100_000, 100, 60, &[60]), [276_400]);
        assert_eq!(readings(&short, 0, 100, 60, &[90, 180]), [264_600, 0]);
    }

    #[test]
    fn pitch_and_frame_rate_of_a_short_sound() {
        let short = tagged(3, None, None);
        assert_eq!(readings(&short, 0, 50, 40, &[40]), [88200]);
    }

    #[test]
    fn no_sample_frame() {
        assert_eq!(
            readings(&format(44100, 2, 0), 100, 100, 60, &[0, 60]),
            [0, 0]
        );
    }
}
