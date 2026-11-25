//! Realtime signal chain (#14): streaming engines and `StreamPipeline` must be
//! stateful and chunking-invariant, so the audio callbacks can hand over audio
//! in whatever block size the OS picks without clicks at block boundaries.
//! Maps to ROADMAP A4 (the chain the latency probe measures) and A3 (the A/B
//! path must not touch the signal when bypassed).

mod common;

use common::TestRng;
use rfwhisper::denoise::spectral_stub::StreamingSpectral;
use rfwhisper::denoise::{select_engine, DenoiseEngine, PassthroughEngine, SpectralStubEngine};
use rfwhisper::realtime::pipeline::StreamPipeline;

const SR: u32 = 48_000;

fn noise(n: usize, seed: u64, amp: f32) -> Vec<f32> {
    let mut rng = TestRng::new(seed);
    (0..n).map(|_| amp * rng.standard_normal() as f32).collect()
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt()
}

/// Feed `x` through `f` in an irregular chunking.
fn chunked(x: &[f32], mut f: impl FnMut(&[f32], &mut Vec<f32>)) -> Vec<f32> {
    let sizes = [480usize, 1, 512, 37, 1024, 441, 2048];
    let (mut out, mut i, mut k) = (Vec::new(), 0, 0);
    while i < x.len() {
        let n = sizes[k % sizes.len()].min(x.len() - i);
        f(&x[i..i + n], &mut out);
        i += n;
        k += 1;
    }
    out
}

/// AGENTS.md: overlap-add windows must sum to 1.0. With the mask pinned at 1
/// the streaming stub must reproduce its input exactly (after the first hop,
/// which only has one window's worth of overlap).
#[test]
fn streaming_stub_identity_reconstructs() {
    let x = noise(SR as usize / 2, 3, 0.3);
    let mut s = StreamingSpectral::with_mask_floor(SR, 1.0);
    let mut y = Vec::new();
    s.process(&x, &mut y);
    assert!(y.len() + s.latency() >= x.len() - 256);
    for k in 256..y.len() {
        assert!(
            (y[k] - x[k]).abs() < 1e-5,
            "sample {k}: {} vs {}",
            y[k],
            x[k]
        );
    }
}

#[test]
fn streaming_stub_is_chunking_invariant_and_attenuates_noise() {
    let x = noise(2 * SR as usize, 7, 0.25);
    let mut a = SpectralStubEngine::new();
    let mut whole = Vec::new();
    a.process_stream(&x, &mut whole);
    let mut b = SpectralStubEngine::new();
    let parts = chunked(&x, |c, o| b.process_stream(c, o));
    assert_eq!(whole, parts);
    // Steady state, after the noise tracker has settled.
    let tail = &whole[SR as usize..];
    let att_db = 20.0 * (rms(tail) / rms(&x[SR as usize..SR as usize + tail.len()])).log10();
    assert!(
        att_db < -0.5,
        "streaming stub should attenuate white noise, got {att_db:.2} dB"
    );
    assert!(whole.iter().all(|v| v.is_finite()));
}

#[test]
fn passthrough_pipeline_is_bit_exact_at_native_rate() {
    let x = noise(SR as usize, 1, 0.3);
    let mut p = StreamPipeline::new(Box::new(PassthroughEngine), SR, SR);
    assert_eq!(p.latency_ms(), 0.0);
    let y = chunked(&x, |c, o| p.process(c, o));
    assert_eq!(x, y);
}

#[test]
fn passthrough_model_name_resolves() {
    for name in ["passthrough", "none"] {
        let e = select_engine(name).expect("passthrough engine");
        assert_eq!(e.name(), "passthrough");
    }
}

/// Mismatched device rates (44.1 kHz mic → 48 kHz model → 44.1 kHz cable)
/// must come out at the output rate, finite, and chunking-invariant.
#[test]
fn pipeline_handles_mismatched_device_rates() {
    let x = noise(44_100, 11, 0.2);
    let mk = || StreamPipeline::new(Box::new(SpectralStubEngine::new()), 44_100, 16_000);
    let mut p = mk();
    let mut whole = Vec::new();
    p.process(&x, &mut whole);
    let mut q = mk();
    let parts = chunked(&x, |c, o| q.process(c, o));
    assert_eq!(whole, parts);
    let expected = 16_000.0 - (p.latency_ms() / 1000.0 * 16_000.0);
    assert!(
        (whole.len() as f64 - expected).abs() < 400.0,
        "got {} samples, expected ~{expected:.0}",
        whole.len()
    );
    assert!(whole.iter().all(|v| v.is_finite()));
    assert!(p.latency_ms() > 0.0 && p.latency_ms() < 20.0);
}

/// Bypassed, the output is the raw input delayed by the engine's stream
/// offset — the A side and B side are the same moment in time.
#[test]
fn bypass_outputs_time_aligned_raw_signal() {
    let x = noise(SR as usize, 5, 0.3);
    let mut p = StreamPipeline::new(Box::new(SpectralStubEngine::new()), SR, SR);
    p.set_bypass(true);
    // Let the 5 ms crossfade finish before comparing.
    let mut warm = Vec::new();
    p.process(&vec![0.0; 4800], &mut warm);
    let mut y = Vec::new();
    p.process(&x, &mut y);
    // Stub offset is 0, but output lags by whatever hasn't been emitted yet:
    // output k corresponds to stream input k, and 4800 warm-up samples precede x.
    let lead = 4800 - warm.len();
    for k in 0..(y.len() - lead) {
        assert_eq!(y[k + lead], x[k], "bypass sample {k} differs");
    }
    // The recorded pair stays aligned too.
    assert_eq!(p.last_raw().len(), p.last_denoised().len());
}

#[test]
fn bypass_toggle_crossfades_without_clicks() {
    let tone: Vec<f32> = (0..SR as usize)
        .map(|i| 0.5 * (std::f32::consts::TAU * 600.0 * i as f32 / SR as f32).sin())
        .collect();
    let mut p = StreamPipeline::new(Box::new(PassthroughEngine), SR, SR);
    let mut y = Vec::new();
    for (i, c) in tone.chunks(480).enumerate() {
        p.set_bypass(i % 10 < 5);
        p.process(c, &mut y);
    }
    // Passthrough on both sides: the crossfade must be invisible.
    assert_eq!(y.len(), tone.len());
    for (k, (a, b)) in y.iter().zip(&tone).enumerate() {
        assert!((a - b).abs() < 1e-6, "sample {k}: {a} vs {b}");
    }
}

/// The DeepFilterNet3 streaming path must equal the offline path exactly,
/// shifted by its stream offset: same model, same hops, no flush/trim between
/// chunks. This is the regression that made `denoise-live` re-flush the model
/// every 10 ms.
#[cfg(feature = "dfn")]
#[test]
fn dfn_streaming_matches_offline() {
    let x = noise(SR as usize, 9, 0.1);
    let mut off = select_engine("deepfilternet3").unwrap();
    let offline = off.process(&x, SR);

    let mut live = select_engine("deepfilternet3").unwrap();
    let offset = live.stream_offset();
    let streamed = chunked(&x, |c, o| live.process_stream(c, o));
    assert!(offset > 0 && offset < 4800, "offset {offset}");
    let n = streamed.len() - offset;
    for k in 0..n {
        assert!(
            (streamed[k + offset] - offline[k]).abs() < 1e-5,
            "sample {k}: streamed {} vs offline {}",
            streamed[k + offset],
            offline[k]
        );
    }
    let ms = live.stream_latency() as f64 / SR as f64 * 1000.0;
    assert!(
        ms < 50.0,
        "DFN3 algorithmic latency {ms:.1} ms blows the v0.3 budget"
    );
}
