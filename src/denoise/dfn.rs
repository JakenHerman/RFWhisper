//! DeepFilterNet3 backend (#10) — the real denoiser, behind the `dfn` feature.
//!
//! Wraps upstream DeepFilterNet's own Rust inference engine (`deep_filter`'s
//! `DfTract`, tract runtime, bundled DeepFilterNet3 model) behind the crate's
//! [`DenoiseEngine`](crate::denoise::DenoiseEngine) trait, so
//! `select_engine("deepfilternet3")` stops falling back to the spectral stub.
//!
//! Why upstream's crate rather than a hand-rolled ONNX path: DeepFilterNet's
//! realtime reference implementation *is* this crate, so we inherit its exact
//! feature extraction (ERB + deep-filter stages) and its tuned model, sidestepping
//! the export-parity risk of a third-party ONNX conversion.
//!
//! The model is full-band 48 kHz; audio at any other rate is resampled in and
//! back out. `DfTract::process` is a streaming API that emits `hop_size` enhanced
//! samples per call with a fixed algorithmic lookahead — this offline wrapper
//! flushes and trims that lookahead so the returned signal stays sample-aligned
//! with the input (which is what the A1 matched-filter metric expects).

// The `deep_filter` package's library is named `df` (see its Cargo.toml `[lib]`).
use df::tract::{DfParams, DfTract, RuntimeParams};
use ndarray::{Array2, ArrayView2};

use crate::constants::NATIVE_DFN_SR_HZ;
use crate::denoise::engine::{DenoiseEngine, EngineError};
use crate::dsp::resample::to_native_rate;

/// A ready-to-run DeepFilterNet3 model plus its per-run scratch buffers.
pub struct DfnEngine {
    model: DfTract,
    hop: usize,
    /// Total algorithmic delay in samples: the STFT delay (`fft_size - hop_size`)
    /// plus the model lookahead (`lookahead * hop_size`). The output is delayed by
    /// this much relative to the input; we flush and trim it so the returned signal
    /// stays sample-aligned. Matches upstream's `enhance_wav` example exactly.
    delay: usize,
}

impl DfnEngine {
    /// Load the bundled DeepFilterNet3 model with default runtime parameters.
    ///
    /// Errors (rather than panicking like `DfTract::default`) so
    /// `select_engine` can surface a load failure as `EngineError` and the CLI
    /// can exit with the model-failure code instead of aborting.
    pub fn new() -> Result<Self, EngineError> {
        let params = DfParams::default(); // bundled model (feature `default-model`)
        let rp = RuntimeParams::default_with_ch(1);
        let model = DfTract::new(params, &rp)
            .map_err(|e| EngineError::BackendInit(format!("DeepFilterNet3 init failed: {e}")))?;
        let hop = model.hop_size;
        let delay = (model.fft_size - model.hop_size) + model.lookahead * model.hop_size;
        Ok(Self { model, hop, delay })
    }

    /// Run one `hop`-sized frame through the model, writing enhanced samples.
    fn process_hop(&mut self, frame: &[f32], out: &mut [f32]) {
        let noisy: ArrayView2<f32> =
            ArrayView2::from_shape((1, self.hop), frame).expect("frame is one hop");
        let mut enh: Array2<f32> = Array2::zeros((1, self.hop));
        // A per-frame inference error should not abort the whole file; on failure
        // pass the frame through unmodified so the output stays intelligible.
        if self.model.process(noisy, enh.view_mut()).is_ok() {
            out.copy_from_slice(enh.as_slice().expect("contiguous"));
        } else {
            out.copy_from_slice(frame);
        }
    }

    /// Denoise a 48 kHz mono buffer, returning a buffer of the same length.
    fn process_native(&mut self, x: &[f32]) -> Vec<f32> {
        if x.is_empty() {
            return Vec::new();
        }
        // Feed every input hop, then enough zero hops to flush the algorithmic
        // delay out of the pipeline; the enhanced sample for input sample k emerges
        // `delay` samples later in the output stream.
        let n_in_hops = x.len().div_ceil(self.hop);
        let flush_hops = self.delay.div_ceil(self.hop);
        let total_hops = n_in_hops + flush_hops;

        let mut enhanced = Vec::with_capacity(total_hops * self.hop);
        let mut frame = vec![0.0f32; self.hop];
        let mut out = vec![0.0f32; self.hop];
        for h in 0..total_hops {
            let start = h * self.hop;
            for (j, f) in frame.iter_mut().enumerate() {
                *f = x.get(start + j).copied().unwrap_or(0.0);
            }
            self.process_hop(&frame, &mut out);
            enhanced.extend_from_slice(&out);
        }

        // Drop the `delay` samples of pipeline latency, then trim to input length.
        let aligned = &enhanced[self.delay.min(enhanced.len())..];
        let mut y = aligned.to_vec();
        y.resize(x.len(), 0.0);
        y
    }
}

impl DenoiseEngine for DfnEngine {
    fn native_sr(&self) -> u32 {
        NATIVE_DFN_SR_HZ
    }

    fn process(&mut self, x: &[f32], sr: u32) -> Vec<f32> {
        let native = self.native_sr();
        let up = to_native_rate(x, sr, native);
        let enhanced = self.process_native(&up);
        to_native_rate(&enhanced, native, sr)
    }
}
