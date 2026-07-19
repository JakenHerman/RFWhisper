//! Denoise engine trait, the spectral stub implementation, and the name → engine
//! selector (`select_engine`).

use std::time::Instant;

use crate::constants::NATIVE_DFN_SR_HZ;
use crate::denoise::spectral_stub::wiener_like_denoise;
use crate::dsp::resample::to_native_rate;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Unknown model {0:?}")]
    UnknownModel(String),
    #[error(
        "ONNX Runtime backend is not yet available in the Rust port (tracked in the \
         DFN3-backend issue); set RFWHISPER_FORCE_STUB=1 or use --model spectral_stub"
    )]
    OnnxUnavailable,
    /// The DeepFilterNet3 backend failed to initialise (feature `dfn`).
    #[error("{0}")]
    BackendInit(String),
}

/// Wall-clock vs audio-clock stats from an offline `process_file` run.
#[derive(Debug, Clone, Copy)]
pub struct ProcessStats {
    pub seconds_audio: f64,
    pub wall_seconds: f64,
}

impl ProcessStats {
    /// Real-time factor: wall seconds per second of audio (`< 1.0` = faster than realtime).
    pub fn rtf(&self) -> f64 {
        if self.seconds_audio > 0.0 {
            self.wall_seconds / self.seconds_audio
        } else {
            f64::INFINITY
        }
    }
}

/// One engine = one denoiser. Mono `f32` PCM in; `sr` may differ from the engine's
/// native rate — implementations resample internally when needed.
///
/// Not `Send`: the DeepFilterNet3 backend ([`crate::denoise::dfn`]) wraps tract's
/// streaming state, which holds `Rc` and cannot cross threads. The realtime path
/// therefore *constructs* its engine inside the worker thread (see
/// `realtime::stream_denoise`) rather than moving one in, so nothing here needs to
/// be `Send`.
pub trait DenoiseEngine {
    fn native_sr(&self) -> u32 {
        NATIVE_DFN_SR_HZ
    }

    fn process(&mut self, x: &[f32], sr: u32) -> Vec<f32>;

    fn process_file(&mut self, x: &[f32], sr: u32) -> (Vec<f32>, ProcessStats) {
        let t0 = Instant::now();
        let y = self.process(x, sr);
        let wall = t0.elapsed().as_secs_f64();
        (
            y,
            ProcessStats {
                seconds_audio: x.len() as f64 / sr as f64,
                wall_seconds: wall,
            },
        )
    }
}

/// CI / no-model path; maps to acceptance wiring, not production quality.
pub struct SpectralStubEngine;

impl DenoiseEngine for SpectralStubEngine {
    fn process(&mut self, x: &[f32], sr: u32) -> Vec<f32> {
        let native = self.native_sr();
        let y = to_native_rate(x, sr, native);
        let y = wiener_like_denoise(&y, native);
        to_native_rate(&y, native, sr)
    }
}

/// Model names: `deepfilternet3` | `spectral_stub` (a path ending in `.onnx` is
/// reserved for the ONNX backend and errors until that lands).
///
/// Env: `RFWHISPER_FORCE_STUB=1` forces the stub (CI without model weights).
///
/// Fallback semantics match the Python package: requesting `deepfilternet3` on a
/// host without a NN backend warns and returns the stub rather than failing, so
/// the CLI stays usable end-to-end.
pub fn select_engine(model: &str) -> Result<Box<dyn DenoiseEngine>, EngineError> {
    if std::env::var("RFWHISPER_FORCE_STUB").as_deref() == Ok("1") {
        return Ok(Box::new(SpectralStubEngine));
    }
    if model == "spectral_stub" {
        return Ok(Box::new(SpectralStubEngine));
    }
    if model.ends_with(".onnx") {
        return Err(EngineError::OnnxUnavailable);
    }
    if model == "deepfilternet3" {
        #[cfg(feature = "dfn")]
        {
            return Ok(Box::new(crate::denoise::dfn::DfnEngine::new()?));
        }
        #[cfg(not(feature = "dfn"))]
        {
            eprintln!(
                "warning: deepfilternet3 requested but this binary was built without the \
                 `dfn` feature; using SpectralStubEngine (CI / no-model path). Rebuild with \
                 `cargo build --features dfn` for the real backend."
            );
            return Ok(Box::new(SpectralStubEngine));
        }
    }
    Err(EngineError::UnknownModel(model.to_string()))
}
