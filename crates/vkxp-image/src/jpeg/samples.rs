//! From component planes to pixels: the triangle chroma upsampling of IJG libjpeg 6b
//! (`jdsample.c`) and its colour conversion (`jdcolor.c`).

/// The samples of one component: the block grid of the component, 8 samples per block.
pub(super) struct Plane {
    pub(super) samples: Vec<u8>,
    pub(super) stride: usize,
    /// The size the component has in the image, which is what upsampling works on.
    pub(super) width: usize,
    pub(super) height: usize,
}

impl Plane {
    fn row(&self, y: usize) -> &[u8] {
        &self.samples[y * self.stride..][..self.width]
    }

    /// Writes row `y` of the image, at twice the width of the plane, into `out`. `vertical`
    /// says that the plane also has half the height of the image.
    ///
    /// At the edges the nearest row or column stands in for the missing one, as in libjpeg.
    fn upsampled_row(&self, y: usize, vertical: bool, out: &mut [u8]) {
        let (near, far) = if vertical {
            let near = y / 2;
            let far = if y.is_multiple_of(2) {
                near.saturating_sub(1)
            } else {
                (near + 1).min(self.height - 1)
            };
            (self.row(near), self.row(far))
        } else {
            (self.row(y), self.row(y))
        };
        let last = self.width - 1;
        // libjpeg replicates samples instead when a component is at most two samples wide.
        if self.width <= 2 {
            for (pair, &sample) in out.as_chunks_mut::<2>().0.iter_mut().zip(near) {
                *pair = [sample; 2];
            }
        } else if vertical {
            // Three quarters of the nearer row and column, one quarter of the farther one.
            let sum = |x: usize| i32::from(near[x]) * 3 + i32::from(far[x]);
            for x in 0..=last {
                let left = if x == 0 { sum(0) } else { sum(x - 1) };
                let right = if x == last { sum(last) } else { sum(x + 1) };
                out[2 * x] = ((sum(x) * 3 + left + 8) >> 4) as u8;
                out[2 * x + 1] = ((sum(x) * 3 + right + 7) >> 4) as u8;
            }
        } else {
            let at = |x: usize| i32::from(near[x]);
            out[0] = near[0];
            out[1] = ((at(0) * 3 + at(1) + 2) >> 2) as u8;
            for x in 1..last {
                out[2 * x] = ((at(x) * 3 + at(x - 1) + 1) >> 2) as u8;
                out[2 * x + 1] = ((at(x) * 3 + at(x + 1) + 2) >> 2) as u8;
            }
            out[2 * last] = ((at(last) * 3 + at(last - 1) + 1) >> 2) as u8;
            out[2 * last + 1] = near[last];
        }
    }
}

/// `FIX` of `jdcolor.c`: a factor with 16 fractional bits.
const fn fix(factor: f64) -> i32 {
    (factor * 65536.0 + 0.5) as i32
}

fn clamp(value: i32) -> u8 {
    value.clamp(0, 255) as u8
}

/// Red, green and blue as the tables of `jdcolor.c` give them.
fn rgb(y: u8, cb: u8, cr: u8) -> [u8; 3] {
    let (y, cb, cr) = (i32::from(y), i32::from(cb) - 128, i32::from(cr) - 128);
    [
        clamp(y + ((fix(1.40200) * cr + 32768) >> 16)),
        clamp(y + ((-fix(0.34414) * cb + 32768 - fix(0.71414) * cr) >> 16)),
        clamp(y + ((fix(1.77200) * cb + 32768) >> 16)),
    ]
}

/// Opaque pixels of a greyscale image.
pub(super) fn grey(plane: &Plane, pixels: &mut [[u8; 4]]) {
    for (y, row) in pixels.chunks_exact_mut(plane.width).enumerate() {
        for (pixel, &sample) in row.iter_mut().zip(plane.row(y)) {
            *pixel = [sample, sample, sample, 255];
        }
    }
}

/// Opaque pixels of a Y, Cb, Cr image of `width` pixels a row, whose chroma is halved
/// horizontally if `horizontal` and also vertically if `vertical`.
pub(super) fn colour(
    [luma, cb, cr]: &[Plane; 3],
    (horizontal, vertical): (bool, bool),
    pixels: &mut [[u8; 4]],
) {
    let mut rows = [vec![0; cb.width * 2], vec![0; cr.width * 2]];
    for (y, row) in pixels.chunks_exact_mut(luma.width).enumerate() {
        let [cb, cr] = if horizontal {
            cb.upsampled_row(y, vertical, &mut rows[0]);
            cr.upsampled_row(y, vertical, &mut rows[1]);
            [&rows[0][..], &rows[1][..]]
        } else {
            [cb.row(y), cr.row(y)]
        };
        for (((pixel, &y), &cb), &cr) in row.iter_mut().zip(luma.row(y)).zip(cb).zip(cr) {
            let [red, green, blue] = rgb(y, cb, cr);
            *pixel = [red, green, blue, 255];
        }
    }
}
