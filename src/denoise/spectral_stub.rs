//! Lightweight broadband noise suppressor for CI + headless hosts (no NN weights).
//!
//! Not a replacement for DeepFilterNet3 on air; use for pipeline / latency /
//! regression wiring.
//!
//! Known artifact inherited (bit-for-bit) from the Python original: at the very
//! first/last window of the signal the OLA normalisation divides masked frames by a
//! near-zero window-sum, so boundary samples can overshoot. Interior samples are
//! well-behaved. Tracked for a fix alongside the DFN3 backend work.

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
    for (o, w) in out.iter_mut().zip(&wsum) {
        if *w > 1e-12 {
            *o /= w;
        }
    }
    out.truncate(x.len());
    out
}
