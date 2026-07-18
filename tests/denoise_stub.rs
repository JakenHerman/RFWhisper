//! End-to-end sanity for the spectral stub engine: on a synthetic tone + white
//! noise mix it must improve effective SNR (the wiring the acceptance gates rely
//! on before real NN weights are present).

mod common;

use common::TestRng;
use rfwhisper::denoise::{select_engine, DenoiseEngine, SpectralStubEngine};
use rfwhisper::dsp::metrics::effective_snr_gain;

const SR: u32 = 48_000;

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| (*v as f64) * (*v as f64)).sum::<f64>() / x.len() as f64).sqrt()
}

/// Stationary noise must be attenuated in the steady-state interior (the stub's
/// Wiener-ish mask sits well below 1 on noise-only bins). The first/last window is
/// excluded: the stub inherits the Python original's OLA edge artifact, where masked
/// frames divided by a near-zero window-sum can overshoot at the signal boundaries.
#[test]
fn test_stub_attenuates_stationary_noise_interior() {
    let n = 2 * SR as usize;
    let mut rng = TestRng::new(7);
    let noise: Vec<f32> = (0..n)
        .map(|_| 0.25 * rng.standard_normal() as f32)
        .collect();

    let mut eng = SpectralStubEngine;
    let out = eng.process(&noise, SR);
    assert_eq!(out.len(), noise.len());
    assert!(out.iter().all(|v| v.is_finite()));

    let trim = 1024;
    // Reference: the Python original measures −0.75 dB on this exact setup — white
    // noise keeps most bins near the mask's upper range, so attenuation is mild.
    let att_db = 20.0 * (rms(&out[trim..n - trim]) / rms(&noise[trim..n - trim])).log10();
    assert!(
        att_db < -0.5,
        "stub should attenuate stationary noise (Python reference: -0.75 dB), got {att_db:.2} dB"
    );
}

/// Boundary regression for #110: the first/last window must not overshoot.
/// Before the OLA divisor clamp, masked frames divided by a near-zero window-sum
/// blew the boundary up to ~40x interior RMS.
#[test]
fn test_stub_boundaries_do_not_overshoot() {
    let n = 2 * SR as usize;
    let mut rng = TestRng::new(7);
    let noisy: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f64 / SR as f64;
            let sig = 0.5
                * (2.0 * std::f64::consts::PI * 800.0 * t).sin()
                * (0.6 + 0.4 * (2.0 * std::f64::consts::PI * 3.0 * t).sin());
            (sig + 0.25 * rng.standard_normal()) as f32
        })
        .collect();

    let mut eng = SpectralStubEngine;
    let out = eng.process(&noisy, SR);

    let edge = 512; // one stub FFT window at 48 kHz
    let interior_rms = rms(&out[edge..n - edge]);
    let head_rms = rms(&out[..edge]);
    let tail_rms = rms(&out[n - edge..]);
    assert!(
        head_rms <= 2.0 * interior_rms,
        "head rms {head_rms:.4} vs interior {interior_rms:.4}"
    );
    assert!(
        tail_rms <= 2.0 * interior_rms,
        "tail rms {tail_rms:.4} vs interior {interior_rms:.4}"
    );
}

/// On a modulated two-tone + noise mix the stub must be roughly transparent to the
/// signal in the interior (|gain| small): it's acceptance wiring, not a quality
/// denoiser — the guarantee is "doesn't wreck the signal", not "+N dB".
#[test]
fn test_stub_near_transparent_on_modulated_signal_interior() {
    let n = 2 * SR as usize;
    let mut rng = TestRng::new(7);
    let clean: Vec<f64> = (0..n)
        .map(|i| {
            let t = i as f64 / SR as f64;
            let carrier = (2.0 * std::f64::consts::PI * 800.0 * t).sin()
                + 0.6 * (2.0 * std::f64::consts::PI * 1_500.0 * t).sin();
            let envelope = 0.6 + 0.4 * (2.0 * std::f64::consts::PI * 3.0 * t).sin();
            0.5 * carrier * envelope
        })
        .collect();
    let noisy: Vec<f64> = clean
        .iter()
        .map(|c| c + 0.25 * rng.standard_normal())
        .collect();

    let noisy32: Vec<f32> = noisy.iter().map(|v| *v as f32).collect();
    let mut eng = SpectralStubEngine;
    let denoised32 = eng.process(&noisy32, SR);
    assert_eq!(denoised32.len(), noisy32.len());

    let trim = 1024;
    let denoised: Vec<f64> = denoised32[trim..n - trim]
        .iter()
        .map(|v| *v as f64)
        .collect();
    let gain = effective_snr_gain(
        &clean[trim..n - trim],
        &noisy[trim..n - trim],
        &denoised,
        SR,
    )
    .unwrap();
    assert!(
        gain.abs() < 2.0,
        "stub should be near-transparent in the interior, got {gain:.2} dB"
    );
}

#[test]
fn test_select_engine_names() {
    assert!(select_engine("spectral_stub").is_ok());
    assert!(select_engine("deepfilternet3").is_ok()); // warns + falls back to stub
    assert!(select_engine("model.onnx").is_err()); // ONNX backend not yet ported
    assert!(select_engine("not_a_model").is_err());
}

#[test]
fn test_process_file_reports_stats() {
    let x = vec![0.1f32; SR as usize];
    let mut eng = SpectralStubEngine;
    let (y, stats) = eng.process_file(&x, SR);
    assert_eq!(y.len(), x.len());
    assert!((stats.seconds_audio - 1.0).abs() < 1e-9);
    assert!(stats.rtf() >= 0.0);
}
