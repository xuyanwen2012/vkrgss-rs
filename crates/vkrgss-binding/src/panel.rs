//! The message panel: the text of a modal message in a box over the scene, shown while the
//! guest waits for the dismissal.
//!
//! The panel is a bitmap of the guest's arena on a sprite of its scene, with no Ruby object
//! behind either, so no script can reach them. The sprite has no viewport and the highest z.
//! That alone does not put it in front of everything: the contents of a window are drawn
//! above the window's z. So the op that draws the panel is moved to the end of every frame
//! sent while it is shown ([`Panel::bring_to_front`]).

use vkrgss_core::{Align, Color, Font, Rect, SpriteId};
use vkrgss_proto::{BitmapId, Op};

use crate::guest::Guest;

/// The largest size of the panel, which is also limited by the screen.
const MAX_WIDTH: u32 = 560;
const MAX_HEIGHT: u32 = 400;
/// The distance of the text from the edge of the panel.
const PADDING: u32 = 16;
/// What ends the last line of a text that is cut.
const ELLIPSIS: &str = "...";

/// A panel on the screen.
pub(crate) struct Panel {
    sprite: SpriteId,
    bitmap: BitmapId,
}

/// Breaks `text` into lines no wider than `width` by `measure`, at its line ends and, where
/// a line is too wide, at its last space or else between two characters. At most `limit`
/// lines are returned; when the text has more, the last one ends with an ellipsis.
fn wrap(text: &str, width: u32, limit: usize, measure: impl Fn(&str) -> u32) -> Vec<String> {
    let mut lines = Vec::new();
    'text: for paragraph in text.split('\n') {
        let mut line = String::new();
        for character in paragraph.trim_end_matches('\r').chars() {
            line.push(character);
            if line.chars().nth(1).is_none() || measure(&line) <= width {
                continue;
            }
            let rest = match line.rfind(' ') {
                Some(space) if space > 0 => {
                    let rest = line[space + 1..].to_owned();
                    line.truncate(space);
                    rest
                }
                _ => {
                    line.pop();
                    character.to_string()
                }
            };
            lines.push(std::mem::replace(&mut line, rest));
            if lines.len() > limit {
                break 'text;
            }
        }
        lines.push(line);
        if lines.len() > limit {
            break;
        }
    }
    if lines.len() > limit {
        lines.truncate(limit);
        if let Some(last) = lines.last_mut() {
            while !last.is_empty() && measure(&format!("{last}{ELLIPSIS}")) > width {
                last.pop();
            }
            last.push_str(ELLIPSIS);
        }
    }
    lines
}

impl Panel {
    /// Puts a panel with `text` on the screen: the default font in white on a box of black
    /// at about 80% opacity with a light border, centred. `None` when the arena or the scene
    /// has no room for it; the message is then only on the error stream.
    pub(crate) fn show(guest: &mut Guest, text: &str) -> Option<Panel> {
        let screen = guest.scene.logical_size();
        let max_width = MAX_WIDTH.min(screen.width);
        let max_height = MAX_HEIGHT.min(screen.height);
        let mut font = Font::new(&guest.font_defaults);
        font.color = Color::rgb(255.0, 255.0, 255.0);

        let line_height = guest.text.metrics(&font, "").height.max(1);
        let limit = (max_height.saturating_sub(2 * PADDING) / line_height).max(1);
        let text_width = max_width.saturating_sub(2 * PADDING).max(1);
        // The width a line is drawn with, its last glyph's overhang included.
        let drawn = |line: &str| {
            let metrics = guest.text.metrics(&font, line);
            metrics.width().saturating_add(metrics.right_overhang)
        };
        let lines = wrap(text, text_width, limit as usize, drawn);
        let widest = lines.iter().map(|line| drawn(line)).max().unwrap_or(0);
        let width = (widest + 2 * PADDING).min(max_width);
        let height = (lines.len() as u32 * line_height + 2 * PADDING).min(max_height);

        let bitmap = guest
            .bitmaps
            .create(width, height, &guest.font_defaults)
            .ok()?;
        let Ok(sprite) = guest.scene.create_sprite(None) else {
            guest.bitmaps.dispose(bitmap);
            return None;
        };
        let panel = Panel { sprite, bitmap };
        if panel.draw(guest, font, &lines, line_height).is_none() {
            panel.dispose(guest);
            return None;
        }
        Some(panel)
    }

    fn draw(
        &self,
        guest: &mut Guest,
        font: Font,
        lines: &[String],
        line_height: u32,
    ) -> Option<()> {
        let size = guest.bitmaps.rect(self.bitmap).ok()?;
        let bitmaps = &mut guest.bitmaps;
        bitmaps
            .fill_rect(self.bitmap, size, Color::rgb(200.0, 200.0, 200.0))
            .ok()?;
        let inside = Rect::new(1, 1, size.width - 2, size.height - 2);
        bitmaps
            .fill_rect(self.bitmap, inside, Color::new(0.0, 0.0, 0.0, 204.0))
            .ok()?;
        bitmaps.set_font(self.bitmap, font).ok()?;
        let padding = PADDING as i32;
        let line_height = line_height as i32;
        for (index, line) in lines.iter().enumerate() {
            let rect = Rect::new(
                padding,
                padding + index as i32 * line_height,
                size.width - 2 * padding,
                line_height,
            );
            bitmaps
                .draw_text(self.bitmap, rect, line, Align::Center, &*guest.text)
                .ok()?;
        }

        let screen = guest.scene.logical_size();
        guest.scene.set_sprite_z(self.sprite, i32::MAX).ok()?;
        let sprite = guest.scene.sprite_mut(self.sprite).ok()?;
        sprite.x = (screen.width as i32 - size.width) / 2;
        sprite.y = (screen.height as i32 - size.height) / 2;
        sprite.set_bitmap(Some(self.bitmap), &guest.bitmaps);
        Some(())
    }

    /// Moves what draws the panel to the end of the ops of an extracted frame, after
    /// everything of the scene.
    pub(crate) fn bring_to_front(&self, ops: &mut Vec<Op>) {
        let draws_panel = |op: &Op| matches!(op, Op::Quad(quad) if quad.bitmap == self.bitmap);
        let (panel, mut scene): (Vec<Op>, Vec<Op>) = ops.drain(..).partition(|op| draws_panel(op));
        scene.extend(panel);
        *ops = scene;
    }

    /// Takes the panel off the screen.
    pub(crate) fn dispose(self, guest: &mut Guest) {
        guest.scene.dispose_sprite(self.sprite);
        guest.bitmaps.dispose(self.bitmap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character is one unit wide.
    fn count(line: &str) -> u32 {
        line.chars().count() as u32
    }

    #[test]
    fn short_lines_are_kept_and_line_ends_are_respected() {
        assert_eq!(wrap("one\r\n\ntwo", 10, 9, count), ["one", "", "two"]);
    }

    #[test]
    fn a_wide_line_breaks_at_its_last_space_or_between_characters() {
        assert_eq!(
            wrap("alpha beta gamma", 10, 9, count),
            ["alpha beta", "gamma"]
        );
        assert_eq!(wrap("abcdefghijkl", 5, 9, count), ["abcde", "fghij", "kl"]);
        assert_eq!(wrap("日本語の文章", 4, 9, count), ["日本語の", "文章"]);
    }

    #[test]
    fn lines_beyond_the_limit_are_cut_with_an_ellipsis() {
        assert_eq!(wrap("a\nb\nc\nd", 10, 2, count), ["a", "b..."]);
        assert_eq!(wrap("abcdefghijkl", 5, 2, count), ["abcde", "fg..."]);
        // Exactly the limit is not cut.
        assert_eq!(wrap("a\nb", 10, 2, count), ["a", "b"]);
    }
}
