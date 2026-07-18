//! Deterministic synthetic test-signal generators (refs #19).
//!
//! Every generator is seeded and reproducible so PR testing criteria can say
//! "generate X, run Y, expect Z" without shipping audio fixtures. These are
//! *test* signals: shaped to exercise the ham-relevant failure modes (keying
//! transients, powerline buzz, static crashes), not to sound pretty.

use std::path::Path;

/// xorshift64* — deterministic, dependency-free RNG for reproducible fixtures.
pub struct SampleRng {
    state: u64,
    spare_normal: Option<f32>,
}

impl SampleRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1),
            spare_normal: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
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

    /// Standard normal via Box-Muller.
    pub fn standard_normal(&mut self) -> f32 {
        if let Some(v) = self.spare_normal.take() {
            return v;
        }
        let (u1, u2) = (self.uniform().max(1e-300), self.uniform());
        let r = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        self.spare_normal = Some((r * theta.sin()) as f32);
        (r * theta.cos()) as f32
    }
}

const TWO_PI: f32 = 2.0 * std::f32::consts::PI;

/// Band-limited, amplitude-modulated tone stack — stands in for ham SSB speech.
/// Formant-ish carriers with a 3 Hz syllabic envelope.
pub fn speech_like(n: usize, sr: u32, seed: u64) -> Vec<f32> {
    let mut rng = SampleRng::new(seed);
    let mut x = vec![0.0f32; n];
    for f0 in [220.0f32, 480.0, 910.0, 1_600.0] {
        let amp = 0.5 + 0.5 * rng.uniform() as f32;
        let phase = TWO_PI * rng.uniform() as f32;
        for (i, v) in x.iter_mut().enumerate() {
            let t = i as f32 / sr as f32;
            *v += amp * (TWO_PI * f0 * t + phase).sin();
        }
    }
    let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    for (i, v) in x.iter_mut().enumerate() {
        let t = i as f32 / sr as f32;
        let envelope = 0.6 + 0.4 * (TWO_PI * 3.0 * t).sin();
        *v *= 0.5 * envelope / peak;
    }
    x
}

/// Keyed CW at `wpm` on a 600 Hz tone with 5 ms raised-cosine edges (A3's
/// "keying transients are sacred" fixture). Repeats "CQ" indefinitely.
pub fn cw(n: usize, sr: u32, wpm: u32) -> Vec<f32> {
    // PARIS timing: dit = 1.2 / wpm seconds.
    let dit_s = 1.2 / wpm.max(1) as f32;
    let dit = (dit_s * sr as f32) as usize;
    // C Q = "-.-. --.-" with 1-dit intra-element, 3-dit inter-letter, 7-dit word gap.
    let pattern: &[(usize, bool)] = &[
        (3, true),
        (1, false),
        (1, true),
        (1, false),
        (3, true),
        (1, false),
        (1, true), // C
        (3, false),
        (3, true),
        (1, false),
        (3, true),
        (1, false),
        (1, true),
        (1, false),
        (3, true), // Q
        (7, false),
    ];
    let edge = (0.005 * sr as f32) as usize; // 5 ms
    let mut x = vec![0.0f32; n];
    let mut i = 0usize;
    'outer: loop {
        for &(units, key_down) in pattern {
            let len = units * dit;
            if key_down {
                for j in 0..len {
                    if i + j >= n {
                        break 'outer;
                    }
                    let t = (i + j) as f32 / sr as f32;
                    // Raised-cosine keying envelope at both edges.
                    let env = if j < edge {
                        0.5 * (1.0 - (std::f32::consts::PI * j as f32 / edge as f32).cos())
                    } else if j + edge > len {
                        0.5 * (1.0 - (std::f32::consts::PI * (len - j) as f32 / edge as f32).cos())
                    } else {
                        1.0
                    };
                    x[i + j] = 0.5 * env * (TWO_PI * 600.0 * t).sin();
                }
            }
            i += len;
            if i >= n {
                break 'outer;
            }
        }
    }
    x
}

/// Gaussian white noise at unit-ish level (scaled to RMS 0.2).
pub fn white(n: usize, seed: u64) -> Vec<f32> {
    let mut rng = SampleRng::new(seed);
    (0..n).map(|_| 0.2 * rng.standard_normal()).collect()
}

/// Powerline buzz: 60 Hz + odd harmonics up to ~1 kHz with per-cycle amplitude
/// jitter, plus a low broadband hash floor. The classic S7 suburban noise.
pub fn powerline(n: usize, sr: u32, seed: u64) -> Vec<f32> {
    let mut rng = SampleRng::new(seed);
    let harmonics: Vec<(f32, f32)> = (0..8)
        .map(|k| {
            let f = 60.0 * (2 * k + 1) as f32;
            let a = 1.0 / (k + 1) as f32;
            (f, a)
        })
        .collect();
    let mut x = vec![0.0f32; n];
    // Amplitude jitter updated once per mains cycle gives the buzz its raspy character.
    let cycle = (sr as f32 / 60.0) as usize;
    let mut jitter = 1.0f32;
    for (i, v) in x.iter_mut().enumerate() {
        if i % cycle == 0 {
            jitter = 0.7 + 0.6 * rng.uniform() as f32;
        }
        let t = i as f32 / sr as f32;
        let mut s = 0.0f32;
        for &(f, a) in &harmonics {
            s += a * (TWO_PI * f * t).sin();
        }
        *v = 0.12 * jitter * s + 0.02 * rng.standard_normal();
    }
    x
}

/// Atmospheric-QRN-style static crashes: sparse exponential-decay noise bursts
/// (~4 per second) over a faint noise floor.
pub fn impulses(n: usize, sr: u32, seed: u64) -> Vec<f32> {
    let mut rng = SampleRng::new(seed);
    let mut x: Vec<f32> = (0..n).map(|_| 0.01 * rng.standard_normal()).collect();
    let per_sample_rate = 4.0 / sr as f64;
    let decay_samples = (0.03 * sr as f32) as usize; // 30 ms crash tail
    let mut i = 0usize;
    while i < n {
        if rng.uniform() < per_sample_rate {
            let amp = 0.3 + 0.7 * rng.uniform() as f32;
            for j in 0..decay_samples.min(n - i) {
                let env = (-(j as f32) / (decay_samples as f32 / 5.0)).exp();
                x[i + j] += amp * env * rng.standard_normal();
            }
            i += decay_samples;
        } else {
            i += 1;
        }
    }
    x
}

/// Scale `noise` so that `clean + noise` sits at `snr_db`, and return the mix.
pub fn mix_at_snr(clean: &[f32], noise: &[f32], snr_db: f64) -> Vec<f32> {
    let power = |v: &[f32]| {
        v.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / v.len().max(1) as f64
    };
    let p_clean = power(clean);
    let p_noise = power(noise);
    let scale = if p_noise > 0.0 {
        (p_clean / (p_noise * 10f64.powf(snr_db / 10.0))).sqrt()
    } else {
        0.0
    } as f32;
    clean
        .iter()
        .zip(noise.iter().cycle())
        .map(|(c, n)| c + scale * n)
        .collect()
}

/// Write a mono float32 WAV.
pub fn write_wav(path: &Path, x: &[f32], sr: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for v in x {
        w.write_sample(*v).map_err(|e| e.to_string())?;
    }
    w.finalize().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    #[test]
    fn generators_are_deterministic() {
        assert_eq!(
            speech_like(SR as usize, SR, 7),
            speech_like(SR as usize, SR, 7)
        );
        assert_eq!(powerline(SR as usize, SR, 7), powerline(SR as usize, SR, 7));
        assert_eq!(impulses(SR as usize, SR, 7), impulses(SR as usize, SR, 7));
        assert_ne!(white(1_000, 1), white(1_000, 2));
    }

    #[test]
    fn mix_hits_requested_snr() {
        let clean = speech_like(4 * SR as usize, SR, 42);
        let noise = white(4 * SR as usize, 42);
        for snr_db in [-6.0, 0.0, 10.0] {
            let mix = mix_at_snr(&clean, &noise, snr_db);
            let resid: Vec<f64> = mix
                .iter()
                .zip(&clean)
                .map(|(m, c)| (*m - *c) as f64)
                .collect();
            let p = |v: &[f64]| v.iter().map(|s| s * s).sum::<f64>() / v.len() as f64;
            let p_clean =
                clean.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / clean.len() as f64;
            let got = 10.0 * (p_clean / p(&resid)).log10();
            assert!((got - snr_db).abs() < 0.1, "snr {snr_db}: got {got}");
        }
    }

    #[test]
    fn cw_has_keying_gaps_and_bounded_amplitude() {
        let x = cw(2 * SR as usize, SR, 20);
        let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!(peak <= 0.5 + 1e-6);
        // A dit at 20 WPM is 60 ms; key-up spans must contain exact zeros.
        assert!(x.iter().filter(|v| **v == 0.0).count() > SR as usize / 4);
    }

    #[test]
    fn generators_are_finite_and_sized() {
        for x in [
            speech_like(1_000, SR, 1),
            cw(1_000, SR, 25),
            white(1_000, 1),
            powerline(1_000, SR, 1),
            impulses(1_000, SR, 1),
        ] {
            assert_eq!(x.len(), 1_000);
            assert!(x.iter().all(|v| v.is_finite()));
        }
    }
}
