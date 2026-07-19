//! Synthetic RFI generators for the acceptance gates (A1, A3, A6).
//!
//! Every generator is a pure function of `(sr, duration_s, …, seed)` — same
//! arguments in, bit-identical samples out, on any machine. That is what lets CI
//! assert hard dB thresholds: a gate failure means the denoiser changed, never
//! that the fixture drifted.
//!
//! The four noise types mirror the RFI sources called out in the README:
//!
//! * [`powerline_buzz`]  — switch-mode supplies, LED drivers, arcing insulators
//! * [`solar_inverter`]  — MPPT / inverter switching, rhythmic buzz across HF
//! * [`vdsl_hash`]       — PLC / Ethernet-over-powerline, a raised broadband floor
//! * [`atmospheric_qrn`] — static crashes on the low bands
//!
//! [`mix`] combines any of them with a clean signal at an exact SNR.
//!
//! Generators return `f64` in `[-1, 1]`, peak-normalized so a clip can be written
//! straight to a WAV without clipping. Absolute level carries no meaning — [`mix`]
//! sets the level that matters.

use crate::dsp::filter::{butter_bandpass, butter_lowpass};
use crate::dsp::{require_positive, DspError};
use crate::samples::rng::SeededRng;

/// Peak headroom for generated clips: loud enough to hear, short of full scale.
pub const PEAK: f64 = 0.95;

/// Validate rate/duration and return the sample count.
fn n_samples(sr: u32, duration_s: f64) -> Result<usize, DspError> {
    if sr == 0 {
        return Err(DspError::new("sr must be positive"));
    }
    require_positive(duration_s, "duration_s")?;
    let n = (sr as f64 * duration_s).round() as i64;
    if n <= 0 {
        return Err(DspError::new(
            "duration_s is too short to produce a single sample at this sr",
        ));
    }
    Ok(n as usize)
}

/// Peak-normalize to [`PEAK`]; all-zero input is returned unchanged.
fn normalize(mut x: Vec<f64>) -> Vec<f64> {
    let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    if peak <= 0.0 {
        return x;
    }
    let scale = PEAK / peak;
    for v in &mut x {
        *v *= scale;
    }
    x
}

/// Harmonic comb from mains-borne RFI — the classic S7 buzz across a quiet band.
///
/// A stack of `n_harmonics` partials of `fundamental_hz` with 1/n amplitude
/// rolloff, randomised phase, and a slow per-harmonic amplitude wobble (real buzz
/// breathes with the load rather than sitting perfectly still). Harmonics above
/// Nyquist are dropped.
pub fn powerline_buzz(
    sr: u32,
    duration_s: f64,
    fundamental_hz: f64,
    n_harmonics: usize,
    seed: u64,
) -> Result<Vec<f64>, DspError> {
    require_positive(fundamental_hz, "fundamental_hz")?;
    if n_harmonics == 0 {
        return Err(DspError::new("n_harmonics must be positive"));
    }
    let n = n_samples(sr, duration_s)?;
    let mut rng = SeededRng::new(seed);
    let nyquist = sr as f64 / 2.0;

    let mut out = vec![0.0f64; n];
    for k in 1..=n_harmonics {
        let f = k as f64 * fundamental_hz;
        if f >= nyquist {
            break;
        }
        let phase = rng.uniform_in(0.0, std::f64::consts::TAU);
        let wobble_hz = rng.uniform_in(0.2, 1.5);
        let wobble_phase = rng.uniform_in(0.0, std::f64::consts::TAU);
        let amp = 1.0 / k as f64;
        for (i, o) in out.iter_mut().enumerate() {
            let t = i as f64 / sr as f64;
            let wobble = 1.0 + 0.25 * (std::f64::consts::TAU * wobble_hz * t + wobble_phase).sin();
            *o += amp * wobble * (std::f64::consts::TAU * f * t + phase).sin();
        }
    }
    Ok(normalize(out))
}

/// Rhythmic switching buzz — an impulse train that rings a resonance.
///
/// Each tick at `tick_rate_hz` (twice mains, i.e. rectified) excites a damped
/// sinusoid whose decay is set by `ringing_q`: `tau = Q / (pi * f0)`. Higher Q
/// rings longer and sounds more tonal. Tick amplitude and center frequency jitter
/// slightly per tick, which is what makes an inverter sound different from a
/// clean square wave.
pub fn solar_inverter(
    sr: u32,
    duration_s: f64,
    tick_rate_hz: f64,
    ringing_q: f64,
    seed: u64,
) -> Result<Vec<f64>, DspError> {
    require_positive(tick_rate_hz, "tick_rate_hz")?;
    require_positive(ringing_q, "ringing_q")?;
    let n = n_samples(sr, duration_s)?;
    let mut rng = SeededRng::new(seed);

    let mut out = vec![0.0f64; n];
    let period = sr as f64 / tick_rate_hz;
    let center_hz = 3_000.0f64.min(sr as f64 / 4.0);
    let n_ticks = (n as f64 / period) as usize;
    for i in 0..=n_ticks {
        let start = (i as f64 * period).round() as usize;
        if start >= n {
            break;
        }
        let f0 = center_hz * rng.uniform_in(0.9, 1.1);
        let tau = ringing_q / (std::f64::consts::PI * f0);
        // Truncate each ring at ~5 tau; beyond that it is below -43 dB and just costs time.
        let length = (n - start).min((5.0 * tau * sr as f64).round() as usize + 1);
        let amp = rng.uniform_in(0.7, 1.0);
        for (j, o) in out[start..start + length].iter_mut().enumerate() {
            let tt = j as f64 / sr as f64;
            *o += amp * (-tt / tau).exp() * (std::f64::consts::TAU * f0 * tt).sin();
        }
    }
    Ok(normalize(out))
}

/// Gaussian white noise — the flat, structureless control case.
///
/// Nothing on the bands actually sounds like this, which is the point: a
/// denoiser that helps on white noise and nothing else has learned gain, not
/// structure. Cheap enough to be the default in a quick `samples synth` call.
pub fn white(sr: u32, duration_s: f64, seed: u64) -> Result<Vec<f64>, DspError> {
    let n = n_samples(sr, duration_s)?;
    let mut rng = SeededRng::new(seed);
    Ok(normalize(rng.standard_normal_vec(n)))
}

/// Wideband raised noise floor — PLC / VDSL / Ethernet-over-powerline hash.
///
/// Band-limited white noise (300 Hz – 0.45 Nyquist) plus a few weak stationary
/// carriers. Perceptually this is the "the band just got 10 dB louder and there
/// is nothing to null out" case, and it is the hardest of the four for a
/// classical notch to touch — which is the point of testing against it.
pub fn vdsl_hash(sr: u32, duration_s: f64, seed: u64) -> Result<Vec<f64>, DspError> {
    let n = n_samples(sr, duration_s)?;
    let mut rng = SeededRng::new(seed);

    let low = 300.0;
    let high = 0.45 * (sr as f64 / 2.0);
    if low >= high {
        return Err(DspError::new(
            "sr is too low to synthesize VDSL hash (needs > ~1.4 kHz)",
        ));
    }
    let mut shaped = rng.standard_normal_vec(n);
    butter_bandpass(4, low, high, sr)?.filter(&mut shaped);

    // A handful of DMT-ish residual carriers.
    for f in [2_100.0, 3_400.0, 5_600.0] {
        if f >= high {
            continue;
        }
        let phase = rng.uniform_in(0.0, std::f64::consts::TAU);
        for (i, v) in shaped.iter_mut().enumerate() {
            let t = i as f64 / sr as f64;
            *v += 0.06 * (std::f64::consts::TAU * f * t + phase).sin();
        }
    }
    Ok(normalize(shaped))
}

/// Static crashes — Poisson-timed broadband bursts with exponential decay.
///
/// `crash_rate_hz` is the mean crash rate; inter-arrival times are exponential,
/// so the clustering is realistic rather than metronomic. Each crash is filtered
/// white noise with a 3–25 ms decay, the range that reads as "distant storm"
/// through an SSB filter.
pub fn atmospheric_qrn(
    sr: u32,
    duration_s: f64,
    crash_rate_hz: f64,
    seed: u64,
) -> Result<Vec<f64>, DspError> {
    require_positive(crash_rate_hz, "crash_rate_hz")?;
    let n = n_samples(sr, duration_s)?;
    let mut rng = SeededRng::new(seed);

    let mut out = vec![0.0f64; n];
    let high = 6_000.0f64.min(0.45 * (sr as f64 / 2.0));

    let mut t_s = rng.exponential(1.0 / crash_rate_hz);
    while t_s < duration_s {
        let start = (t_s * sr as f64).round() as usize;
        if start >= n {
            break;
        }
        let tau = rng.uniform_in(0.003, 0.025);
        let length = (n - start).min((6.0 * tau * sr as f64).round() as usize + 1);
        let amp = rng.uniform_in(0.5, 1.0);
        for j in 0..length {
            let tt = j as f64 / sr as f64;
            out[start + j] += amp * rng.standard_normal() * (-tt / tau).exp();
        }
        t_s += rng.exponential(1.0 / crash_rate_hz);
    }

    butter_lowpass(2, high, sr)?.filter(&mut out);
    Ok(normalize(out))
}

/// Add `noise` to `clean` at exactly `snr_db`, by signal-to-noise power ratio.
///
/// The noise is rescaled — the clean signal is never touched — so the caller's
/// reference stays sample-aligned with the mix, which is what
/// [`crate::dsp::metrics::effective_snr_gain`] needs. The result is *not*
/// normalized: peak-normalizing here would silently change the SNR the caller
/// just asked for. Clip-check before writing a WAV.
pub fn mix(clean: &[f64], noise: &[f64], snr_db: f64) -> Result<Vec<f64>, DspError> {
    if clean.len() != noise.len() {
        return Err(DspError::new(format!(
            "clean and noise must be the same length, got {} and {}",
            clean.len(),
            noise.len()
        )));
    }
    let clean_power: f64 = clean.iter().map(|v| v * v).sum();
    let noise_power: f64 = noise.iter().map(|v| v * v).sum();
    if clean_power <= 0.0 {
        return Err(DspError::new("clean is all zeros; SNR is undefined"));
    }
    if noise_power <= 0.0 {
        return Err(DspError::new("noise is all zeros; SNR is undefined"));
    }

    let target = clean_power / 10f64.powf(snr_db / 10.0);
    let scale = (target / noise_power).sqrt();
    Ok(clean
        .iter()
        .zip(noise)
        .map(|(c, n)| c + scale * n)
        .collect())
}
