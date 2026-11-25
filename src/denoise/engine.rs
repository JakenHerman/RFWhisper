//! Denoise engine trait, the spectral stub implementation, and the name → engine
//! selector (`select_engine`).

use std::time::Instant;

use crate::constants::NATIVE_DFN_SR_HZ;
use crate::denoise::spectral_stub::{wiener_like_denoise, StreamingSpectral};
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
    /// The DeepFilterNet3 backend failed to initialize (feature `dfn`).
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

    /// Offline: denoise a whole buffer at `sr`, returning a sample-aligned buffer
    /// of the same length. Not for chunked realtime use — see [`Self::process_stream`].
    fn process(&mut self, x: &[f32], sr: u32) -> Vec<f32>;

    /// Realtime: feed mono samples at [`Self::native_sr`] and append enhanced
    /// samples to `out`. Stateful — each call continues the previous one, so a
    /// stream may be fed in chunks of any size. Output stream index `k`
    /// corresponds to input stream index `k - stream_offset()`.
    fn process_stream(&mut self, x: &[f32], out: &mut Vec<f32>);

    /// Index offset between the input and output streams of
    /// [`Self::process_stream`] (see there).
    fn stream_offset(&self) -> usize {
        0
    }

    /// Algorithmic latency of [`Self::process_stream`] in native-rate samples:
    /// how many further input samples must arrive before an input sample's
    /// enhanced version is emitted.
    fn stream_latency(&self) -> usize {
        self.stream_offset()
    }

    /// Short human-readable name for logs and telemetry.
    fn name(&self) -> &'static str;

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
#[derive(Default)]
pub struct SpectralStubEngine {
    stream: Option<StreamingSpectral>,
}

impl SpectralStubEngine {
    pub fn new() -> Self {
        Self::default()
    }

    fn stream_state(&mut self) -> &mut StreamingSpectral {
        let sr = self.native_sr();
        self.stream
            .get_or_insert_with(|| StreamingSpectral::new(sr))
    }
}

impl DenoiseEngine for SpectralStubEngine {
    fn process(&mut self, x: &[f32], sr: u32) -> Vec<f32> {
        let native = self.native_sr();
        let y = to_native_rate(x, sr, native);
        let y = wiener_like_denoise(&y, native);
        to_native_rate(&y, native, sr)
    }

    fn process_stream(&mut self, x: &[f32], out: &mut Vec<f32>) {
        self.stream_state().process(x, out);
    }

    fn stream_latency(&self) -> usize {
        StreamingSpectral::new(self.native_sr()).latency()
    }

    fn name(&self) -> &'static str {
        "spectral_stub"
    }
}

/// No processing at all: output equals input. For digital modes (FT8/FT4,
/// RTTY, PSK) and CW, where the speech-trained DeepFilterNet3 can remove the
/// very tones the operator needs, and as the "B" side of an A/B comparison.
#[derive(Default)]
pub struct PassthroughEngine;

impl DenoiseEngine for PassthroughEngine {
    fn process(&mut self, x: &[f32], _sr: u32) -> Vec<f32> {
        x.to_vec()
    }

    fn process_stream(&mut self, x: &[f32], out: &mut Vec<f32>) {
        out.extend_from_slice(x);
    }

    fn name(&self) -> &'static str {
        "passthrough"
    }
}

/// Every name [`select_engine`] accepts, for help text and error messages.
pub const MODEL_NAMES: [&str; 3] = ["deepfilternet3", "spectral_stub", "passthrough"];

/// Model names: `deepfilternet3` | `spectral_stub` | `passthrough` (alias
/// `none`) (a path ending in `.onnx` is reserved for the ONNX backend and errors
/// until that lands).
///
/// Env: `RFWHISPER_FORCE_STUB=1` forces the stub (CI without model weights).
///
/// Fallback semantics match the Python package: requesting `deepfilternet3` on a
/// host without a NN backend warns and returns the stub rather than failing, so
/// the CLI stays usable end-to-end.
pub fn select_engine(model: &str) -> Result<Box<dyn DenoiseEngine>, EngineError> {
    if std::env::var("RFWHISPER_FORCE_STUB").as_deref() == Ok("1") {
        return Ok(Box::new(SpectralStubEngine::new()));
    }
    if model == "spectral_stub" {
        return Ok(Box::new(SpectralStubEngine::new()));
    }
    if model == "passthrough" || model == "none" {
        return Ok(Box::new(PassthroughEngine));
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
                 `dfn` feature (--no-default-features); using SpectralStubEngine, which is \
                 NOT a real denoiser. Rebuild with `cargo build --release` for DeepFilterNet3."
            );
            return Ok(Box::new(SpectralStubEngine::new()));
        }
    }
    Err(EngineError::UnknownModel(model.to_string()))
}
