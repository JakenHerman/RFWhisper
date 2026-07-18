//! Shared test helpers: deterministic RNG (xorshift + Box-Muller) so tests don't
//! need an external `rand` dependency, mirroring the seeded NumPy generators in
//! the original Python tests.
#![allow(dead_code)] // each integration-test binary uses a different subset

pub struct TestRng {
    state: u64,
    spare_normal: Option<f64>,
}

impl TestRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1),
            spare_normal: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// Uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in [lo, hi).
    pub fn uniform_in(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.uniform()
    }

    /// Standard normal via Box-Muller.
    pub fn standard_normal(&mut self) -> f64 {
        if let Some(v) = self.spare_normal.take() {
            return v;
        }
        let (u1, u2) = (self.uniform().max(1e-300), self.uniform());
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        self.spare_normal = Some(r * theta.sin());
        r * theta.cos()
    }

    pub fn standard_normal_vec(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.standard_normal()).collect()
    }
}

pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[allow(dead_code)]
pub fn assert_close(got: f64, want: f64, atol: f64, msg: &str) {
    assert!(
        (got - want).abs() <= atol,
        "{msg}: got {got}, want {want} (atol {atol})"
    );
}
