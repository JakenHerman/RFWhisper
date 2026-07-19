//! Synthetic *clean* signals — the other half of every mixed fixture.
//!
//! Issue #19 scopes `samples/clean/*.wav` as real on-air clips under Git LFS.
//! Those are still wanted (they are what a human listener judges), but a gate
//! that needs a multi-hundred-MB LFS pull to run is a gate contributors skip, and
//! a demo that needs one is a demo nobody reproduces. So the generators here
//! stand in: `synth` + `signal` together produce a complete clean/noisy/reference
//! triple from nothing but a seed, which is what A1 actually measures against.
//!
//! These are *plausible*, not authentic. The CW envelope, FT8 tone spacing, and
//! SSB passband are right; the "voice" is a formant model, not a person. Read a
//! gate result as "the denoiser behaves correctly on a signal shaped like this",
//! and keep real clips (#47, #68) for judging how it sounds on air.

use crate::dsp::filter::butter_bandpass;
use crate::dsp::{require_positive, DspError};
use crate::samples::rng::SeededRng;
use crate::samples::synth::PEAK;

/// SSB communications passband — the 300 Hz–2.7 kHz a rig actually passes.
const SSB_BAND_HZ: (f64, f64) = (300.0, 2_700.0);
/// Narrowband-FM recovered-audio band; wider top end than SSB.
const FM_BAND_HZ: (f64, f64) = (300.0, 3_400.0);

fn normalize(mut x: Vec<f64>) -> Vec<f64> {
    let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    if peak <= 0.0 {
        return x;
    }
    let scale = PEAK / peak;
    for v in &mut x {
        *v *= scale;
    }
    x
}

fn n_samples(sr: u32, duration_s: f64) -> Result<usize, DspError> {
    if sr == 0 {
        return Err(DspError::new("sr must be positive"));
    }
    require_positive(duration_s, "duration_s")?;
    let n = (sr as f64 * duration_s).round() as i64;
    if n <= 0 {
        return Err(DspError::new("duration_s is too short at this sr"));
    }
    Ok(n as usize)
}

/// A keyed CW clip plus the sample index of every key-down edge.
///
/// The onsets are the reason this returns a struct: A3 ([`crate::dsp::metrics::
/// keying_onset_rms`]) compares per-onset RMS between raw and denoised audio, and
/// recovering those edges by thresholding the waveform would re-introduce exactly
/// the ambiguity the gate is trying to measure.
#[derive(Debug, Clone)]
pub struct CwClip {
    pub samples: Vec<f64>,
    pub onsets: Vec<usize>,
}

/// Keyed CW at `wpm` using the PARIS timing standard.
///
/// The element envelope is a raised cosine with a `rise_ms` edge — hard keying
/// would splatter across the passband and make the A3 onset measurement a
/// property of the fixture's clicks rather than of the denoiser.
pub fn cw(
    sr: u32,
    duration_s: f64,
    tone_hz: f64,
    wpm: f64,
    rise_ms: f64,
    seed: u64,
) -> Result<CwClip, DspError> {
    if !(tone_hz > 0.0 && tone_hz < sr as f64 / 2.0) {
        return Err(DspError::new(format!(
            "tone_hz {tone_hz} must be in (0, {})",
            sr as f64 / 2.0
        )));
    }
    require_positive(wpm, "wpm")?;
    require_positive(rise_ms, "rise_ms")?;
    let n = n_samples(sr, duration_s)?;
    let mut rng = SeededRng::new(seed);

    // PARIS: one dit = 1.2 s / wpm.
    let dit = 1.2 / wpm;
    let rise = rise_ms / 1_000.0;
    if rise * 2.0 >= dit {
        return Err(DspError::new(format!(
            "rise_ms {rise_ms} is too long for {wpm} WPM (dit is {:.1} ms)",
            dit * 1_000.0
        )));
    }

    let mut samples = vec![0.0f64; n];
    let mut onsets = Vec::new();
    let mut t = 0.0f64;
    // "CQ CQ DE ..." in dits/dahs; a fixed pattern keeps clips comparable, and the
    // seed only jitters the inter-word gaps (an operator's fist is not a metronome).
    let pattern: [u8; 12] = [3, 1, 3, 1, 3, 3, 1, 3, 0, 3, 1, 1];
    let mut idx = 0usize;
    while t < duration_s {
        let units = pattern[idx % pattern.len()];
        idx += 1;
        if units == 0 {
            // Word gap: 7 dits, jittered.
            t += dit * rng.uniform_in(5.0, 8.0);
            continue;
        }
        let element = dit * units as f64;
        let start = (t * sr as f64).round() as usize;
        if start >= n {
            break;
        }
        let len = ((element * sr as f64).round() as usize).min(n - start);
        if len > 0 {
            onsets.push(start);
            let rise_n = ((rise * sr as f64).round() as usize).max(1).min(len / 2);
            for j in 0..len {
                // Raised-cosine attack and release, flat key-down between.
                let env = if j < rise_n {
                    0.5 * (1.0 - (std::f64::consts::PI * j as f64 / rise_n as f64).cos())
                } else if j >= len - rise_n {
                    let k = len - j;
                    0.5 * (1.0 - (std::f64::consts::PI * k as f64 / rise_n as f64).cos())
                } else {
                    1.0
                };
                let tt = (start + j) as f64 / sr as f64;
                samples[start + j] += env * (std::f64::consts::TAU * tone_hz * tt).sin();
            }
        }
        // Inter-element gap: 1 dit.
        t += element + dit;
    }
    Ok(CwClip {
        samples: normalize(samples),
        onsets,
    })
}

/// Voice-like audio: three drifting formants under a syllabic envelope.
///
/// Not speech — a source-filter caricature with the right long-term spectrum and
/// the right amplitude rhythm, band-limited to `band`. Enough for a denoiser to
/// have something structured to preserve; not enough for PESQ to mean what it
/// means on real speech (see the module note).
fn voice_like(sr: u32, duration_s: f64, band: (f64, f64), seed: u64) -> Result<Vec<f64>, DspError> {
    let n = n_samples(sr, duration_s)?;
    let mut rng = SeededRng::new(seed);
    let nyquist = sr as f64 / 2.0;
    if band.1 >= nyquist {
        return Err(DspError::new(format!(
            "band top {} Hz must be below Nyquist {nyquist}",
            band.1
        )));
    }

    // Glottal pulse train, jittered around a speaker's mean pitch.
    let f0 = rng.uniform_in(95.0, 135.0);
    let jitter_hz = rng.uniform_in(0.4, 1.1);
    let jitter_phase = rng.uniform_in(0.0, std::f64::consts::TAU);
    // Syllable rate: ~4 Hz is conversational.
    let syl_hz = rng.uniform_in(3.0, 5.0);
    let syl_phase = rng.uniform_in(0.0, std::f64::consts::TAU);

    let formants: [(f64, f64); 3] = [
        (rng.uniform_in(500.0, 800.0), 1.0),
        (rng.uniform_in(1_100.0, 1_600.0), 0.55),
        (rng.uniform_in(2_200.0, 2_600.0), 0.3),
    ];
    let drift_hz: Vec<f64> = (0..3).map(|_| rng.uniform_in(0.3, 0.9)).collect();

    let mut out = vec![0.0f64; n];
    for (i, o) in out.iter_mut().enumerate() {
        let t = i as f64 / sr as f64;
        // Syllabic envelope: never fully closes, so the denoiser always has signal.
        let syl =
            0.15 + 0.85 * (0.5 * (1.0 - (std::f64::consts::TAU * syl_hz * t + syl_phase).cos()));
        let pitch =
            f0 * (1.0 + 0.03 * (std::f64::consts::TAU * jitter_hz * t + jitter_phase).sin());
        let mut v = 0.0;
        for (k, (center, amp)) in formants.iter().enumerate() {
            // Formants drift a little; a static one sounds like an organ, not a voice.
            let f = center * (1.0 + 0.05 * (std::f64::consts::TAU * drift_hz[k] * t).sin());
            // Harmonic nearest the formant center, so partials stay on the pitch grid.
            let harmonic = (f / pitch).round().max(1.0);
            v += amp * (std::f64::consts::TAU * harmonic * pitch * t).sin();
        }
        *o = syl * v;
    }

    butter_bandpass(4, band.0, band.1, sr)?.filter(&mut out);
    Ok(normalize(out))
}

/// SSB voice in the 300 Hz–2.7 kHz communications passband.
pub fn ssb(sr: u32, duration_s: f64, seed: u64) -> Result<Vec<f64>, DspError> {
    voice_like(sr, duration_s, SSB_BAND_HZ, seed)
}

/// Recovered narrowband-FM audio (VHF): same voice model, wider passband.
pub fn vhf_fm(sr: u32, duration_s: f64, seed: u64) -> Result<Vec<f64>, DspError> {
    voice_like(sr, duration_s, FM_BAND_HZ, seed)
}

/// FT8-shaped 8-GFSK: 79 symbols, 6.25 Hz spacing, 0.16 s each.
///
/// Tone *timing and spacing* are per the FT8 specification, so a decoder's
/// front-end sees the right thing; the symbol values are seeded noise rather than
/// a real Costas-framed, LDPC-coded message. A2 (#21) counts decodes and
/// therefore needs genuine encoded messages — this fixture is for the A1/A6 path,
/// where what matters is that narrow tones survive denoising intact.
pub fn ft8(sr: u32, duration_s: f64, base_hz: f64, seed: u64) -> Result<Vec<f64>, DspError> {
    const SYMBOL_S: f64 = 0.16;
    const TONE_SPACING_HZ: f64 = 6.25;
    const N_TONES: u32 = 8;

    let n = n_samples(sr, duration_s)?;
    let top = base_hz + TONE_SPACING_HZ * f64::from(N_TONES);
    if !(base_hz > 0.0 && top < sr as f64 / 2.0) {
        return Err(DspError::new(format!(
            "FT8 tones {base_hz}–{top} Hz must fit under Nyquist {}",
            sr as f64 / 2.0
        )));
    }
    let mut rng = SeededRng::new(seed);

    let mut out = vec![0.0f64; n];
    // Continuous phase across symbol boundaries — a phase discontinuity would
    // splatter energy into neighboring bins and defeat the point of the fixture.
    let mut phase = 0.0f64;
    let mut i = 0usize;
    while i < n {
        let tone = (rng.uniform() * f64::from(N_TONES))
            .floor()
            .min(f64::from(N_TONES - 1));
        let f = base_hz + TONE_SPACING_HZ * tone;
        let len = ((SYMBOL_S * sr as f64).round() as usize).min(n - i);
        let dphase = std::f64::consts::TAU * f / sr as f64;
        for o in out[i..i + len].iter_mut() {
            *o = phase.sin();
            phase = (phase + dphase) % std::f64::consts::TAU;
        }
        i += len;
    }
    Ok(normalize(out))
}
