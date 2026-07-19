//! DeepFilterNet3 backend contract (#10), gated behind the `dfn` feature.
//!
//! Run with `cargo test --features dfn --test dfn_backend`. These assert the
//! *wrapper* contract — the engine loads, is length-preserving, stays finite, and
//! keeps its output time-aligned with its input — not denoising *quality*, which
//! can only be judged on real speech (DeepFilterNet3 correctly removes synthetic
//! tones as non-speech, so a synthetic-fixture quality assertion would be
//! measuring the fixture, not the model).
#![cfg(feature = "dfn")]

use rfwhisper::denoise::select_engine;

const SR: u32 = 48_000;

fn sine(freq: f64, sr: u32, secs: f64) -> Vec<f32> {
    let n = (sr as f64 * secs) as usize;
    (0..n)
        .map(|i| 0.3 * (std::f64::consts::TAU * freq * i as f64 / sr as f64).sin() as f32)
        .collect()
}

#[test]
fn deepfilternet3_selects_the_real_backend() {
    // With the feature on, "deepfilternet3" must NOT fall back to the stub.
    let mut eng = select_engine("deepfilternet3").expect("DFN3 engine loads");
    assert_eq!(eng.native_sr(), SR, "DeepFilterNet3 is full-band 48 kHz");
    // It runs and returns *something* of the right shape.
    let x = sine(300.0, SR, 1.0);
    let y = eng.process(&x, SR);
    assert_eq!(y.len(), x.len(), "engine must be length-preserving");
    assert!(y.iter().all(|v| v.is_finite()), "output must be finite");
}

#[test]
fn output_length_matches_input_across_sample_rates() {
    // The engine resamples in and back out, so output length tracks *input*
    // length at the input rate, not the 48 kHz native rate.
    let mut eng = select_engine("deepfilternet3").unwrap();
    for &sr in &[16_000u32, 44_100, 48_000] {
        let x = sine(440.0, sr, 0.5);
        let y = eng.process(&x, sr);
        assert_eq!(y.len(), x.len(), "length mismatch at {sr} Hz");
        assert!(y.iter().all(|v| v.is_finite()));
    }
}

#[test]
fn output_stays_time_aligned_with_input() {
    // The wrapper trims the model's algorithmic delay (STFT + lookahead). A
    // broadband burst should come back out at roughly the same sample offset;
    // if the delay compensation regresses, the peak shifts by hundreds of samples.
    let mut eng = select_engine("deepfilternet3").unwrap();
    let n = SR as usize; // 1 s
    let mut x = vec![0.0f32; n];
    // A short noise burst at 0.5 s — broadband, so the model passes some of it.
    let burst_at = n / 2;
    let mut state = 0x9e37_79b9u32;
    for s in x.iter_mut().skip(burst_at).take(SR as usize / 20) {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *s = (state as f32 / u32::MAX as f32 - 0.5) * 0.6;
    }
    let y = eng.process(&x, SR);

    // Center of mass of |y| should land near the burst, not delayed far past it.
    let energy: f32 = y.iter().map(|v| v * v).sum();
    if energy > 1e-6 {
        let com: f32 = y
            .iter()
            .enumerate()
            .map(|(i, v)| i as f32 * v * v)
            .sum::<f32>()
            / energy;
        let drift = (com - burst_at as f32).abs();
        assert!(
            drift < SR as f32 * 0.1, // within 100 ms of the input burst
            "output energy center drifted {drift} samples from the input burst — \
             delay compensation regressed"
        );
    }
}
