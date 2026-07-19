//! Fixture-generator tests (#19): determinism, exact SNR, headroom, and the
//! signal-shape properties the gates depend on.

use rfwhisper::samples::{
    self, render_mix, render_preset, signal, synth, NoiseKind, SignalKind, PRESETS,
};

const SR: u32 = 48_000;
const DUR: f64 = 2.0;

fn peak(x: &[f64]) -> f64 {
    x.iter().fold(0.0f64, |m, v| m.max(v.abs()))
}

fn power(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum()
}

// --- Determinism ------------------------------------------------------------------------

/// The whole point of the module: a seed pins the samples, so a gate failure is
/// always the denoiser moving and never the fixture drifting.
#[test]
fn generators_are_deterministic_for_a_fixed_seed() {
    for k in [
        NoiseKind::Powerline,
        NoiseKind::Inverter,
        NoiseKind::Vdsl,
        NoiseKind::Qrn,
    ] {
        let a = k.render(SR, DUR, 7).unwrap();
        let b = k.render(SR, DUR, 7).unwrap();
        assert_eq!(a, b, "{} is not deterministic", k.as_str());
    }
    for k in [
        SignalKind::Ssb,
        SignalKind::Cw,
        SignalKind::Ft8,
        SignalKind::VhfFm,
    ] {
        let a = k.render(SR, DUR, 7).unwrap();
        let b = k.render(SR, DUR, 7).unwrap();
        assert_eq!(a, b, "{} is not deterministic", k.as_str());
    }
}

#[test]
fn different_seeds_produce_different_audio() {
    for k in [
        NoiseKind::Powerline,
        NoiseKind::Inverter,
        NoiseKind::Vdsl,
        NoiseKind::Qrn,
    ] {
        let a = k.render(SR, DUR, 1).unwrap();
        let b = k.render(SR, DUR, 2).unwrap();
        assert_ne!(a, b, "{} ignores its seed", k.as_str());
    }
}

#[test]
fn generators_are_peak_normalised_and_finite() {
    for k in [
        NoiseKind::Powerline,
        NoiseKind::Inverter,
        NoiseKind::Vdsl,
        NoiseKind::Qrn,
    ] {
        let x = k.render(SR, DUR, 0).unwrap();
        assert_eq!(x.len(), (SR as f64 * DUR) as usize);
        assert!(
            x.iter().all(|v| v.is_finite()),
            "{}: non-finite",
            k.as_str()
        );
        assert!(
            (peak(&x) - synth::PEAK).abs() < 1e-9,
            "{} should peak at {}, got {}",
            k.as_str(),
            synth::PEAK,
            peak(&x)
        );
    }
}

// --- mix --------------------------------------------------------------------------------

/// `mix` is the one function whose contract is a number, not a shape.
#[test]
fn mix_hits_the_requested_snr_exactly() {
    let clean = signal::ssb(SR, DUR, 0).unwrap();
    let noise = synth::powerline_buzz(SR, DUR, 60.0, 30, 1).unwrap();
    for target in [20.0, 6.0, 0.0, -6.0, -24.0] {
        let noisy = synth::mix(&clean, &noise, target).unwrap();
        let residual: Vec<f64> = noisy.iter().zip(&clean).map(|(n, c)| n - c).collect();
        let measured = 10.0 * (power(&clean) / power(&residual)).log10();
        assert!(
            (measured - target).abs() < 1e-6,
            "asked for {target} dB SNR, measured {measured} dB"
        );
    }
}

#[test]
fn mix_leaves_the_clean_reference_sample_aligned() {
    let clean = signal::ssb(SR, DUR, 0).unwrap();
    let noise = synth::vdsl_hash(SR, DUR, 1).unwrap();
    let noisy = synth::mix(&clean, &noise, 0.0).unwrap();
    assert_eq!(noisy.len(), clean.len());
    // Subtracting the reference must leave exactly the scaled noise, no shift.
    let residual: Vec<f64> = noisy.iter().zip(&clean).map(|(n, c)| n - c).collect();
    let scale = residual[1_000] / noise[1_000];
    for i in [0, 500, 5_000, 20_000] {
        assert!(
            (residual[i] - scale * noise[i]).abs() < 1e-9,
            "residual is not a pure scaling of the noise at sample {i}"
        );
    }
}

#[test]
fn mix_rejects_mismatched_or_silent_input() {
    let clean = signal::ssb(SR, 1.0, 0).unwrap();
    let short = signal::ssb(SR, 0.5, 0).unwrap();
    assert!(synth::mix(&clean, &short, 0.0).is_err(), "length mismatch");
    let silence = vec![0.0; clean.len()];
    assert!(synth::mix(&clean, &silence, 0.0).is_err(), "zero noise");
    assert!(synth::mix(&silence, &clean, 0.0).is_err(), "zero signal");
}

// --- render_mix headroom ----------------------------------------------------------------

/// Regression: at preset SNRs the raw mix peaked at ~13x full scale and clipped to
/// hash on write. Both channels are now scaled by one shared factor.
#[test]
fn render_mix_never_clips_at_preset_snrs() {
    for p in &PRESETS {
        let fx = render_preset(p, SR, DUR, 0).unwrap();
        assert!(
            fx.noisy_peak <= synth::PEAK + 1e-9,
            "{} peaks at {} — would clip on write",
            p.name,
            fx.noisy_peak
        );
        assert!(peak(&fx.clean) <= synth::PEAK + 1e-9, "{}", p.name);
    }
}

/// Headroom scaling is uniform, so it must not move the SNR the caller asked for.
#[test]
fn headroom_scaling_preserves_snr() {
    for p in &PRESETS {
        let fx = render_preset(p, SR, DUR, 0).unwrap();
        let residual: Vec<f64> = fx.noisy.iter().zip(&fx.clean).map(|(n, c)| n - c).collect();
        let measured = 10.0 * (power(&fx.clean) / power(&residual)).log10();
        assert!(
            (measured - p.snr_db).abs() < 1e-6,
            "{}: asked {} dB, measured {measured} dB after headroom scaling",
            p.name,
            p.snr_db
        );
        assert!(
            fx.headroom_scale > 0.0 && fx.headroom_scale <= 1.0,
            "{}: headroom_scale {} out of range",
            p.name,
            fx.headroom_scale
        );
    }
}

#[test]
fn every_preset_resolves_by_name() {
    for p in &PRESETS {
        let found = samples::preset(p.name).expect("preset lookup");
        assert_eq!(found.name, p.name);
    }
    assert!(samples::preset("no_such_preset").is_none());
}

// --- signal shapes ----------------------------------------------------------------------

/// A3 (#3) measures per-onset RMS, so `cw` has to hand back the key-down edges.
#[test]
fn cw_reports_keying_onsets_in_order() {
    let clip = signal::cw(SR, 4.0, 600.0, 20.0, 5.0, 0).unwrap();
    assert!(
        clip.onsets.len() >= 8,
        "4 s at 20 WPM should key more than {} elements",
        clip.onsets.len()
    );
    assert!(
        clip.onsets.windows(2).all(|w| w[0] < w[1]),
        "onsets must be strictly increasing"
    );
    assert!(*clip.onsets.last().unwrap() < clip.samples.len());
    // Every onset should sit at a rising edge: quiet just before, loud just after.
    let rms = |r: &[f64]| (r.iter().map(|v| v * v).sum::<f64>() / r.len() as f64).sqrt();
    let win = SR as usize / 200; // 5 ms
    for &o in clip.onsets.iter().skip(1) {
        if o < win || o + 2 * win >= clip.samples.len() {
            continue;
        }
        let before = rms(&clip.samples[o - win..o]);
        let after = rms(&clip.samples[o + win..o + 2 * win]);
        assert!(
            after > before,
            "onset at {o} is not a rising edge ({before} -> {after})"
        );
    }
}

#[test]
fn cw_rejects_a_rise_time_longer_than_a_dit() {
    // At 60 WPM a dit is 20 ms; a 50 ms rise cannot fit inside it.
    assert!(signal::cw(SR, 1.0, 600.0, 60.0, 50.0, 0).is_err());
}

/// SSB has to land inside the communications passband, or "denoised speech"
/// measurements are really measuring out-of-band junk.
#[test]
fn ssb_energy_sits_in_the_communications_passband() {
    let x = signal::ssb(SR, DUR, 0).unwrap();
    let in_band = band_power(&x, SR, 300.0, 2_700.0);
    let total = power(&x);
    assert!(
        in_band / total > 0.9,
        "only {:.1}% of SSB energy is in 300-2700 Hz",
        100.0 * in_band / total
    );
}

#[test]
fn ft8_energy_sits_in_its_narrow_tone_block() {
    let x = signal::ft8(SR, DUR, 1_500.0, 0).unwrap();
    // 8 tones at 6.25 Hz spacing = a 50 Hz block; allow for keying sidebands.
    let in_band = band_power(&x, SR, 1_400.0, 1_650.0);
    assert!(
        in_band / power(&x) > 0.9,
        "FT8 tones leak outside their block: {:.1}% in band",
        100.0 * in_band / power(&x)
    );
}

#[test]
fn powerline_buzz_is_a_harmonic_comb_of_its_fundamental() {
    let x = synth::powerline_buzz(SR, DUR, 60.0, 30, 0).unwrap();
    // Energy should concentrate near multiples of 60 Hz, not between them.
    let on_harmonic: f64 = (1..=20)
        .map(|k| band_power(&x, SR, 60.0 * k as f64 - 4.0, 60.0 * k as f64 + 4.0))
        .sum();
    let off_harmonic: f64 = (1..=20)
        .map(|k| band_power(&x, SR, 60.0 * k as f64 + 20.0, 60.0 * k as f64 + 28.0))
        .sum();
    assert!(
        on_harmonic > 10.0 * off_harmonic,
        "comb is not sharp: {on_harmonic} on-harmonic vs {off_harmonic} off-harmonic"
    );
}

// --- validation -------------------------------------------------------------------------

#[test]
fn generators_reject_nonsense_parameters() {
    assert!(synth::powerline_buzz(0, DUR, 60.0, 30, 0).is_err(), "sr 0");
    assert!(
        synth::powerline_buzz(SR, 0.0, 60.0, 30, 0).is_err(),
        "dur 0"
    );
    assert!(
        synth::powerline_buzz(SR, -1.0, 60.0, 30, 0).is_err(),
        "dur<0"
    );
    assert!(
        synth::powerline_buzz(SR, f64::NAN, 60.0, 30, 0).is_err(),
        "NaN"
    );
    assert!(synth::powerline_buzz(SR, DUR, 0.0, 30, 0).is_err(), "f0 0");
    assert!(
        synth::powerline_buzz(SR, DUR, 60.0, 0, 0).is_err(),
        "0 harmonics"
    );
    assert!(
        synth::solar_inverter(SR, DUR, 0.0, 50.0, 0).is_err(),
        "tick 0"
    );
    assert!(
        synth::solar_inverter(SR, DUR, 120.0, 0.0, 0).is_err(),
        "Q 0"
    );
    assert!(synth::atmospheric_qrn(SR, DUR, 0.0, 0).is_err(), "rate 0");
    // 800 Hz sample rate leaves no room for a 300 Hz - 0.45*Nyquist band.
    assert!(synth::vdsl_hash(800, DUR, 0).is_err(), "sr too low");
}

#[test]
fn kind_names_round_trip() {
    for k in [
        SignalKind::Ssb,
        SignalKind::Cw,
        SignalKind::Ft8,
        SignalKind::VhfFm,
    ] {
        assert_eq!(SignalKind::parse(k.as_str()), Some(k));
    }
    for k in [
        NoiseKind::Powerline,
        NoiseKind::Inverter,
        NoiseKind::Vdsl,
        NoiseKind::Qrn,
    ] {
        assert_eq!(NoiseKind::parse(k.as_str()), Some(k));
    }
    assert_eq!(SignalKind::parse("vhf_fm"), Some(SignalKind::VhfFm));
    assert!(SignalKind::parse("nope").is_none());
    assert!(NoiseKind::parse("nope").is_none());
}

#[test]
fn render_mix_propagates_generator_errors() {
    assert!(
        render_mix(SignalKind::Ssb, NoiseKind::Powerline, 0.0, SR, 0.0, 0).is_err(),
        "zero duration must not silently produce an empty fixture"
    );
}

// --- helpers ----------------------------------------------------------------------------

/// Energy of `x` between `lo` and `hi` Hz, via a Goertzel-free DFT over the band.
///
/// A full FFT would be faster, but this keeps the test independent of the crate's
/// own framing code — a fixture test that leans on `dsp::features` would go green
/// when both sides drift together.
fn band_power(x: &[f64], sr: u32, lo: f64, hi: f64) -> f64 {
    let n = x.len();
    let bin_hz = sr as f64 / n as f64;
    let k_lo = (lo / bin_hz).ceil() as usize;
    let k_hi = (hi / bin_hz).floor() as usize;
    let mut total = 0.0;
    for k in k_lo..=k_hi.min(n / 2) {
        let w = std::f64::consts::TAU * k as f64 / n as f64;
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, v) in x.iter().enumerate() {
            let ph = w * i as f64;
            re += v * ph.cos();
            im -= v * ph.sin();
        }
        // Parseval: both halves of the spectrum carry the energy.
        total += 2.0 * (re * re + im * im) / n as f64;
    }
    total
}
