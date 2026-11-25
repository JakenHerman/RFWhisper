//! Device-independent realtime signal chain (#14).
//!
//! `device rate → StreamResampler → engine.process_stream → A/B mix →
//! StreamResampler → device rate`.
//!
//! Everything here is stateful and chunking-invariant, so the cpal callbacks can
//! deliver audio in whatever block size the OS picks. Nothing touches an audio
//! device, which is what makes the realtime path unit-testable on headless CI.
//!
//! **A/B bypass.** The engine keeps running while bypassed, and the "raw" side
//! is delayed to line up sample-for-sample with the denoised side. Flipping the
//! toggle therefore never jumps in time, and a 5 ms crossfade keeps it click-free
//! — what you hear is the same moment of audio, with and without the denoiser.

use std::collections::VecDeque;

use crate::denoise::DenoiseEngine;
use crate::dsp::resample::StreamResampler;

/// Crossfade length for the A/B toggle, in milliseconds.
pub const BYPASS_FADE_MS: f32 = 5.0;

pub struct StreamPipeline {
    engine: Box<dyn DenoiseEngine>,
    sr_in: u32,
    sr_out: u32,
    native_sr: u32,
    rs_in: StreamResampler,
    rs_out: StreamResampler,
    native_in: Vec<f32>,
    enhanced: Vec<f32>,
    raw_fifo: VecDeque<f32>,
    raw_aligned: Vec<f32>,
    mixed: Vec<f32>,
    bypass: bool,
    /// 1.0 = fully denoised, 0.0 = fully raw.
    wet: f32,
    fade_step: f32,
}

impl StreamPipeline {
    /// Wrap `engine` for a stream arriving at `sr_in` and leaving at `sr_out`.
    pub fn new(engine: Box<dyn DenoiseEngine>, sr_in: u32, sr_out: u32) -> Self {
        let native_sr = engine.native_sr();
        let offset = engine.stream_offset();
        let mut raw_fifo = VecDeque::with_capacity(offset + 8192);
        raw_fifo.extend(std::iter::repeat(0.0).take(offset));
        Self {
            engine,
            sr_in,
            sr_out,
            native_sr,
            rs_in: StreamResampler::new(sr_in, native_sr),
            rs_out: StreamResampler::new(native_sr, sr_out),
            native_in: Vec::with_capacity(8192),
            enhanced: Vec::with_capacity(8192),
            raw_fifo,
            raw_aligned: Vec::with_capacity(8192),
            mixed: Vec::with_capacity(8192),
            bypass: false,
            wet: 1.0,
            fade_step: 1.0 / (BYPASS_FADE_MS / 1000.0 * native_sr as f32).max(1.0),
        }
    }

    /// Route the raw (aligned) signal to the output instead of the denoised one.
    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn bypass(&self) -> bool {
        self.bypass
    }

    pub fn engine_name(&self) -> &'static str {
        self.engine.name()
    }

    pub fn sr_in(&self) -> u32 {
        self.sr_in
    }

    pub fn sr_out(&self) -> u32 {
        self.sr_out
    }

    /// Sample rate of [`Self::last_raw`] / [`Self::last_denoised`].
    pub fn native_sr(&self) -> u32 {
        self.native_sr
    }

    /// Algorithmic latency of the chain (resamplers + engine), in milliseconds.
    /// Device buffers and queueing come on top; `denoise-live` reports those.
    pub fn latency_ms(&self) -> f64 {
        let n = self.native_sr as f64;
        1000.0
            * (self.rs_in.delay_out() / n
                + self.engine.stream_latency() as f64 / n
                + self.rs_out.delay_out() / self.sr_out as f64)
    }

    /// Process one chunk at `sr_in`; append output at `sr_out` to `out`.
    pub fn process(&mut self, x: &[f32], out: &mut Vec<f32>) {
        self.native_in.clear();
        self.rs_in.process(x, &mut self.native_in);
        self.raw_fifo.extend(self.native_in.iter().copied());

        self.enhanced.clear();
        self.engine
            .process_stream(&self.native_in, &mut self.enhanced);

        self.raw_aligned.clear();
        self.mixed.clear();
        let target = if self.bypass { 0.0 } else { 1.0 };
        for &e in &self.enhanced {
            let r = self.raw_fifo.pop_front().unwrap_or(0.0);
            self.raw_aligned.push(r);
            if self.wet < target {
                self.wet = (self.wet + self.fade_step).min(target);
            } else if self.wet > target {
                self.wet = (self.wet - self.fade_step).max(target);
            }
            self.mixed.push(self.wet * e + (1.0 - self.wet) * r);
        }
        self.rs_out.process(&self.mixed, out);
    }

    /// Raw input from the last [`Self::process`] call at the native rate,
    /// time-aligned sample-for-sample with [`Self::last_denoised`].
    pub fn last_raw(&self) -> &[f32] {
        &self.raw_aligned
    }

    /// Engine output from the last [`Self::process`] call at the native rate
    /// (independent of the A/B state).
    pub fn last_denoised(&self) -> &[f32] {
        &self.enhanced
    }
}
