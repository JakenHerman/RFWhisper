//! Benchmark helpers (A4/A5): processing latency percentiles and real-time factor.
//!
//! Processing latency here is file → file, not full audio-interface round trip.
//! Full round-trip (A4) needs an impulse loop through hardware.

use std::path::Path;
use std::time::Instant;

use crate::denoise::{select_engine, EngineError};

#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    #[error("cannot read {path}: {msg}")]
    Read { path: String, msg: String },
    #[error(transparent)]
    Engine(#[from] EngineError),
}

#[derive(Debug, Clone, Copy)]
pub struct LatencyReport {
    pub p50_ms: f64,
    pub p99_ms: f64,
    pub n_chunks: usize,
}

/// Read a WAV file as mono f32 (first channel) plus its sample rate.
pub fn read_wav_mono(path: &Path) -> Result<(Vec<f32>, u32), BenchError> {
    let mut reader = hound::WavReader::open(path).map_err(|e| BenchError::Read {
        path: path.display().to_string(),
        msg: e.to_string(),
    })?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let mono: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .step_by(channels)
            .collect::<Result<_, _>>(),
        hound::SampleFormat::Int => {
            let denom = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .step_by(channels)
                .map(|s| s.map(|v| v as f32 / denom))
                .collect::<Result<_, _>>()
        }
    }
    .map_err(|e| BenchError::Read {
        path: path.display().to_string(),
        msg: e.to_string(),
    })?;
    Ok((mono, spec.sample_rate))
}

fn resolve_model(model: &str) -> &str {
    if std::env::var("RFWHISPER_FORCE_STUB").as_deref() == Ok("1") {
        "spectral_stub"
    } else {
        model
    }
}

/// Chunked per-block processing latency on a WAV file (p50 / p99, ms).
pub fn measure_file_latency(
    path: &Path,
    model: &str,
    block: usize,
) -> Result<LatencyReport, BenchError> {
    let (mono, sr) = read_wav_mono(path)?;
    let mut eng = select_engine(resolve_model(model))?;
    let mut times: Vec<f64> = Vec::new();
    let mut i = 0;
    while i + block <= mono.len() {
        let t0 = Instant::now();
        let _ = eng.process(&mono[i..i + block], sr);
        times.push(t0.elapsed().as_secs_f64() * 1000.0);
        i += block;
    }
    if times.is_empty() {
        return Ok(LatencyReport {
            p50_ms: 0.0,
            p99_ms: 0.0,
            n_chunks: 0,
        });
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = times.len();
    Ok(LatencyReport {
        p50_ms: times[n / 2],
        p99_ms: times[(0.99 * (n - 1) as f64) as usize],
        n_chunks: n,
    })
}

/// Real-time factor for offline `process_file` (A5 gate helper).
pub fn measure_file_rtf(path: &Path, model: &str) -> Result<(f64, f64), BenchError> {
    let (mono, sr) = read_wav_mono(path)?;
    let mut eng = select_engine(resolve_model(model))?;
    let (_, stats) = eng.process_file(&mono, sr);
    Ok((stats.rtf(), stats.seconds_audio))
}
