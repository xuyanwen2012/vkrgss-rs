//! The drawing of the app's own pictures: the on-screen buttons and the library screen, drawn
//! on the CPU into a picture the presenter puts over the frame ([`OverlayImage`]).
//!
//! The shapes are a fixed set: rounded rectangles, triangles and text, each filled with one
//! colour and smoothed at its edges by the share of each pixel it covers. Text is set with
//! the runtime's font stack ([`TextEngine`]) in the fonts of the system.

use std::path::PathBuf;

use vkrgss_proto::Size;
use vkrgss_text::{FontSources, SystemFonts, TextEngine, TextRequest};
use vkrgss_vk::OverlayImage;

/// A colour, not premultiplied: red, green, blue and alpha.
pub type Color = [u8; 4];

/// A rectangle in pixels, by its left, top, width and height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Area {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Area {
        Area {
            x,
            y,
            width,
            height,
        }
    }

    /// The rectangle of `width` by `height` centred on `(x, y)`.
    pub fn centred(x: f32, y: f32, width: f32, height: f32) -> Area {
        Area::new(x - width / 2.0, y - height / 2.0, width, height)
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    pub fn centre(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

/// What covers the edges of a window, in pixels: the system's bars and the display's cutout.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Insets {
    /// The area of a window of `window` pixels that nothing covers.
    pub fn inside(&self, window: Size) -> Area {
        Area::new(
            self.left,
            self.top,
            (window.width as f32 - self.left - self.right).max(0.0),
            (window.height as f32 - self.top - self.bottom).max(0.0),
        )
    }
}

/// A picture being drawn: premultiplied RGBA8 pixels, transparent where nothing is drawn.
pub struct Canvas {
    size: Size,
    pixels: Vec<u8>,
}

/// The whole pixels from `start` to `end` that a shape between them can touch, within `0..limit`.
fn span(start: f32, end: f32, limit: u32) -> std::ops::Range<u32> {
    let first = start.floor().max(0.0) as u32;
    let last = (end.ceil().max(0.0) as u32).min(limit);
    first.min(last)..last
}

impl Canvas {
    pub fn new(size: Size) -> Canvas {
        Canvas {
            size,
            pixels: vec![0; size.width as usize * size.height as usize * 4],
        }
    }

    pub fn into_image(self) -> OverlayImage {
        OverlayImage {
            size: self.size,
            pixels: self.pixels,
        }
    }

    /// Puts `color` over the pixel `(x, y)`, which `coverage` (0 to 1) of it covers.
    fn blend(&mut self, x: u32, y: u32, color: Color, coverage: f32) {
        let alpha = f32::from(color[3]) / 255.0 * coverage.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let at = (y as usize * self.size.width as usize + x as usize) * 4;
        let pixel = &mut self.pixels[at..at + 4];
        for channel in 0..3 {
            let source = f32::from(color[channel]) * alpha;
            pixel[channel] = (source + f32::from(pixel[channel]) * (1.0 - alpha)).round() as u8;
        }
        pixel[3] = (alpha * 255.0 + f32::from(pixel[3]) * (1.0 - alpha)).round() as u8;
    }

    /// Fills `area` with corners rounded by `radius`.
    pub fn rounded(&mut self, area: Area, radius: f32, color: Color) {
        let radius = radius.min(area.width / 2.0).min(area.height / 2.0).max(0.0);
        let (cx, cy) = area.centre();
        let (half_width, half_height) = (area.width / 2.0 - radius, area.height / 2.0 - radius);
        for y in span(area.y, area.y + area.height, self.size.height) {
            for x in span(area.x, area.x + area.width, self.size.width) {
                // The distance of the pixel's centre from the shape, negative inside.
                let dx = ((x as f32 + 0.5 - cx).abs() - half_width).max(0.0);
                let dy = ((y as f32 + 0.5 - cy).abs() - half_height).max(0.0);
                let outside = (dx * dx + dy * dy).sqrt() - radius;
                // Within the straight edges the distance is that to the edge.
                let edge = ((x as f32 + 0.5 - cx).abs() - area.width / 2.0)
                    .max((y as f32 + 0.5 - cy).abs() - area.height / 2.0);
                let distance = if dx > 0.0 && dy > 0.0 { outside } else { edge };
                self.blend(x, y, color, 0.5 - distance);
            }
        }
    }

    /// Fills the triangle with the corners `points`.
    pub fn triangle(&mut self, points: [(f32, f32); 3], color: Color) {
        let xs = points.map(|(x, _)| x);
        let ys = points.map(|(_, y)| y);
        let (left, right) = (
            xs.iter().copied().fold(f32::MAX, f32::min),
            xs.iter().copied().fold(f32::MIN, f32::max),
        );
        let (top, bottom) = (
            ys.iter().copied().fold(f32::MAX, f32::min),
            ys.iter().copied().fold(f32::MIN, f32::max),
        );
        // The edges in the order that keeps the inside on their left.
        let [a, b, c] = points;
        let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
        let edges = match area >= 0.0 {
            true => [(a, b), (b, c), (c, a)],
            false => [(a, c), (c, b), (b, a)],
        };
        for y in span(top, bottom, self.size.height) {
            for x in span(left, right, self.size.width) {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let inside = edges
                    .iter()
                    .map(|&((x0, y0), (x1, y1))| {
                        let length = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt().max(1e-6);
                        ((x1 - x0) * (py - y0) - (y1 - y0) * (px - x0)) / length
                    })
                    .fold(f32::MAX, f32::min);
                self.blend(x, y, color, 0.5 + inside);
            }
        }
    }

    /// Draws a coverage mask of `width` by `height` with its top left corner at `(x, y)`.
    pub fn mask(&mut self, x: i32, y: i32, width: u32, height: u32, coverage: &[u8], color: Color) {
        for row in 0..height {
            for column in 0..width {
                let (px, py) = (x + column as i32, y + row as i32);
                if px < 0 || py < 0 || px as u32 >= self.size.width || py as u32 >= self.size.height
                {
                    continue;
                }
                let value = coverage[(row * width + column) as usize];
                if value > 0 {
                    self.blend(px as u32, py as u32, color, f32::from(value) / 255.0);
                }
            }
        }
    }
}

/// Text in the fonts of the system, set by the runtime's font stack.
pub struct Text {
    engine: TextEngine,
}

/// The families text asks for: a Latin face and the CJK face of the system, which the font
/// stack also falls back to for the characters a face lacks.
const FAMILIES: [&str; 3] = ["Roboto", "Noto Sans CJK SC", "Noto Sans CJK JP"];

impl Text {
    /// Text in the fonts of `dirs`, which stand in for the system's.
    pub fn new(dirs: Vec<PathBuf>) -> Text {
        Text {
            engine: TextEngine::new(&FontSources {
                system: SystemFonts::Directories(dirs),
                ..FontSources::default()
            }),
        }
    }

    /// Draws `text` in a cell `size` pixels high, centred on `(x, y)` or, with `left`, starting
    /// at `x` and centred on `y` vertically, no wider than `limit`. Returns how wide it is.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        canvas: &mut Canvas,
        text: &str,
        size: u32,
        (x, y): (f32, f32),
        left: bool,
        limit: f32,
        color: Color,
    ) -> u32 {
        let request = TextRequest::new(FAMILIES, size.max(1));
        let mask = self.engine.rasterize(&request, text);
        let width = mask.width.min(limit.max(0.0) as u32);
        let start = match left {
            true => x,
            false => x - width as f32 / 2.0,
        };
        let top = y - mask.height as f32 / 2.0;
        // A mask cut to the limit keeps its rows whole.
        let mut coverage = Vec::with_capacity((width * mask.height) as usize);
        for row in 0..mask.height {
            let at = (row * mask.width) as usize;
            coverage.extend_from_slice(&mask.coverage[at..at + width as usize]);
        }
        canvas.mask(
            start.round() as i32,
            top.round() as i32,
            width,
            mask.height,
            &coverage,
            color,
        );
        width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(image: &OverlayImage, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * image.size.width + x) * 4) as usize;
        image.pixels[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn shapes_cover_their_inside_and_leave_the_rest_clear() {
        let mut canvas = Canvas::new(Size::new(40, 30));
        canvas.rounded(Area::new(10.0, 5.0, 20.0, 20.0), 6.0, [255, 0, 0, 128]);
        canvas.triangle([(0.0, 0.0), (8.0, 0.0), (0.0, 8.0)], [0, 0, 255, 255]);
        let image = canvas.into_image();
        assert_eq!(image.pixels.len(), 40 * 30 * 4);
        // Premultiplied: half of red at half alpha.
        assert_eq!(pixel(&image, 20, 15), [128, 0, 0, 128]);
        assert_eq!(pixel(&image, 1, 1), [0, 0, 255, 255]);
        // The rounded corner leaves the corner pixel clear, and the outside is clear.
        assert_eq!(pixel(&image, 10, 5), [0; 4]);
        assert_eq!(pixel(&image, 35, 15), [0; 4]);
        assert_eq!(pixel(&image, 7, 7), [0; 4]);
        // An edge pixel is partly covered.
        let edge = pixel(&image, 20, 5);
        assert!(edge[3] > 0 && edge[3] <= 128, "{edge:?}");
    }

    #[test]
    fn shapes_past_the_edges_are_cut() {
        let mut canvas = Canvas::new(Size::new(8, 8));
        canvas.rounded(Area::new(-20.0, -20.0, 100.0, 100.0), 3.0, [1, 2, 3, 255]);
        canvas.mask(-2, 6, 4, 4, &[255; 16], [0, 0, 0, 255]);
        let image = canvas.into_image();
        assert_eq!(pixel(&image, 1, 7), [0, 0, 0, 255]);
        assert_eq!(pixel(&image, 2, 7), [1, 2, 3, 255]);
        assert_eq!(pixel(&image, 0, 0), [1, 2, 3, 255]);
    }
}
