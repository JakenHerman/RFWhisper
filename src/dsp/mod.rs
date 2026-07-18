//! DSP primitives shared by the offline and realtime paths.

pub mod features;
pub mod metrics;
pub mod resample;

/// Error type for DSP-layer contract violations (bad shapes, empty input, …).
///
/// The Python package raised `ValueError` for these; the Rust port surfaces them
/// as `Result` so realtime callers can decide whether to crash or skip.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("{0}")]
pub struct DspError(pub String);

impl DspError {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}
