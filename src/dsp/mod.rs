//! DSP primitives shared by the offline and realtime paths.

pub mod features;
pub mod filter;
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

/// Reject non-positive, NaN, and infinite parameters in one place.
///
/// Written as a positive test rather than `!(v > 0.0)` so NaN is rejected without
/// tripping clippy's `neg_cmp_op_on_partial_ord`.
pub(crate) fn require_positive(v: f64, what: &str) -> Result<(), DspError> {
    if v.is_finite() && v > 0.0 {
        return Ok(());
    }
    Err(DspError::new(format!(
        "{what} must be positive and finite (got {v})"
    )))
}
