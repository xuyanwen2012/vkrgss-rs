//! Rendered glyphs, kept up to a byte limit.

use std::collections::HashMap;
use std::sync::Arc;

use swash::GlyphId;

/// Bookkeeping bytes charged per cached glyph on top of its coverage.
const ENTRY_OVERHEAD: usize = 64;

/// A glyph's coverage with its position relative to the pen.
#[derive(Debug, Default)]
pub(crate) struct Glyph {
    /// Distance from the pen position to the left edge of the image.
    pub(crate) left: i32,
    /// Distance from the baseline up to the top edge of the image.
    pub(crate) top: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// `width * height` coverage bytes, rows from the top.
    pub(crate) coverage: Vec<u8>,
}

impl Glyph {
    /// How far the image reaches left of the pen.
    pub(crate) fn left_overhang(&self) -> u32 {
        if self.width == 0 {
            return 0;
        }
        self.left.min(0).unsigned_abs()
    }

    /// How far the image reaches right of the pen moved on by `advance`.
    pub(crate) fn right_overhang(&self, advance: u32) -> u32 {
        if self.width == 0 {
            return 0;
        }
        let right = i64::from(self.left) + i64::from(self.width) - i64::from(advance);
        right.clamp(0, i64::from(u32::MAX)) as u32
    }
}

/// Identifies a rendered glyph: the loaded face, its pixels per em, the glyph, the styles
/// synthesised for it, and the fraction of its natural width it is condensed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GlyphKey {
    pub(crate) face: usize,
    pub(crate) pixels_per_em: u32,
    pub(crate) glyph: GlyphId,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) condense: (u32, u32),
}

struct Entry {
    glyph: Arc<Glyph>,
    last_used: u64,
}

/// Glyph images by key. When their total size passes the limit, the least recently used ones
/// are dropped.
pub(crate) struct GlyphCache {
    entries: HashMap<GlyphKey, Entry>,
    bytes: usize,
    limit: usize,
    clock: u64,
}

fn cost(glyph: &Glyph) -> usize {
    glyph.coverage.len() + ENTRY_OVERHEAD
}

impl GlyphCache {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            entries: HashMap::new(),
            bytes: 0,
            limit,
            clock: 0,
        }
    }

    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }

    pub(crate) fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        self.evict(limit, None);
    }

    pub(crate) fn get(&mut self, key: GlyphKey) -> Option<Arc<Glyph>> {
        self.clock += 1;
        let entry = self.entries.get_mut(&key)?;
        entry.last_used = self.clock;
        Some(Arc::clone(&entry.glyph))
    }

    /// Stores `glyph`, dropping older glyphs to stay within the limit. A glyph larger than
    /// the whole limit is not stored.
    pub(crate) fn insert(&mut self, key: GlyphKey, glyph: Arc<Glyph>) {
        let cost = cost(&glyph);
        if cost > self.limit {
            return;
        }
        self.clock += 1;
        let entry = Entry {
            glyph,
            last_used: self.clock,
        };
        if let Some(old) = self.entries.insert(key, entry) {
            self.bytes -= self::cost(&old.glyph);
        }
        self.bytes += cost;
        if self.bytes > self.limit {
            // Free a quarter of the limit at once, so that a full cache does not sort its
            // entries on every insertion.
            let target = (self.limit - self.limit / 4).max(cost);
            self.evict(target, Some(key));
        }
    }

    /// Drops least recently used glyphs, never `keep`, until at most `target` bytes remain.
    fn evict(&mut self, target: usize, keep: Option<GlyphKey>) {
        if self.bytes <= target {
            return;
        }
        let mut by_age: Vec<(u64, GlyphKey)> = self
            .entries
            .iter()
            .filter(|(key, _)| Some(**key) != keep)
            .map(|(key, entry)| (entry.last_used, *key))
            .collect();
        by_age.sort_unstable_by_key(|&(last_used, _)| last_used);
        for (_, key) in by_age {
            if self.bytes <= target {
                break;
            }
            if let Some(entry) = self.entries.remove(&key) {
                self.bytes -= cost(&entry.glyph);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glyph(bytes: usize) -> Arc<Glyph> {
        Arc::new(Glyph {
            width: bytes as u32,
            height: 1,
            coverage: vec![0; bytes],
            ..Glyph::default()
        })
    }

    fn key(glyph: GlyphId) -> GlyphKey {
        GlyphKey {
            face: 0,
            pixels_per_em: 0,
            glyph,
            bold: false,
            italic: false,
            condense: (1, 1),
        }
    }

    #[test]
    fn stays_within_the_limit_and_drops_the_least_recently_used() {
        let each = 100 + ENTRY_OVERHEAD;
        let mut cache = GlyphCache::new(4 * each);
        for id in 0..4 {
            cache.insert(key(id), glyph(100));
        }
        assert_eq!(cache.bytes(), 4 * each);
        // Touch the oldest, so that glyph 1 becomes the least recently used.
        assert!(cache.get(key(0)).is_some());
        cache.insert(key(4), glyph(100));
        assert!(cache.bytes() <= 4 * each);
        assert!(cache.get(key(1)).is_none());
        assert!(cache.get(key(0)).is_some());
        assert!(cache.get(key(4)).is_some());
    }

    #[test]
    fn does_not_store_a_glyph_larger_than_the_limit() {
        let mut cache = GlyphCache::new(128);
        cache.insert(key(0), glyph(1000));
        assert_eq!(cache.bytes(), 0);
        assert!(cache.get(key(0)).is_none());
    }

    #[test]
    fn lowering_the_limit_evicts() {
        let mut cache = GlyphCache::new(10_000);
        for id in 0..10 {
            cache.insert(key(id), glyph(100));
        }
        cache.set_limit(500);
        assert!(cache.bytes() <= 500);
        assert!(cache.get(key(9)).is_some());
    }
}
