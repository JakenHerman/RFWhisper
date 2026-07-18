//! Single source of truth for sample rates, framing, and ONNX — see ROADMAP + AGENTS.md.

/// DeepFilterNet3 is full-band 48 kHz in upstream recipes.
pub const NATIVE_DFN_SR_HZ: u32 = 48_000;

/// 10 ms frame at 48 kHz (per AGENTS): primary chunk size for streaming alignment.
pub const DFN_FRAME_SAMPLES: usize = 480;

/// Acceptable input device rates; must resample to `NATIVE_DFN_SR_HZ` before the NN.
pub const SUPPORTED_IN_RATES_HZ: [u32; 6] = [8_000, 16_000, 32_000, 44_100, 48_000, 96_000];

/// v0.1 real-time default block (may be 10 ms at 48 kHz or 20 ms at 16 kHz after resample).
pub const DEFAULT_BLOCKSIZE: usize = 480;

/// ONNX export / runtime.
pub const ONNX_OPSET_MIN: u32 = 17;

/// Default ring-buffer depth in frames (tune to soundcard; keep total well under 100 ms p99).
pub const DEFAULT_RING_FRAMES: usize = 3;

/// Models registry path relative to project root.
pub const MODELS_DIRNAME: &str = "models";
