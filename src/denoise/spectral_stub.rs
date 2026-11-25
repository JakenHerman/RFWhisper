//! Lightweight broadband noise suppressor for CI + headless hosts (no NN weights).
//!
//! Not a replacement for DeepFilterNet3 on air; use for pipeline / latency /
//! regression wiring.
//!
//! The OLA normalization clamps the window-sum divisor to a fraction of its
//! interior value (see #110): the Python original divided masked frames by a
//! near-zero window-sum at the very first/last analysis window, which could
//! overshoot boundary samples by ~40x. Clamping attenuates those partial-overlap
//! edges instead of amplifying them; interior samples are unaffected.

use realfft::num_complex::Complex;
use realfft::RealFftPlanner;

/// Symmetric Hann (NumPy `hanning` convention, `sym=True`) used by the stub's STFT.
fn hanning_sym(n: usize) -> Vec<f32> {
    if n == 1 {
        return vec![1.0];
    }
    (0..n)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (n as f32 - 1.0)).cos())
        .collect()
}

/// Linear-interpolated percentile (NumPy default) of `values`; `values` is sorted in place.
fn percentile_sorted(values: &mut [f32], q: f64) -> f32 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = values.len();
    if n == 1 {
        return values[0];
    }
    let pos = q / 100.0 * (n - 1) as f64;
    let lo = pos.floor() as usize;
    let frac = (pos - lo as f64) as f32;
    if lo + 1 >= n {
        values[n - 1]
    } else {
        values[lo] * (1.0 - frac) + values[lo + 1] * frac
    }
}

/// Short-time magnitude-domain Wiener-ish mask (stationary noise assumption).
///
/// Improves SNR on synthetic tone+noise tests; does not target ham-specific QRM.
/// The mask floor is a fixed 0.15 (≈ -16.5 dB) — a gentle gate that's enough to
/// keep CW / FT8 transients stable; if you need a different gate you should reach
/// for DFN3, not parameterise this stub.
pub fn wiener_like_denoise(x: &[f32], sr: u32) -> Vec<f32> {
    if x.is_empty() {
        return Vec::new();
    }
    let n_fft = if sr >= 16_000 { 512 } else { 256 };
    let hop = n_fft / 2;
    let window = hanning_sym(n_fft);

    let pad = (n_fft - (x.len() % hop)) % hop;
    let mut xp = x.to_vec();
    xp.extend(std::iter::repeat(0.0).take(pad));
    if xp.len() < n_fft {
        return x.to_vec();
    }
    let nframes = (xp.len() - n_fft) / hop + 1;

    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(n_fft);
    let c2r = planner.plan_fft_inverse(n_fft);
    let n_bins = n_fft / 2 + 1;

    let mut spec: Vec<Vec<Complex<f32>>> = Vec::with_capacity(nframes);
    let mut buf = vec![0.0f32; n_fft];
    for i in 0..nframes {
        let idx = i * hop;
        for (b, (v, w)) in buf.iter_mut().zip(xp[idx..idx + n_fft].iter().zip(&window)) {
            *b = v * w;
        }
        let mut out = r2c.make_output_vec();
        r2c.process(&mut buf, &mut out).expect("fft forward");
        spec.push(out);
    }

    // 10th-percentile magnitude per bin ≈ stationary noise floor.
    let mut noise_floor = vec![0.0f32; n_bins];
    let mut col = vec![0.0f32; nframes];
    for (bin, floor) in noise_floor.iter_mut().enumerate() {
        for (i, frame) in spec.iter().enumerate() {
            col[i] = frame[bin].norm();
        }
        *floor = percentile_sorted(&mut col, 10.0);
    }

    for frame in &mut spec {
        for (bin, v) in frame.iter_mut().enumerate() {
            let mag = v.norm();
            let snr_est = mag / (noise_floor[bin] + 1e-8);
            let mask = (snr_est * snr_est) / (snr_est * snr_est + 1.0);
            // gentle floor so we never zero bands (helps CW/FT8 stability vs hard gate)
            *v *= mask.clamp(0.15, 1.0);
        }
    }

    let mut out = vec![0.0f32; xp.len()];
    let mut wsum = vec![0.0f32; xp.len()];
    let inv_n = 1.0 / n_fft as f32;
    let mut frame_time = vec![0.0f32; n_fft];
    for (i, frame) in spec.iter_mut().enumerate() {
        let idx = i * hop;
        c2r.process(frame, &mut frame_time).expect("fft inverse");
        for (j, w) in window.iter().enumerate() {
            out[idx + j] += frame_time[j] * inv_n * w;
            wsum[idx + j] += w * w;
        }
    }
    // Clamp the divisor to 10 % of the peak window-sum (#110): at the signal
    // boundaries only a partial window has accumulated, and masked frames no
    // longer cancel the tiny weights the way the identity path does — dividing
    // by them amplified boundary junk up to ~40x. Attenuate instead.
    let wsum_max = wsum.iter().cloned().fold(0.0f32, f32::max);
    let floor = 0.1 * wsum_max;
    for (o, w) in out.iter_mut().zip(&wsum) {
        if *w > 1e-12 {
            *o /= w.max(floor);
        }
    }
    out.truncate(x.len());
    out
}

/// Streaming twin of [`wiener_like_denoise`] for the realtime path.
///
/// The offline stub estimates the noise floor from the 10th percentile of the
/// *whole file*, which a live stream cannot see. This version tracks the floor
/// per bin with a gated recursive average (bins far above the floor are treated
/// as signal and only let it creep up at ~3 dB/s) and
/// applies the same `P / (P + N)` mask with the same 0.15 floor.
///
/// Framing is WOLA with a periodic sqrt-Hann on both analysis and synthesis
/// (n_fft 512, hop 256 at 48 kHz): the squared window sums to exactly 1.0 at 50 %
/// overlap, so with the mask pinned at 1 the output reproduces the input
/// (see `streaming_stub_identity_reconstructs`). Latency is `n_fft - hop`
/// samples (5.3 ms at 48 kHz).
/// A bin above `NOISE_GATE x` the current floor is treated as signal and
/// does not update the noise estimate.
const NOISE_GATE: f32 = 4.0;
/// Per-hop smoothing of the noise estimate (time constant ~100 ms at 48 kHz).
const NOISE_SMOOTH: f32 = 0.95;

pub struct StreamingSpectral {
    n_fft: usize,
    hop: usize,
    window: Vec<f32>,
    mask_floor: f32,
    pending: Vec<f32>,
    ola: Vec<f32>,
    noise: Vec<f32>,
    frames: u64,
    r2c: std::sync::Arc<dyn realfft::RealToComplex<f32>>,
    c2r: std::sync::Arc<dyn realfft::ComplexToReal<f32>>,
    time_buf: Vec<f32>,
    spec_buf: Vec<Complex<f32>>,
}

impl StreamingSpectral {
    /// Streaming stub at `sr` with the standard 0.15 mask floor.
    pub fn new(sr: u32) -> Self {
        Self::with_mask_floor(sr, 0.15)
    }

    /// Same, with an explicit mask floor (`1.0` = identity; used by tests to
    /// prove the WOLA framing reconstructs perfectly).
    pub fn with_mask_floor(sr: u32, mask_floor: f32) -> Self {
        let n_fft = if sr >= 16_000 { 512 } else { 256 };
        let hop = n_fft / 2;
        let window = (0..n_fft)
            .map(|i| {
                (0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n_fft as f32).cos()).sqrt()
            })
            .collect();
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(n_fft);
        let c2r = planner.plan_fft_inverse(n_fft);
        let spec_buf = r2c.make_output_vec();
        Self {
            n_fft,
            hop,
            window,
            mask_floor,
            pending: Vec::with_capacity(4 * n_fft),
            ola: vec![0.0; n_fft],
            noise: vec![0.0; n_fft / 2 + 1],
            frames: 0,
            r2c,
            c2r,
            time_buf: vec![0.0; n_fft],
            spec_buf,
        }
    }

    /// Samples of latency between an input sample and its output.
    pub fn latency(&self) -> usize {
        self.n_fft - self.hop
    }

    /// Feed samples; enhanced samples are appended to `out` one hop at a time.
    /// Output index `k` corresponds to input index `k`.
    pub fn process(&mut self, x: &[f32], out: &mut Vec<f32>) {
        self.pending.extend_from_slice(x);
        let mut pos = 0;
        // Per-hop power rise for the noise tracker: ~3 dB/s.
        let rise = 2f32.powf(self.hop as f32 / 48_000.0);
        let inv_n = 1.0 / self.n_fft as f32;
        while self.pending.len() - pos >= self.n_fft {
            for ((b, v), w) in self
                .time_buf
                .iter_mut()
                .zip(&self.pending[pos..pos + self.n_fft])
                .zip(&self.window)
            {
                *b = v * w;
            }
            self.r2c
                .process(&mut self.time_buf, &mut self.spec_buf)
                .expect("fft forward");
            for (bin, v) in self.spec_buf.iter_mut().enumerate() {
                let p = v.norm_sqr();
                let n = &mut self.noise[bin];
                if self.frames == 0 {
                    *n = p;
                } else if p < NOISE_GATE * *n {
                    // Likely noise-only: track the mean noise power (~100 ms).
                    *n = NOISE_SMOOTH * *n + (1.0 - NOISE_SMOOTH) * p;
                } else {
                    // Likely signal: let the floor creep up slowly in case the
                    // noise itself got louder.
                    *n *= rise;
                }
                let mask = p / (p + *n + 1e-12);
                *v *= mask.clamp(self.mask_floor, 1.0);
            }
            self.frames += 1;
            self.c2r
                .process(&mut self.spec_buf, &mut self.time_buf)
                .expect("fft inverse");
            for ((o, t), w) in self.ola.iter_mut().zip(&self.time_buf).zip(&self.window) {
                *o += t * inv_n * w;
            }
            out.extend_from_slice(&self.ola[..self.hop]);
            self.ola.copy_within(self.hop.., 0);
            let n = self.ola.len();
            self.ola[n - self.hop..].fill(0.0);
            pos += self.hop;
        }
        self.pending.drain(..pos);
    }
}
