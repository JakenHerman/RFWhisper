//! Model registry, fetch, and the frame-level `Model` trait.

pub mod fetch;
pub mod null_model;
pub mod registry;

pub use null_model::NullModel;
pub use registry::{
    load_model, load_model_in, model_path, repo_root, resolve_providers, ModelArtifact,
    RegistryError, ARTIFACTS,
};

/// One-frame-in / one-frame-out denoiser at a fixed sample rate.
///
/// The realtime path consumes models through this trait — the DFN3 / RNNoise
/// backends and `rfwhisper::realtime` are the two sides of the contract.
pub trait Model {
    fn sample_rate(&self) -> u32;
    fn hop(&self) -> usize;

    /// Process a single mono `f32` frame of length `hop`. Returns the same shape.
    fn process(&mut self, frame: &[f32]) -> Result<Vec<f32>, ModelError>;
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{0}")]
pub struct ModelError(pub String);

/// Resolve `name` (`null` | `deepfilternet3` | `rnnoise`) to a `Model` instance.
pub fn load(name: &str, fallback_to_null: bool) -> Result<Box<dyn Model>, RegistryError> {
    load_model(name, fallback_to_null)
}
