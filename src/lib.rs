//! RFWhisper — real-time AI denoising for ham radio (local-first, GPLv3+).
//!
//! Rust port of the original Python package; module layout mirrors it 1:1 so the
//! ROADMAP acceptance gates (A1–A6) keep their meaning.

pub mod bench;
pub mod constants;
pub mod denoise;
pub mod dsp;
pub mod models;
pub mod realtime;
