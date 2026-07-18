//! Identity pass-through model.
//!
//! Used so CI can exercise the realtime path on matrix runners without Git LFS
//! weights. Honors the same `hop` / `sample_rate` contract as DFN3 / RNNoise so
//! framing, ring-buffer, and overlap-add machinery upstream is exercised
//! end-to-end. Output is bit-equal to input — gates A3 / A4 / A5 / A7 use this;
//! A1 / A2 / A6 must skip-with-reason since identity cannot improve SNR or decode
//! counts.

use crate::constants::{DFN_FRAME_SAMPLES, NATIVE_DFN_SR_HZ};
use crate::models::{Model, ModelError};

#[derive(Debug, Default, Clone, Copy)]
pub struct NullModel;

impl Model for NullModel {
    fn sample_rate(&self) -> u32 {
        NATIVE_DFN_SR_HZ
    }

    fn hop(&self) -> usize {
        DFN_FRAME_SAMPLES
    }

    fn process(&mut self, frame: &[f32]) -> Result<Vec<f32>, ModelError> {
        if frame.len() != self.hop() {
            return Err(ModelError(format!(
                "NullModel expects shape ({},); got ({},)",
                self.hop(),
                frame.len()
            )));
        }
        Ok(frame.to_vec())
    }
}
