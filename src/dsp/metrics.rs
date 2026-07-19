//! Shared audio-quality metrics for the ROADMAP acceptance gates (A1, A3, A5).
//!
//! Every gate that needs a number gets it from here, so a threshold can never drift
//! between two tests that claim to measure the same thing.
//!
//! PESQ (ITU-T P.862) and STOI have no maintained Rust implementations; the A6 gate
//! keeps using the reference tools out-of-process (see `docs/` and the A6 issue).

use realfft::RealFftPlanner;

use crate::dsp::DspError;

/// Alignment search window for the matched filter. Denoisers introduce algorithmic
/// delay (WOLA framing, lookahead); 50 ms covers every v0.1 path with margin.
pub const MAX_ALIGN_MS: f64 = 50.0;

fn require_non_empty(x: &[f64], name: &str) -> Result<(), DspError> {
    if x.is_empty() {
        return Err(DspError::new(format!("{name} must be non-empty")));
    }
    Ok(())
}

/// Full cross-correlation of `x` against `ref_` (equivalent to
/// `numpy.correlate(x, ref, mode="full")` via FFT), length `x.len() + ref_.len() - 1`.
/// Zero lag sits at index `ref_.len() - 1`.
fn correlate_full(x: &[f64], ref_: &[f64]) -> Vec<f64> {
    let n = x.len() + ref_.len() - 1;
    // Correlation = convolution with the reversed reference.
    let len = n.next_power_of_two();
    let mut planner = RealFftPlanner::<f64>::new();
    let r2c = planner.plan_fft_forward(len);
    let c2r = planner.plan_fft_inverse(len);

    let mut a = vec![0.0; len];
    a[..x.len()].copy_from_slice(x);
    let mut b = vec![0.0; len];
    for (i, v) in ref_.iter().rev().enumerate() {
        b[i] = *v;
    }

    let mut fa = r2c.make_output_vec();
    let mut fb = r2c.make_output_vec();
    r2c.process(&mut a, &mut fa).expect("fft forward");
    r2c.process(&mut b, &mut fb).expect("fft forward");
    for (va, vb) in fa.iter_mut().zip(&fb) {
        *va *= vb;
    }
    let mut out = vec![0.0; len];
    c2r.process(&mut fa, &mut out).expect("fft inverse");
    let scale = 1.0 / len as f64;
    out.truncate(n);
    for v in &mut out {
        *v *= scale;
    }
    out
}

/// Lag (in samples) of `x` relative to `ref_` maximising |cross-correlation|.
fn best_lag(x: &[f64], ref_: &[f64], max_lag: usize) -> i64 {
    let corr = correlate_full(x, ref_);
    let zero = ref_.len() - 1;
    let lo = zero.saturating_sub(max_lag);
    let hi = (zero + max_lag + 1).min(corr.len());
    let mut best = lo;
    let mut best_val = f64::NEG_INFINITY;
    for (i, v) in corr[lo..hi].iter().enumerate() {
        if v.abs() > best_val {
            best_val = v.abs();
            best = lo + i;
        }
    }
    best as i64 - zero as i64
}

/// SNR of `x` against clean `ref_`, in dB.
///
/// Aligns `x` to `ref_` by cross-correlation, projects it onto the reference to
/// recover the best-fit scale (so gain differences are not counted as noise), and
/// reports `10 log10(||projection||^2 / ||residual||^2)`.
///
/// Returns `+inf` when the residual is exactly zero (`x` is a scaled copy of
/// `ref_`) — see [`effective_snr_gain`] for what that means for callers.
/// Matched-filter SNR in dB of `x` against clean reference `ref_`.
///
/// `x` is aligned to `ref_` by cross-correlation (up to `MAX_ALIGN_MS`), then
/// projected onto it: the component of `x` parallel to `ref_` is signal, the
/// residual is noise. This is the per-signal number the A1 report shows before
/// and after denoising; [`effective_snr_gain`] is just the difference of two of
/// these. Same `+inf` / `-inf` sentinels as that function.
pub fn matched_filter_snr_db(x: &[f64], ref_: &[f64], sr: u32) -> Result<f64, DspError> {
    let max_lag = (MAX_ALIGN_MS * sr as f64 / 1000.0).round() as usize;
    let lag = best_lag(x, ref_, max_lag);
    let (x_seg, ref_seg): (&[f64], &[f64]) = if lag >= 0 {
        let lag = lag as usize;
        let n = (x.len().saturating_sub(lag)).min(ref_.len());
        if n == 0 {
            return Err(DspError::new(
                "signals do not overlap after alignment; check lengths and sample rate",
            ));
        }
        (&x[lag..lag + n], &ref_[..n])
    } else {
        let shift = (-lag) as usize;
        let n = x.len().min(ref_.len().saturating_sub(shift));
        if n == 0 {
            return Err(DspError::new(
                "signals do not overlap after alignment; check lengths and sample rate",
            ));
        }
        (&x[..n], &ref_[shift..shift + n])
    };

    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(p, q)| p * q).sum::<f64>();
    let ref_energy = dot(ref_seg, ref_seg);
    if ref_energy <= 0.0 {
        return Err(DspError::new(
            "clean reference is all zeros; SNR is undefined",
        ));
    }
    let scale = dot(x_seg, ref_seg) / ref_energy;
    let signal_energy = scale * scale * ref_energy;
    let residual_energy: f64 = x_seg
        .iter()
        .zip(ref_seg)
        .map(|(xv, rv)| {
            let e = xv - scale * rv;
            e * e
        })
        .sum();

    if signal_energy <= 0.0 {
        return Ok(f64::NEG_INFINITY);
    }
    if residual_energy <= 0.0 {
        return Ok(f64::INFINITY);
    }
    Ok(10.0 * (signal_energy / residual_energy).log10())
}

/// Effective SNR gain in dB from denoising, measured against a clean reference (A1).
///
/// Both `noisy` and `denoised` are matched-filtered against `clean`; the result is
/// `snr(denoised) - snr(noisy)`. Positive means the denoiser helped.
///
/// Sentinels, so callers can assert on them rather than on NaN:
///
/// * `+inf` — `denoised` is a scaled copy of `clean` (perfect reconstruction).
/// * `-inf` — `denoised` retains no component of `clean` at all.
/// * `NaN`  — `noisy` was already a perfect copy of `clean`, so there was no noise
///   to remove and "gain" has no meaning.
pub fn effective_snr_gain(
    clean: &[f64],
    noisy: &[f64],
    denoised: &[f64],
    sr: u32,
) -> Result<f64, DspError> {
    if sr == 0 {
        return Err(DspError::new("sr must be positive"));
    }
    require_non_empty(clean, "clean")?;
    require_non_empty(noisy, "noisy")?;
    require_non_empty(denoised, "denoised")?;

    let snr_noisy = matched_filter_snr_db(noisy, clean, sr)?;
    let snr_denoised = matched_filter_snr_db(denoised, clean, sr)?;
    if snr_noisy.is_infinite() && snr_noisy > 0.0 {
        return Ok(f64::NAN);
    }
    Ok(snr_denoised - snr_noisy)
}

/// RMS of the first `window_ms` after each key-down, one value per onset (A3).
///
/// A3 compares this array between raw and denoised audio: a denoiser that softens CW
/// keying edges shows up here as a drop, even when broadband metrics look fine.
pub fn keying_onset_rms(
    signal: &[f64],
    onset_samples: &[usize],
    window_ms: f64,
    sr: u32,
) -> Result<Vec<f64>, DspError> {
    if sr == 0 {
        return Err(DspError::new("sr must be positive"));
    }
    if window_ms <= 0.0 {
        return Err(DspError::new("window_ms must be positive"));
    }
    require_non_empty(signal, "signal")?;

    let width = (window_ms * sr as f64 / 1000.0).round() as usize;
    if width == 0 {
        return Err(DspError::new(
            "window_ms is too short to cover a single sample at this sample rate",
        ));
    }

    let mut out = Vec::with_capacity(onset_samples.len());
    for &onset in onset_samples {
        if onset + width > signal.len() {
            return Err(DspError::new(format!(
                "onset {onset} with a {width}-sample window falls outside the signal (length {})",
                signal.len()
            )));
        }
        let window = &signal[onset..onset + width];
        let energy: f64 = window.iter().map(|v| v * v).sum();
        out.push((energy / width as f64).sqrt());
    }
    Ok(out)
}

/// Real-time factor: wall-clock seconds spent per second of audio (A5).
///
/// `< 1.0` is faster than real time; A5 wants `< 0.5` to leave the rest of the
/// shack's software some headroom.
pub fn rtf(input_duration_s: f64, process_wall_s: f64) -> Result<f64, DspError> {
    if input_duration_s <= 0.0 {
        return Err(DspError::new("input_duration_s must be positive"));
    }
    if process_wall_s < 0.0 {
        return Err(DspError::new("process_wall_s must not be negative"));
    }
    Ok(process_wall_s / input_duration_s)
}
