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

// --- StreamResampler (realtime path, #14) ----------------------------------

use rfwhisper::dsp::resample::StreamResampler;

fn tone(freq: f64, sr: u32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| (0.5 * (std::f64::consts::TAU * freq * i as f64 / sr as f64).sin()) as f32)
        .collect()
}

/// Chunk boundaries must be invisible: any chunking gives bit-identical output.
#[test]
fn stream_resampler_is_chunking_invariant() {
    for &(a, b) in &[
        (48_000u32, 16_000u32),
        (16_000, 48_000),
        (44_100, 48_000),
        (48_000, 44_100),
    ] {
        let x = tone(997.0, a, a as usize / 2);
        let mut whole = Vec::new();
        StreamResampler::new(a, b).process(&x, &mut whole);

        let mut chunked = Vec::new();
        let mut rs = StreamResampler::new(a, b);
        let sizes = [1usize, 7, 480, 13, 512, 1024, 3];
        let mut i = 0;
        let mut k = 0;
        while i < x.len() {
            let n = sizes[k % sizes.len()].min(x.len() - i);
            rs.process(&x[i..i + n], &mut chunked);
            i += n;
            k += 1;
        }
        assert_eq!(whole, chunked, "{a} -> {b}: chunked output differs");
        let expected = (x.len() as f64 * b as f64 / a as f64).round() as i64;
        assert!(
            (whole.len() as i64 - expected).abs() <= 1,
            "{a} -> {b}: got {} samples, expected ~{expected}",
            whole.len()
        );
    }
}

/// Streaming output equals the offline polyphase result shifted by the group delay.
#[test]
fn stream_resampler_matches_offline_after_delay() {
    for &(a, b) in &[(48_000u32, 16_000u32), (16_000, 48_000)] {
        let x = tone(440.0, a, a as usize / 4);
        let mut y = Vec::new();
        let mut rs = StreamResampler::new(a, b);
        rs.process(&x, &mut y);
        let d = rs.delay_out();
        assert_eq!(d.fract(), 0.0, "integer delay expected for {a} -> {b}");
        let d = d as usize;
        let x64: Vec<f64> = x.iter().map(|v| *v as f64).collect();
        let off =
            rfwhisper::dsp::resample::resample_poly(&x64, b as usize / 16_000, a as usize / 16_000);
        // Compare away from both edges.
        let margin = 200;
        for k in margin..(y.len() - d - margin) {
            let diff = (y[k + d] as f64 - off[k]).abs();
            assert!(diff < 1e-4, "{a} -> {b}: sample {k} differs by {diff}");
        }
    }
}

/// A tone survives 44.1 kHz <-> 48 kHz at unity gain (the common laptop pair).
#[test]
fn stream_resampler_preserves_tone_amplitude() {
    let x = tone(1000.0, 44_100, 44_100);
    let mut y = Vec::new();
    StreamResampler::new(44_100, 48_000).process(&x, &mut y);
    let rms =
        |v: &[f32]| (v.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / v.len() as f64).sqrt();
    let ratio_db = 20.0 * (rms(&y[2000..y.len() - 2000]) / rms(&x[2000..x.len() - 2000])).log10();
    assert!(ratio_db.abs() < 0.05, "gain error {ratio_db:.3} dB");
}

#[test]
fn stream_resampler_equal_rates_is_identity() {
    let x = tone(300.0, 48_000, 1000);
    let mut rs = StreamResampler::new(48_000, 48_000);
    assert!(rs.is_identity());
    assert_eq!(rs.delay_out(), 0.0);
    let mut y = Vec::new();
    rs.process(&x, &mut y);
    assert_eq!(x, y);
}
