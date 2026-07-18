//! Polyphase resampler tests (port of `tests/dsp/test_resample.py`).

mod common;

use common::dot;
use rfwhisper::dsp::resample::{next_chunk_size, resample_16k_to_48k, resample_48k_to_16k};

/// 48 → 16 → 48 kHz roundtrip on a 1 kHz tone reconstructs to better than -60 dB.
#[test]
fn test_resample_roundtrip_1khz_under_60_db() {
    let sr_in = 48_000usize;
    let duration = 6.0f64;
    let f0 = 1_000.0f64;
    let n = (sr_in as f64 * duration) as usize;
    let x: Vec<f64> = (0..n)
        .map(|i| (2.0 * std::f64::consts::PI * f0 * i as f64 / sr_in as f64).sin())
        .collect();

    let y = resample_48k_to_16k(&x);
    assert_eq!(y.len(), n / 3);
    let z = resample_16k_to_48k(&y);
    assert_eq!(z.len(), x.len());

    let trim = sr_in / 2;
    let x_mid = &x[trim..n - trim];
    let z_mid = &z[trim..n - trim];

    // Find best alignment over a modest lag window (polyphase delay is tiny).
    let max_lag = 64i64;
    let mut best_lag = 0i64;
    let mut best_corr = f64::NEG_INFINITY;
    for lag in -max_lag..=max_lag {
        let mut c = 0.0;
        for (i, xv) in x_mid.iter().enumerate() {
            let j = i as i64 + lag;
            if j >= 0 && (j as usize) < z_mid.len() {
                c += xv * z_mid[j as usize];
            }
        }
        if c.abs() > best_corr {
            best_corr = c.abs();
            best_lag = lag;
        }
    }

    let (z_a, x_a): (Vec<f64>, Vec<f64>) = if best_lag >= 0 {
        let lag = best_lag as usize;
        let m = (z_mid.len() - lag).min(x_mid.len());
        (z_mid[lag..lag + m].to_vec(), x_mid[..m].to_vec())
    } else {
        let lag = (-best_lag) as usize;
        let m = z_mid.len().min(x_mid.len() - lag);
        (z_mid[..m].to_vec(), x_mid[lag..lag + m].to_vec())
    };

    let gain = dot(&z_a, &x_a) / (dot(&x_a, &x_a) + 1e-20);
    let err: Vec<f64> = z_a.iter().zip(&x_a).map(|(z, x)| z - gain * x).collect();
    let rms_sig = (dot(&x_a, &x_a) / x_a.len() as f64).sqrt();
    let rms_err = (dot(&err, &err) / err.len() as f64).sqrt();
    let err_db = 20.0 * (rms_err / (rms_sig + 1e-20)).log10();
    assert!(err_db < -60.0, "roundtrip error {err_db:.1} dB");
}

/// `next_chunk_size` sizes the input needed per output block.
#[test]
fn test_next_chunk_size() {
    assert_eq!(next_chunk_size(480, 48_000, 48_000), 480);
    assert_eq!(next_chunk_size(480, 44_100, 48_000), 441); // ceil(480 * 44100 / 48000)
    assert_eq!(next_chunk_size(160, 48_000, 16_000), 480);
}
