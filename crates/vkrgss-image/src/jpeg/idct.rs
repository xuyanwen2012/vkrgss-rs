//! The floating-point inverse DCT of IJG libjpeg 6b (`jidctflt.c`) as the original runtime
//! runs it: on the x87 with 53-bit precision control. Every value stored in a variable or in
//! the workspace is rounded to `f32`, every expression is evaluated in `f64`. That is written
//! out here with [`store`], so nothing depends on how the compiler evaluates `f32` arithmetic.

/// The scale factors of the AAN algorithm that are folded into the quantisation table.
const AAN: [f64; 8] = [
    1.0,
    1.387039845,
    1.306562965,
    1.175875602,
    1.0,
    0.785694958,
    0.541196100,
    0.275899379,
];

// The constants as the source writes them, which the code holds as `float`.
#[allow(clippy::approx_constant)]
const C_1_414: f64 = 1.414213562_f64 as f32 as f64;
const C_1_847: f64 = 1.847759065_f64 as f32 as f64;
const C_1_082: f64 = 1.082392200_f64 as f32 as f64;
const C_2_613: f64 = 2.613125930_f64 as f32 as f64;

/// A store to a `float` variable.
fn store(value: f64) -> f64 {
    f64::from(value as f32)
}

/// The multiplier of each coefficient, in row order, for a quantisation table in row order.
pub(super) fn multipliers(quant: &[u16; 64]) -> [f64; 64] {
    std::array::from_fn(|index| store(f64::from(quant[index]) * AAN[index / 8] * AAN[index % 8]))
}

/// One pass over eight values, up to the final butterflies.
fn pass(input: [f64; 8]) -> [f64; 8] {
    let [in0, in1, in2, in3, in4, in5, in6, in7] = input;

    // Even part.
    let tmp10 = store(in0 + in4);
    let tmp11 = store(in0 - in4);
    let tmp13 = store(in2 + in6);
    let tmp12 = store((in2 - in6) * C_1_414 - tmp13);
    let tmp0 = store(tmp10 + tmp13);
    let tmp3 = store(tmp10 - tmp13);
    let tmp1 = store(tmp11 + tmp12);
    let tmp2 = store(tmp11 - tmp12);

    // Odd part.
    let z13 = store(in5 + in3);
    let z10 = store(in5 - in3);
    let z11 = store(in1 + in7);
    let z12 = store(in1 - in7);
    let tmp7 = store(z11 + z13);
    let tmp11 = store((z11 - z13) * C_1_414);
    let z5 = store((z10 + z12) * C_1_847);
    let tmp10 = store(C_1_082 * z12 - z5);
    let tmp12 = store(z5 - C_2_613 * z10);
    let tmp6 = store(tmp12 - tmp7);
    let tmp5 = store(tmp11 - tmp6);
    let tmp4 = store(tmp10 + tmp5);

    [tmp0, tmp1, tmp2, tmp3, tmp4, tmp5, tmp6, tmp7]
}

/// The sample of a row-pass sum: truncated toward zero, then the rounding shift by three bits
/// and the range limit table of libjpeg, which wraps at 1024.
fn sample(sum: f64) -> u8 {
    let index = ((sum as i32).wrapping_add(4) >> 3) & 0x3FF;
    match index {
        0..128 => (index + 128) as u8,
        128..512 => 255,
        512..896 => 0,
        _ => (index - 896) as u8,
    }
}

/// Transforms one block of coefficients in row order into 8 rows of samples, which are
/// written at `out[row * stride..]`.
pub(super) fn inverse(
    coefficients: &[i16; 64],
    multipliers: &[f64; 64],
    out: &mut [u8],
    stride: usize,
) {
    let mut workspace = [0.0; 64];
    for column in 0..8 {
        let at = |row: usize| row * 8 + column;
        // A column without AC terms takes a shortcut. Rows take none.
        if (1..8).all(|row| coefficients[at(row)] == 0) {
            let dc = store(f64::from(coefficients[column]) * multipliers[column]);
            for row in 0..8 {
                workspace[at(row)] = dc;
            }
            continue;
        }
        let [t0, t1, t2, t3, t4, t5, t6, t7] = pass(std::array::from_fn(|row| {
            store(f64::from(coefficients[at(row)]) * multipliers[at(row)])
        }));
        workspace[at(0)] = store(t0 + t7);
        workspace[at(7)] = store(t0 - t7);
        workspace[at(1)] = store(t1 + t6);
        workspace[at(6)] = store(t1 - t6);
        workspace[at(2)] = store(t2 + t5);
        workspace[at(5)] = store(t2 - t5);
        workspace[at(4)] = store(t3 + t4);
        workspace[at(3)] = store(t3 - t4);
    }
    for (row, values) in workspace.as_chunks::<8>().0.iter().enumerate() {
        let [t0, t1, t2, t3, t4, t5, t6, t7] = pass(*values);
        let sums = [
            t0 + t7,
            t1 + t6,
            t2 + t5,
            t3 - t4,
            t3 + t4,
            t2 - t5,
            t1 - t6,
            t0 - t7,
        ];
        for (out, sum) in out[row * stride..][..8].iter_mut().zip(sums) {
            *out = sample(sum);
        }
    }
}
