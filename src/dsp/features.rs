//! Framing, periodic Hann windows, overlap-add, and offline STFT-frame helpers.

use crate::dsp::DspError;

/// DeepFilterNet3-style framing: 10 ms hop at native rates (ROADMAP / A4).
pub const HOP_48K: usize = 480;
pub const WIN_48K: usize = 960;
pub const HOP_16K: usize = 160;
pub const WIN_16K: usize = 320;

/// Periodic Hann of length `n` (`fftbins=True` / DFN3 reference convention).
///
/// Matches `scipy.signal.get_window("hann", n, fftbins=True)` (`sym=False`) within
/// floating error, including SciPy's length-1 special case (all-ones window).
pub fn hann_window(n: usize) -> Result<Vec<f64>, DspError> {
    if n == 0 {
        return Err(DspError::new("n must be positive"));
    }
    if n == 1 {
        return Ok(vec![1.0]);
    }
    Ok((0..n)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos()))
        .collect())
}

/// Streaming sqrt-Hann analysis + COLA synthesis (WOLA) for one channel.
///
/// * `push` ingests `hop` new samples.
/// * When `ready()`, `next_frame` returns `x * sqrt(w)` where `w` is the periodic
///   Hann (`fftbins=True`); advancing the FIFO by `hop`.
/// * `overlap_add` does `ola += processed * sqrt(w)` and returns the next `hop`
///   samples from the accumulator (50 % overlap ⇒ overlapping Hann weights sum to 1).
///
/// Samples are held in a fixed-capacity ring buffer (`win_size + hop`) so `push`
/// does not grow or concatenate on each hop—important for long streams and for
/// eventual realtime use (see AGENTS.md: avoid per-callback allocations).
pub struct FrameBuffer {
    win_size: usize,
    hop: usize,
    win_sqrt: Vec<f64>,
    cap: usize,
    fifo: Vec<f64>,
    rd: usize,
    n: usize,
    ola: Vec<f64>,
}

impl FrameBuffer {
    /// Pre-allocate the analysis ring buffer and overlap-add accumulator.
    ///
    /// `win_size` is the analysis window in samples (e.g. 960 at 48 kHz for DFN3);
    /// `hop` is the per-step advance (e.g. 480 = 50 % overlap). Both must be positive
    /// and `hop <= win_size`. Allocations happen here so the realtime path never
    /// allocates inside `push` / `next_frame` / `overlap_add`.
    pub fn new(win_size: usize, hop: usize) -> Result<Self, DspError> {
        if win_size == 0 || hop == 0 {
            return Err(DspError::new("win_size and hop must be positive"));
        }
        if hop > win_size {
            return Err(DspError::new("hop must not exceed win_size"));
        }
        let w = hann_window(win_size)?;
        let cap = win_size + hop;
        Ok(Self {
            win_size,
            hop,
            win_sqrt: w.iter().map(|v| v.sqrt()).collect(),
            cap,
            fifo: vec![0.0; cap],
            rd: 0,
            n: 0,
            ola: vec![0.0; win_size],
        })
    }

    /// Analysis-window length in samples.
    pub fn win_size(&self) -> usize {
        self.win_size
    }

    /// Per-step advance in samples (50 % overlap when `hop == win_size / 2`).
    pub fn hop(&self) -> usize {
        self.hop
    }

    /// Append exactly one hop of new samples to the analysis FIFO.
    ///
    /// Errors if the input length isn't `hop`, or if the caller has pushed more hops
    /// than the buffer can hold without consuming a frame — that's a programming
    /// error (the contract is push, then `next_frame` whenever `ready`).
    pub fn push(&mut self, hop_samples: &[f64]) -> Result<(), DspError> {
        if hop_samples.len() != self.hop {
            return Err(DspError::new(format!(
                "expected hop_samples with length {}, got {}",
                self.hop,
                hop_samples.len()
            )));
        }
        if self.n + self.hop > self.cap {
            return Err(DspError::new(
                "analysis fifo full: call next_frame() whenever ready() before pushing more hops",
            ));
        }
        let wr = (self.rd + self.n) % self.cap;
        let room = self.cap - wr;
        if room >= self.hop {
            self.fifo[wr..wr + self.hop].copy_from_slice(hop_samples);
        } else {
            self.fifo[wr..].copy_from_slice(&hop_samples[..room]);
            self.fifo[..self.hop - room].copy_from_slice(&hop_samples[room..]);
        }
        self.n += self.hop;
        Ok(())
    }

    /// True when the FIFO holds at least one full window.
    pub fn ready(&self) -> bool {
        self.n >= self.win_size
    }

    /// Return the next windowed analysis frame `x * sqrt(hann)` and advance by one hop.
    ///
    /// Errors if `ready()` is false — callers must check before calling.
    pub fn next_frame(&mut self) -> Result<Vec<f64>, DspError> {
        if !self.ready() {
            return Err(DspError::new("not enough samples for a full frame"));
        }
        let wsz = self.win_size;
        let mut x = vec![0.0; wsz];
        let first = wsz.min(self.cap - self.rd);
        x[..first].copy_from_slice(&self.fifo[self.rd..self.rd + first]);
        if first < wsz {
            x[first..].copy_from_slice(&self.fifo[..wsz - first]);
        }
        self.rd = (self.rd + self.hop) % self.cap;
        self.n -= self.hop;
        for (v, w) in x.iter_mut().zip(&self.win_sqrt) {
            *v *= w;
        }
        Ok(x)
    }

    /// Synthesis half of WOLA: weight by `sqrt(hann)`, add to the accumulator,
    /// and emit the next `hop` output samples.
    ///
    /// For 50 % overlap with periodic Hann, the overlapping square-root windows sum
    /// to one — so this reconstructs the time-domain signal without amplitude bias.
    pub fn overlap_add(&mut self, processed_frame: &[f64]) -> Result<Vec<f64>, DspError> {
        if processed_frame.len() != self.win_size {
            return Err(DspError::new(format!(
                "expected processed_frame with length {}, got {}",
                self.win_size,
                processed_frame.len()
            )));
        }
        for ((acc, p), w) in self.ola.iter_mut().zip(processed_frame).zip(&self.win_sqrt) {
            *acc += p * w;
        }
        let out = self.ola[..self.hop].to_vec();
        // Shift accumulator left by one hop; zero the new tail for the next overlap.
        let (w, h) = (self.win_size, self.hop);
        self.ola.copy_within(h..w, 0);
        for v in &mut self.ola[w - h..] {
            *v = 0.0;
        }
        Ok(out)
    }
}

/// Offline stack of sqrt-Hann analysis frames (matches [`FrameBuffer`]).
///
/// Frame `k` is `x[k * hop .. k * hop + win_size] * sqrt(hann_window(win_size))`.
/// Only complete frames are returned.
pub fn stft_frames(x: &[f64], win_size: usize, hop: usize) -> Result<Vec<Vec<f64>>, DspError> {
    if win_size == 0 || hop == 0 {
        return Err(DspError::new("win_size and hop must be positive"));
    }
    if hop > win_size {
        return Err(DspError::new("hop must not exceed win_size"));
    }
    if x.len() < win_size {
        return Ok(Vec::new());
    }
    let w_sqrt: Vec<f64> = hann_window(win_size)?.iter().map(|v| v.sqrt()).collect();
    let n_frames = 1 + (x.len() - win_size) / hop;
    let mut frames = Vec::with_capacity(n_frames);
    for k in 0..n_frames {
        let start = k * hop;
        frames.push(
            x[start..start + win_size]
                .iter()
                .zip(&w_sqrt)
                .map(|(v, w)| v * w)
                .collect(),
        );
    }
    Ok(frames)
}
