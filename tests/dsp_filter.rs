//! Butterworth section tests: passband transparency, stopband rejection, validation.

use rfwhisper::dsp::filter::{butter_bandpass, butter_highpass, butter_lowpass};

const SR: u32 = 48_000;

/// RMS of a filtered unit sine at `f`, after discarding the filter's startup transient.
fn response_rms(sos: &rfwhisper::dsp::filter::Sos, f: f64) -> f64 {
    let n = SR as usize; // 1 s is many periods at every frequency under test
    let mut x: Vec<f64> = (0..n)
        .map(|i| (std::f64::consts::TAU * f * i as f64 / SR as f64).sin())
        .collect();
    sos.filter(&mut x);
    // Skip the first 100 ms: the sections start from zero state.
    let tail = &x[SR as usize / 10..];
    (tail.iter().map(|v| v * v).sum::<f64>() / tail.len() as f64).sqrt()
}

/// Unit sine RMS is 1/sqrt(2); express a section's response relative to that.
fn gain_db(sos: &rfwhisper::dsp::filter::Sos, f: f64) -> f64 {
    20.0 * (response_rms(sos, f) / std::f64::consts::FRAC_1_SQRT_2).log10()
}

#[test]
fn lowpass_passes_below_and_rejects_above() {
    let sos = butter_lowpass(4, 1_000.0, SR).unwrap();
    assert!(
        gain_db(&sos, 200.0).abs() < 0.5,
        "200 Hz should pass a 1 kHz lowpass untouched, got {} dB",
        gain_db(&sos, 200.0)
    );
    // 4th order = 24 dB/octave; two octaves up is ~48 dB down.
    assert!(
        gain_db(&sos, 4_000.0) < -30.0,
        "4 kHz should be well rejected, got {} dB",
        gain_db(&sos, 4_000.0)
    );
}

#[test]
fn lowpass_is_minus_3db_at_cutoff() {
    let sos = butter_lowpass(4, 1_000.0, SR).unwrap();
    let g = gain_db(&sos, 1_000.0);
    assert!(
        (g - -3.0).abs() < 0.6,
        "Butterworth cutoff is the -3 dB point, got {g} dB"
    );
}

#[test]
fn highpass_mirrors_the_lowpass() {
    let sos = butter_highpass(4, 1_000.0, SR).unwrap();
    assert!(gain_db(&sos, 4_000.0).abs() < 0.5);
    assert!(gain_db(&sos, 200.0) < -30.0);
}

#[test]
fn bandpass_passes_only_its_band() {
    let sos = butter_bandpass(4, 300.0, 2_700.0, SR).unwrap();
    assert!(
        gain_db(&sos, 1_000.0).abs() < 1.0,
        "mid-band should be flat, got {} dB",
        gain_db(&sos, 1_000.0)
    );
    assert!(gain_db(&sos, 50.0) < -25.0, "50 Hz should be rejected");
    assert!(gain_db(&sos, 12_000.0) < -25.0, "12 kHz should be rejected");
}

#[test]
fn cascade_section_count_matches_order() {
    assert_eq!(butter_lowpass(4, 1_000.0, SR).unwrap().len(), 2);
    assert_eq!(butter_lowpass(2, 1_000.0, SR).unwrap().len(), 1);
    // Bandpass is highpass ∘ lowpass, so both legs contribute.
    assert_eq!(butter_bandpass(4, 300.0, 3_000.0, SR).unwrap().len(), 4);
}

#[test]
fn rejects_invalid_design_parameters() {
    // Odd order: this designer only emits conjugate pairs.
    assert!(butter_lowpass(3, 1_000.0, SR).is_err());
    assert!(butter_lowpass(0, 1_000.0, SR).is_err());
    // Cutoff at or above Nyquist has no bilinear pre-warp.
    assert!(butter_lowpass(4, 24_000.0, SR).is_err());
    assert!(butter_lowpass(4, 30_000.0, SR).is_err());
    assert!(butter_lowpass(4, 0.0, SR).is_err());
    assert!(butter_lowpass(4, 1_000.0, 0).is_err());
    // Inverted band.
    assert!(butter_bandpass(4, 3_000.0, 300.0, SR).is_err());
}

#[test]
fn filtering_is_stateless_between_calls() {
    let sos = butter_lowpass(4, 1_000.0, SR).unwrap();
    let make = || -> Vec<f64> {
        (0..1_000)
            .map(|i| (std::f64::consts::TAU * 500.0 * i as f64 / SR as f64).sin())
            .collect()
    };
    let (mut a, mut b) = (make(), make());
    sos.filter(&mut a);
    sos.filter(&mut b);
    assert_eq!(a, b, "each filter() call must start from zero state");
}
