//! Unit tests for the NullModel identity pass-through
//! (port of `tests/models/test_null_model.py`).

mod common;

use rfwhisper::constants::{DFN_FRAME_SAMPLES, NATIVE_DFN_SR_HZ};
use rfwhisper::models::{load, Model, NullModel};

/// NullModel exposes the canonical 48 kHz / 480-sample contract.
#[test]
fn test_null_model_attributes() {
    let m = NullModel;
    assert_eq!(m.sample_rate(), NATIVE_DFN_SR_HZ);
    assert_eq!(m.hop(), DFN_FRAME_SAMPLES);
}

/// Output must be bit-equal to the input on random 480-sample frames (A4 gate).
#[test]
fn test_null_model_identity_bit_equal() {
    let mut rng = common::TestRng::new(0xC0FFEE);
    let mut m = NullModel;
    for _ in 0..8 {
        let x: Vec<f32> = (0..DFN_FRAME_SAMPLES)
            .map(|_| rng.standard_normal() as f32)
            .collect();
        let y = m.process(&x).unwrap();
        // Bit-equal — compare raw bits, no tolerance.
        assert_eq!(y.len(), DFN_FRAME_SAMPLES);
        assert!(y.iter().zip(&x).all(|(a, b)| a.to_bits() == b.to_bits()));
    }
}

/// Wrong frame size must error — silent reshape would mask upstream bugs.
#[test]
fn test_null_model_rejects_wrong_shape() {
    let mut m = NullModel;
    let err = m.process(&vec![0.0f32; DFN_FRAME_SAMPLES - 1]).unwrap_err();
    assert!(err.0.contains("expects shape"));
}

/// `load("null")` is the public entry point and must return a working identity model.
#[test]
fn test_load_null_returns_null_model() {
    let mut m = load("null", false).unwrap();
    assert_eq!(m.sample_rate(), NATIVE_DFN_SR_HZ);
    assert_eq!(m.hop(), DFN_FRAME_SAMPLES);
    let x = vec![0.5f32; DFN_FRAME_SAMPLES];
    assert_eq!(m.process(&x).unwrap(), x);
}
