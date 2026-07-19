//! Seeded PRNG for fixture synthesis — deterministic across machines and runs.
//!
//! This is a fixture generator, not a source of cryptographic randomness, and it
//! is deliberately not `rand`: a hard dependency would have to be reproduced
//! bit-for-bit by every contributor's toolchain, and pinning the crate version is
//! a weaker guarantee than owning ~40 lines of PCG. Same seed in, same samples
//! out, forever — that is what lets the acceptance gates assert hard dB
//! thresholds and know a failure means the *denoiser* moved, not the fixture.
//!
//! The stream does not match NumPy's `default_rng(seed)`, so fixtures generated
//! here differ sample-for-sample from the pre-Rust-port Python originals.

/// PCG-XSH-RR 64/32 — small, fast, and stable by specification.
pub struct SeededRng {
    state: u64,
    inc: u64,
    spare_normal: Option<f64>,
}

const PCG_MULT: u64 = 6_364_136_223_846_793_005;

impl SeededRng {
    /// Seed the generator. Every seed yields a distinct, reproducible stream.
    pub fn new(seed: u64) -> Self {
        // Fixed stream selector: one seed, one sequence (odd increment per PCG).
        let mut rng = Self {
            state: 0,
            inc: 0xda3e_39cb_94b9_5bdb,
            spare_normal: None,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(PCG_MULT).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in `[0, 1)` with 53 bits of mantissa.
    pub fn uniform(&mut self) -> f64 {
        let hi = u64::from(self.next_u32());
        let lo = u64::from(self.next_u32());
        (((hi << 32) | lo) >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[lo, hi)`.
    pub fn uniform_in(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.uniform()
    }

    /// Standard normal via Box-Muller (the spare deviate is cached, not discarded).
    pub fn standard_normal(&mut self) -> f64 {
        if let Some(v) = self.spare_normal.take() {
            return v;
        }
        let u1 = self.uniform().max(f64::MIN_POSITIVE);
        let u2 = self.uniform();
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        self.spare_normal = Some(r * theta.sin());
        r * theta.cos()
    }

    /// `n` standard-normal deviates.
    pub fn standard_normal_vec(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.standard_normal()).collect()
    }

    /// Exponential deviate with the given mean (inverse-CDF).
    pub fn exponential(&mut self, mean: f64) -> f64 {
        -mean * (1.0 - self.uniform()).ln()
    }
}
