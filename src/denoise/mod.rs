//! Denoise engines: spectral stub (CI / no-model path) and the engine selector.
//!
//! The Python package also shipped optional `torch`/`deepfilternet` and placeholder
//! ONNX Runtime backends; in the Rust port the DeepFilterNet path is tracked as the
//! DFN3 backend issue (upstream DeepFilterNet's reference realtime implementation is
//! itself Rust — `deep_filter` / libDF).

#[cfg(feature = "dfn")]
pub mod dfn;
pub mod engine;
pub mod spectral_stub;

pub use engine::{select_engine, DenoiseEngine, EngineError, ProcessStats, SpectralStubEngine};
