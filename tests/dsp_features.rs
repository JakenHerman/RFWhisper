//! Tests for framing, Hann windows, overlap-add, and STFT frame stacking
//! (port of `tests/dsp/test_features.py`).

mod common;

use rfwhisper::dsp::features::{
    hann_window, stft_frames, FrameBuffer, HOP_16K, HOP_48K, WIN_16K, WIN_48K,
};

/// `hann_window` must match the periodic (`fftbins=True`) reference formula,
/// including the length-1 special case (SciPy convention).
#[test]
fn test_hann_matches_scipy_fftbins() {
    assert_eq!(hann_window(1).unwrap(), vec![1.0]);
    for n in [100usize, 320, 960] {
        let got = hann_window(n).unwrap();
        assert_eq!(got.len(), n);
        for (i, v) in got.iter().enumerate() {
            let want = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos());
            assert!((v - want).abs() <= 1e-15, "n={n} i={i}");
        }
        // Periodic window: first sample is 0, and w[n/2] == 1 for even n.
        assert_eq!(got[0], 0.0);
        if n % 2 == 0 {
            assert!((got[n / 2] - 1.0).abs() <= 1e-15);
        }
    }
    assert!(hann_window(0).is_err());
}

/// Hop / window constants are 10 ms / 20 ms at their declared rates.
#[test]
fn test_constants_ten_ms_hop() {
    assert_eq!(HOP_48K, 480);
    assert_eq!(WIN_48K, 2 * HOP_48K);
    assert_eq!(HOP_16K, 160);
    assert_eq!(WIN_16K, 2 * HOP_16K);
}

/// WOLA gain: two 50 %-overlapped Hann windows sum to 1 (linear COLA).
#[test]
fn test_ola_hann_half_overlap_sums_to_unity() {
    let w = hann_window(WIN_48K).unwrap();
    let hop = HOP_48K;
    assert_eq!(hop * 2, WIN_48K);
    for i in hop..WIN_48K {
        let s = w[i] + w[i - hop];
        assert!((s - 1.0).abs() <= 1e-6, "i={i}: {s}");
    }
}

/// Push → next_frame → overlap_add reconstructs a 1 kHz sine after the WOLA warmup.
#[test]
fn test_frame_buffer_sine_roundtrip_steady_state() {
    let sr = 48_000usize;
    let f0 = 1_000.0f64;
    let n = sr; // 1 s
    let x: Vec<f64> = (0..n)
        .map(|i| (2.0 * std::f64::consts::PI * f0 * i as f64 / sr as f64).sin())
        .collect();

    let mut fb = FrameBuffer::new(WIN_48K, HOP_48K).unwrap();
    let mut out: Vec<f64> = Vec::new();

    let feed = |fb: &mut FrameBuffer, out: &mut Vec<f64>, sig: &[f64]| {
        let hop = HOP_48K;
        let mut padded = sig.to_vec();
        let pad = (hop - (padded.len() % hop)) % hop;
        padded.extend(std::iter::repeat(0.0).take(pad));
        for chunk in padded.chunks(hop) {
            fb.push(chunk).unwrap();
            while fb.ready() {
                let frame = fb.next_frame().unwrap();
                out.extend(fb.overlap_add(&frame).unwrap());
            }
        }
    };

    feed(&mut fb, &mut out, &x);
    let warmup = WIN_48K - HOP_48K;
    // One extra window of synthesis tail beyond input length (COLA drain).
    let need = warmup + x.len() + WIN_48K;
    let zeros = vec![0.0; HOP_48K];
    while out.len() < need {
        fb.push(&zeros).unwrap();
        while fb.ready() {
            let frame = fb.next_frame().unwrap();
            out.extend(fb.overlap_add(&frame).unwrap());
        }
    }

    // With F = (N - W) / H + 1 frames, COLA emits F * H = N - (W - H) steady-state
    // samples that match the input; the last (W - H) samples need further tail flush
    // beyond this acceptance test's scope.
    let steady = x.len() - (WIN_48K - HOP_48K);
    for i in 0..steady {
        let (got, want) = (out[warmup + i], x[i]);
        assert!(
            (got - want).abs() <= 5e-3 + 1e-3 * want.abs(),
            "sample {i}: got {got}, want {want}"
        );
    }
}

/// `stft_frames` must reject non-positive sizes and `hop > win_size`.
#[test]
fn test_stft_frames_rejects_invalid_params() {
    let x = vec![0.0; 100];
    let err = stft_frames(&x, 0, 10).unwrap_err();
    assert!(err.0.contains("win_size and hop must be positive"));
    let err = stft_frames(&x, 32, 0).unwrap_err();
    assert!(err.0.contains("win_size and hop must be positive"));
    let err = stft_frames(&x, 32, 64).unwrap_err();
    assert!(err.0.contains("hop must not exceed win_size"));
}

/// `stft_frames` shape and contents match a hand-computed sliding window × sqrt-Hann.
#[test]
fn test_stft_frames_shape_and_matches_manual() {
    let mut rng = common::TestRng::new(0xD5F0);
    let x = rng.standard_normal_vec(5_000);
    let (win, hop) = (320usize, 160usize);
    let frames = stft_frames(&x, win, hop).unwrap();
    let w_sqrt: Vec<f64> = hann_window(win).unwrap().iter().map(|v| v.sqrt()).collect();
    let n_frames = 1 + (x.len() - win) / hop;
    assert_eq!(frames.len(), n_frames);
    assert!(frames.iter().all(|f| f.len() == win));
    for j in 0..win {
        let want = x[3 * hop + j] * w_sqrt[j];
        assert!((frames[3][j] - want).abs() <= 1e-15);
    }
}

/// Input shorter than one window yields no frames.
#[test]
fn test_stft_frames_short_input_is_empty() {
    let x = vec![1.0; 100];
    assert!(stft_frames(&x, 320, 160).unwrap().is_empty());
}
