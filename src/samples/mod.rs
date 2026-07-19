//! Fixture synthesis: deterministic clean signals, RFI noise, and mixed pairs.
//!
//! Everything the acceptance gates measure against is generated here from a seed,
//! so a fresh clone can run every gate — and reproduce every number in the
//! release notes — without pulling a byte of LFS.
//!
//! ```text
//! signal::ssb(…)  ──┐
//!                   ├── synth::mix(clean, noise, snr_db) ──▶ noisy
//! synth::powerline_buzz(…) ──┘                              clean (reference)
//! ```

pub mod rng;
pub mod signal;
pub mod synth;

use std::path::Path;

use crate::dsp::DspError;

/// A clean-signal generator by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    Ssb,
    Cw,
    Ft8,
    VhfFm,
}

/// An RFI noise generator by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseKind {
    Powerline,
    Inverter,
    Vdsl,
    Qrn,
    White,
}

impl SignalKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            // `speech` is the pre-reconciliation CLI name for the SSB voice model.
            "ssb" | "speech" => Some(Self::Ssb),
            "cw" => Some(Self::Cw),
            "ft8" => Some(Self::Ft8),
            "vhf-fm" | "vhf_fm" => Some(Self::VhfFm),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ssb => "ssb",
            Self::Cw => "cw",
            Self::Ft8 => "ft8",
            Self::VhfFm => "vhf-fm",
        }
    }

    /// Render this signal. CW keying onsets are discarded here; call
    /// [`signal::cw`] directly when A3 needs them.
    pub fn render(self, sr: u32, duration_s: f64, seed: u64) -> Result<Vec<f64>, DspError> {
        match self {
            Self::Ssb => signal::ssb(sr, duration_s, seed),
            Self::Cw => signal::cw(sr, duration_s, 600.0, 20.0, 5.0, seed).map(|c| c.samples),
            Self::Ft8 => signal::ft8(sr, duration_s, 1_500.0, seed),
            Self::VhfFm => signal::vhf_fm(sr, duration_s, seed),
        }
    }
}

impl NoiseKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "powerline" => Some(Self::Powerline),
            "inverter" => Some(Self::Inverter),
            "vdsl" => Some(Self::Vdsl),
            // `impulses` is the pre-reconciliation CLI name; CONTRIBUTING tells
            // contributors to paste `samples synth` lines into PRs, so the old
            // spelling has to keep resolving.
            "qrn" | "impulses" => Some(Self::Qrn),
            "white" => Some(Self::White),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Powerline => "powerline",
            Self::Inverter => "inverter",
            Self::Vdsl => "vdsl",
            Self::Qrn => "qrn",
            Self::White => "white",
        }
    }

    pub fn render(self, sr: u32, duration_s: f64, seed: u64) -> Result<Vec<f64>, DspError> {
        match self {
            Self::Powerline => synth::powerline_buzz(sr, duration_s, 60.0, 30, seed),
            Self::Inverter => synth::solar_inverter(sr, duration_s, 120.0, 50.0, seed),
            Self::Vdsl => synth::vdsl_hash(sr, duration_s, seed),
            Self::Qrn => synth::atmospheric_qrn(sr, duration_s, 2.0, seed),
            Self::White => synth::white(sr, duration_s, seed),
        }
    }
}

/// A named clean+noise+SNR combination.
///
/// The names and SNRs are the ones A1 (#20) asserts on, in S-units: `s3_s7` reads
/// "an S3 signal under an S7 noise floor", i.e. 4 S-units down — and one S-unit is
/// 6 dB, so that is the −24 dB case. These are the presets the release notes and
/// the demo script quote, so changing an SNR here moves a published number.
#[derive(Debug, Clone, Copy)]
pub struct Preset {
    pub name: &'static str,
    pub signal: SignalKind,
    pub noise: NoiseKind,
    pub snr_db: f64,
}

pub const PRESETS: [Preset; 6] = [
    Preset {
        name: "ssb_powerline_s3_s7",
        signal: SignalKind::Ssb,
        noise: NoiseKind::Powerline,
        snr_db: -24.0,
    },
    Preset {
        name: "ssb_inverter_s5_s9",
        signal: SignalKind::Ssb,
        noise: NoiseKind::Inverter,
        snr_db: -24.0,
    },
    Preset {
        name: "ssb_vdsl_s4_s8",
        signal: SignalKind::Ssb,
        noise: NoiseKind::Vdsl,
        snr_db: -24.0,
    },
    Preset {
        name: "ssb_qrn_s3_s9",
        signal: SignalKind::Ssb,
        noise: NoiseKind::Qrn,
        snr_db: -36.0,
    },
    Preset {
        name: "cw_powerline_s3_s7",
        signal: SignalKind::Cw,
        noise: NoiseKind::Powerline,
        snr_db: -24.0,
    },
    Preset {
        name: "ft8_vdsl_s4_s8",
        signal: SignalKind::Ft8,
        noise: NoiseKind::Vdsl,
        snr_db: -24.0,
    },
];

pub fn preset(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.name == name)
}

/// A generated fixture: the reference and the mix a gate feeds the denoiser.
#[derive(Debug, Clone)]
pub struct MixedFixture {
    pub clean: Vec<f64>,
    pub noisy: Vec<f64>,
    pub sr: u32,
    /// Peak absolute sample of `noisy` after headroom scaling (≤ [`synth::PEAK`]).
    pub noisy_peak: f64,
    /// Headroom factor applied to *both* channels; `< 1.0` means the raw mix
    /// would have clipped.
    pub headroom_scale: f64,
}

/// Render a clean/noisy pair at an exact SNR.
///
/// `seed` drives the signal; the noise uses `seed + 1` so that a preset rendered
/// at two SNRs keeps the same voice under the same buzz.
///
/// At the deeply negative SNRs the presets use, [`synth::mix`] has to scale the
/// noise far above full scale — an S3 signal under an S7 floor is −24 dB, so the
/// noise comes out ~16x hotter than a peak-normalized clean signal. Writing that
/// straight to a WAV clips it to hash. Both channels are therefore scaled by one
/// shared factor to fit the headroom: SNR is a *ratio*, so uniform scaling leaves
/// it untouched, and scaling the reference by the same factor keeps it sample- and
/// gain-aligned for [`crate::dsp::metrics::effective_snr_gain`].
pub fn render_mix(
    signal: SignalKind,
    noise: NoiseKind,
    snr_db: f64,
    sr: u32,
    duration_s: f64,
    seed: u64,
) -> Result<MixedFixture, DspError> {
    let mut clean = signal.render(sr, duration_s, seed)?;
    let noise = noise.render(sr, duration_s, seed.wrapping_add(1))?;
    let mut noisy = synth::mix(&clean, &noise, snr_db)?;

    let raw_peak = noisy.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let headroom_scale = if raw_peak > synth::PEAK {
        synth::PEAK / raw_peak
    } else {
        1.0
    };
    if headroom_scale < 1.0 {
        for v in noisy.iter_mut() {
            *v *= headroom_scale;
        }
        for v in clean.iter_mut() {
            *v *= headroom_scale;
        }
    }
    let noisy_peak = noisy.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    Ok(MixedFixture {
        clean,
        noisy,
        sr,
        noisy_peak,
        headroom_scale,
    })
}

/// Render a named [`Preset`].
pub fn render_preset(
    p: &Preset,
    sr: u32,
    duration_s: f64,
    seed: u64,
) -> Result<MixedFixture, DspError> {
    render_mix(p.signal, p.noise, p.snr_db, sr, duration_s, seed)
}

/// Write mono `f32` PCM. Samples are **not** scaled or limited — a caller that
/// hands this a mix hotter than full scale gets a clipped file, which is why
/// [`MixedFixture::noisy_peak`] exists.
pub fn write_wav(path: &Path, samples: &[f64], sr: u32) -> Result<(), DspError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sr,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DspError::new(format!("cannot create {}: {e}", parent.display())))?;
        }
    }
    let mut w = hound::WavWriter::create(path, spec)
        .map_err(|e| DspError::new(format!("cannot write {}: {e}", path.display())))?;
    for v in samples {
        w.write_sample(*v as f32)
            .map_err(|e| DspError::new(format!("write failed: {e}")))?;
    }
    w.finalize()
        .map_err(|e| DspError::new(format!("finalize failed: {e}")))?;
    Ok(())
}
