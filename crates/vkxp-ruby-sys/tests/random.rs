//! `srand` and `rand` produce the sequence of Ruby 1.8.1, which the original runtime embeds.
//!
//! The expected values come from the MT19937 below, not from the interpreter's own generator.

mod common;

use std::sync::OnceLock;

use common::eval_inspect;
use vkxp_ruby_sys::{Interpreter, Options};

fn eval(source: &'static str) -> Result<String, String> {
    static INTERPRETER: OnceLock<Interpreter> = OnceLock::new();
    INTERPRETER
        .get_or_init(|| Interpreter::start(Options::default()).expect("the interpreter starts"))
        .run(move |_| eval_inspect(source))
}

/// The reference MT19937 of Matsumoto and Nishimura, initialised by `init_genrand`.
struct Mt19937 {
    state: [u32; 624],
    index: usize,
}

impl Mt19937 {
    fn new(seed: u32) -> Self {
        let mut state = [0_u32; 624];
        state[0] = seed;
        for i in 1..624 {
            let previous = state[i - 1];
            state[i] = 1_812_433_253_u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(i as u32);
        }
        Self { state, index: 624 }
    }

    fn next_u32(&mut self) -> u32 {
        if self.index == 624 {
            for i in 0..624 {
                let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
                let twist = if y & 1 == 1 { 0x9908_b0df } else { 0 };
                self.state[i] = self.state[(i + 397) % 624] ^ (y >> 1) ^ twist;
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// A real in [0, 1): one output divided by 2^32.
    fn next_real(&mut self) -> f64 {
        f64::from(self.next_u32()) * (1.0 / 4_294_967_296.0)
    }
}

#[test]
fn reference_generator_matches_the_published_first_output() {
    assert_eq!(Mt19937::new(5489).next_u32(), 3_499_211_612);
}

#[test]
fn integers_are_the_real_scaled_and_truncated() {
    let mut reference = Mt19937::new(5489);
    let expected: Vec<i64> = (0..3)
        .map(|_| (1000.0 * reference.next_real()) as i64)
        .collect();
    assert_eq!(
        eval("srand(5489); [rand(1000), rand(1000), rand(1000)]"),
        Ok(format!("{expected:?}"))
    );
}

#[test]
fn negative_limit_counts_as_its_absolute_value() {
    assert_eq!(
        eval("srand(5489); a = rand(-1000); srand(5489); a == rand(1000)"),
        Ok("true".to_owned())
    );
}

/// Measured in the original runtime.
#[test]
fn integers_match_the_original() {
    assert_eq!(
        eval("srand(1234); (1..12).map { rand(1000) }"),
        Ok("[191, 497, 622, 817, 437, 612, 785, 771, 779, 860, 272, 150]".to_owned())
    );
}

/// Measured in the original runtime: the thirteenth output divided by 2^32.
#[test]
fn float_is_the_real_itself() {
    assert_eq!(
        eval("srand(1234); 12.times { rand(1000) }; '%.15f' % rand"),
        Ok("\"0.276464259717613\"".to_owned())
    );
    assert_eq!(
        eval("srand(1234); 12.times { rand(1000) }; '%.15f' % rand(0)"),
        Ok("\"0.276464259717613\"".to_owned())
    );
}

#[test]
fn one_output_is_consumed_per_number() {
    let mut reference = Mt19937::new(1234);
    let expected: Vec<String> = (0..4)
        .map(|_| format!("{:.15}", reference.next_real()))
        .collect();
    assert_eq!(
        eval("srand(1234); (1..4).map { '%.15f' % rand }"),
        Ok(format!("{expected:?}"))
    );
}

#[test]
fn seed_is_taken_by_its_low_32_bits() {
    assert_eq!(
        eval(
            "srand(1); a = rand; srand((1 << 32) + 1); b = rand; srand((1 << 70) + 1); a == b && b == rand"
        ),
        Ok("true".to_owned())
    );
}

#[test]
fn srand_returns_the_previous_seed() {
    assert_eq!(eval("srand(3); srand(4)"), Ok("3".to_owned()));
}
