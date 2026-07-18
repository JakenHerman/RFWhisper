//! Shared acceptance-gate metric tests (refs A1, A3, A5) —
//! port of `tests/dsp/test_metrics.py`. (PESQ / STOI stay external tools; see
//! `src/dsp/metrics.rs` module docs.)

mod common;

use common::{dot, TestRng};
use rfwhisper::dsp::metrics::{effective_snr_gain, keying_onset_rms, rtf};

const SR: u32 = 48_000;

/// Band-limited, amplitude-modulated tone stack — stands in for ham SSB audio.
fn speech_like(n: usize, seed: u64) -> Vec<f64> {
    let mut rng = TestRng::new(seed);
    let mut x = vec![0.0f64; n];
    for f0 in [220.0, 480.0, 910.0, 1_600.0] {
        let amp = rng.uniform_in(0.5, 1.0);
        let phase = rng.uniform_in(0.0, 2.0 * std::f64::consts::PI);
        for (i, v) in x.iter_mut().enumerate() {
            let t = i as f64 / SR as f64;
            *v += amp * (2.0 * std::f64::consts::PI * f0 * t + phase).sin();
        }
    }
    for (i, v) in x.iter_mut().enumerate() {
        let t = i as f64 / SR as f64;
        *v *= 0.6 + 0.4 * (2.0 * std::f64::consts::PI * 3.0 * t).sin();
    }
    x
}

/// denoised == clean leaves no residual, so the gain sentinel is +inf.
#[test]
fn test_snr_gain_of_perfect_reconstruction_is_inf() {
    let clean = speech_like(SR as usize, 42);
    let mut rng = TestRng::new(42);
    let noisy: Vec<f64> = clean
        .iter()
        .map(|c| c + 0.5 * rng.standard_normal())
        .collect();
    let gain = effective_snr_gain(&clean, &noisy, &clean, SR).unwrap();
    assert_eq!(gain, f64::INFINITY);
}

/// No noise to remove means "gain" is undefined rather than zero.
#[test]
fn test_snr_gain_is_nan_when_input_was_already_clean() {
    let clean = speech_like(SR as usize, 42);
    let gain = effective_snr_gain(&clean, &clean, &clean, SR).unwrap();
    assert!(gain.is_nan());
}

/// Halving noise power (-3 dB) must read back as +3 dB gain within ±0.3 dB.
#[test]
fn test_snr_gain_matches_synthetic_3_db_improvement() {
    let clean = speech_like(4 * SR as usize, 42);
    let mut rng = TestRng::new(42);
    let mut noise = rng.standard_normal_vec(clean.len());
    let scale = (dot(&clean, &clean) / dot(&noise, &noise)).sqrt(); // 0 dB SNR
    for v in &mut noise {
        *v *= scale;
    }

    let noisy: Vec<f64> = clean.iter().zip(&noise).map(|(c, n)| c + n).collect();
    let k = 10.0f64.powf(-3.0 / 20.0);
    let denoised: Vec<f64> = clean.iter().zip(&noise).map(|(c, n)| c + n * k).collect();

    let gain = effective_snr_gain(&clean, &noisy, &denoised, SR).unwrap();
    assert!((gain - 3.0).abs() <= 0.3, "gain = {gain}");
}

/// Algorithmic delay and a level change are alignment/scale, not noise.
#[test]
fn test_snr_gain_tolerates_denoiser_delay_and_gain() {
    let clean = speech_like(4 * SR as usize, 42);
    let mut rng = TestRng::new(42);
    let noise: Vec<f64> = (0..clean.len())
        .map(|_| 0.3 * rng.standard_normal())
        .collect();
    let delay = (0.02 * SR as f64) as usize; // 20 ms, well inside MAX_ALIGN_MS

    let noisy: Vec<f64> = clean.iter().zip(&noise).map(|(c, n)| c + n).collect();
    let mut delayed = vec![0.0; delay];
    delayed.extend(clean.iter().zip(&noise).map(|(c, n)| 2.0 * (c + 0.1 * n)));

    let want = 20.0 * (1.0f64 / 0.1).log10();
    let gain = effective_snr_gain(&clean, &noisy, &delayed, SR).unwrap();
    assert!((gain - want).abs() <= 0.5, "gain = {gain}, want ≈ {want}");
}

/// Empty input must be rejected (the Rust-port equivalent of the 1-D mono check).
#[test]
fn test_snr_gain_rejects_empty() {
    let clean = speech_like(SR as usize, 42);
    let err = effective_snr_gain(&clean, &[], &clean, SR).unwrap_err();
    assert!(err.0.contains("must be non-empty"));
}

/// Rectangular bursts of known amplitude read back as that amplitude within 1 %.
#[test]
fn test_keying_onset_rms_matches_analytical_click_train() {
    let width = (5.0 * SR as f64 / 1000.0).round() as usize;
    let onsets = [0usize, SR as usize / 2, SR as usize];
    let amplitudes = [1.0, 0.5, 0.25];

    let mut x = vec![0.0f64; 2 * SR as usize];
    for (&onset, &amp) in onsets.iter().zip(&amplitudes) {
        for v in &mut x[onset..onset + width] {
            *v = amp;
        }
    }

    let measured = keying_onset_rms(&x, &onsets, 5.0, SR).unwrap();
    for (m, a) in measured.iter().zip(&amplitudes) {
        assert!((m - a).abs() <= 0.01 * a, "measured {m}, want {a}");
    }
}

/// A 5 ms window at 16 kHz is 80 samples, so a 40-sample burst reads as -3 dB RMS.
#[test]
fn test_keying_onset_rms_window_scales_with_sample_rate() {
    let sr = 16_000u32;
    let mut x = vec![0.0f64; sr as usize];
    for v in &mut x[100..140] {
        *v = 1.0;
    }
    let measured = keying_onset_rms(&x, &[100], 5.0, sr).unwrap();
    let want = (40.0f64 / 80.0).sqrt();
    assert!((measured[0] - want).abs() <= 1e-9 * want);
}

#[test]
fn test_keying_onset_rms_rejects_window_past_end() {
    let x = vec![0.0f64; 1_000];
    let err = keying_onset_rms(&x, &[900], 5.0, SR).unwrap_err();
    assert!(err.0.contains("falls outside the signal"));
}

#[test]
fn test_rtf_reports_wall_seconds_per_audio_second() {
    assert!((rtf(10.0, 2.5).unwrap() - 0.25).abs() <= 1e-12);
    let err = rtf(0.0, 1.0).unwrap_err();
    assert!(err.0.contains("input_duration_s must be positive"));
}
