//! Polyphase resampling helpers between common rates.
//!
//! `to_native_rate` is the streaming-prep workhorse used by the denoise engine to
//! land arbitrary input rates on the model's native sample rate.
//! `resample_48k_to_16k` / `resample_16k_to_48k` are fixed-ratio convenience
//! wrappers used by tests and any caller that knows it only ever moves between
//! those two rates (the v0.1 model I/O pair).
//!
//! The filter design mirrors SciPy's `resample_poly` defaults (Kaiser β=5.0
//! windowed sinc, `2 * 10 * max(up, down) + 1` taps) so behavior matches the
//! original Python package: polyphase rather than FFT resampling avoids the edge
//! artefacts that would surface in the streaming path.

/// Zeroth-order modified Bessel function of the first kind (series expansion).
fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let half_x = x / 2.0;
    for m in 1..64 {
        term *= (half_x / m as f64) * (half_x / m as f64);
        sum += term;
        if term < sum * 1e-18 {
            break;
        }
    }
    sum
}

/// Kaiser window of length `n` with shape parameter `beta` (symmetric).
fn kaiser_window(n: usize, beta: f64) -> Vec<f64> {
    if n == 1 {
        return vec![1.0];
    }
    let denom = bessel_i0(beta);
    let alpha = (n - 1) as f64 / 2.0;
    (0..n)
        .map(|i| {
            let t = (i as f64 - alpha) / alpha;
            bessel_i0(beta * (1.0 - t * t).max(0.0).sqrt()) / denom
        })
        .collect()
}

fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}

/// Windowed-sinc lowpass (SciPy `firwin` with a Kaiser window, `scale=True`).
/// `cutoff` is in Nyquist units (1.0 = Nyquist).
fn firwin_kaiser(numtaps: usize, cutoff: f64, beta: f64) -> Vec<f64> {
    let alpha = (numtaps - 1) as f64 / 2.0;
    let win = kaiser_window(numtaps, beta);
    let mut h: Vec<f64> = (0..numtaps)
        .map(|i| cutoff * sinc(cutoff * (i as f64 - alpha)) * win[i])
        .collect();
    // Normalize unity gain at DC (firwin scale=True with a passband at 0 Hz).
    let s: f64 = h.iter().sum();
    for v in &mut h {
        *v /= s;
    }
    h
}

/// Length of `upfirdn(h, x, up, down)` output (SciPy `_output_len`).
fn upfirdn_output_len(len_h: usize, in_len: usize, up: usize, down: usize) -> usize {
    if in_len == 0 {
        return 0;
    }
    ((in_len - 1) * up + len_h - 1) / down + 1
}

/// Naive polyphase `upfirdn`: zero-stuff by `up`, filter with `h`, keep every
/// `down`-th sample. O(n_out · taps / up) — plenty for offline prep and tests.
fn upfirdn(h: &[f64], x: &[f64], up: usize, down: usize) -> Vec<f64> {
    let n_out = upfirdn_output_len(h.len(), x.len(), up, down);
    let mut y = vec![0.0; n_out];
    if x.is_empty() {
        return y;
    }
    let max_j = (x.len() - 1) * up;
    for (m, out) in y.iter_mut().enumerate() {
        let t = m * down;
        let k_min = t.saturating_sub(max_j);
        let k_max = h.len().min(t + 1);
        let mut acc = 0.0;
        let mut k = k_min;
        // Advance k to the first tap where (t - k) is a multiple of `up`.
        while k < k_max && (t - k) % up != 0 {
            k += 1;
        }
        while k < k_max {
            acc += h[k] * x[(t - k) / up];
            k += up;
        }
        *out = acc;
    }
    y
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Polyphase resample of a 1-D signal by the rational factor `up / down`
/// (SciPy `resample_poly` semantics: Kaiser β=5.0, `half_len = 10 * max_rate`,
/// output length `ceil(n * up / down)`).
pub fn resample_poly(x: &[f64], up: usize, down: usize) -> Vec<f64> {
    assert!(up > 0 && down > 0, "up and down must be positive");
    let g = gcd(up, down);
    let (up, down) = (up / g, down / g);
    if up == down {
        return x.to_vec();
    }
    let n_out = (x.len() * up).div_ceil(down);
    let max_rate = up.max(down);
    let f_c = 1.0 / max_rate as f64;
    let half_len = 10 * max_rate;
    let mut h = firwin_kaiser(2 * half_len + 1, f_c, 5.0);
    for v in &mut h {
        *v *= up as f64;
    }

    let n_pre_pad = (down - half_len % down) % down;
    let n_pre_remove = (half_len + n_pre_pad) / down;
    let mut n_post_pad = 0;
    while upfirdn_output_len(h.len() + n_pre_pad + n_post_pad, x.len(), up, down)
        < n_out + n_pre_remove
    {
        n_post_pad += 1;
    }
    let mut h_padded = vec![0.0; n_pre_pad];
    h_padded.extend_from_slice(&h);
    h_padded.extend(std::iter::repeat(0.0).take(n_post_pad));

    let y = upfirdn(&h_padded, x, up, down);
    y[n_pre_remove..n_pre_remove + n_out].to_vec()
}

/// Resample 1-D float audio from `sr_in` to `sr_out` with polyphase [`resample_poly`].
///
/// Returns `f32` because the realtime path consumes `f32` throughout. When
/// `sr_in == sr_out` the input is returned unchanged.
pub fn to_native_rate(x: &[f32], sr_in: u32, sr_out: u32) -> Vec<f32> {
    if sr_in == sr_out {
        return x.to_vec();
    }
    let g = gcd(sr_in as usize, sr_out as usize);
    let up = sr_out as usize / g;
    let down = sr_in as usize / g;
    let x64: Vec<f64> = x.iter().map(|v| *v as f64).collect();
    resample_poly(&x64, up, down)
        .into_iter()
        .map(|v| v as f32)
        .collect()
}

/// Estimate input samples required to produce `block` output samples after resampling.
///
/// Used by ring-buffer sizing in the streaming pipeline so we can pre-allocate the
/// correct amount of input memory before each resample call.
pub fn next_chunk_size(block: usize, sr_in: u32, sr_out: u32) -> usize {
    if sr_in == sr_out {
        return block;
    }
    (block * sr_in as usize).div_ceil(sr_out as usize)
}

/// 48 kHz → 16 kHz via polyphase [`resample_poly`] (`up=1`, `down=3`).
pub fn resample_48k_to_16k(x: &[f64]) -> Vec<f64> {
    resample_poly(x, 1, 3)
}

/// 16 kHz → 48 kHz via polyphase [`resample_poly`] (`up=3`, `down=1`).
pub fn resample_16k_to_48k(x: &[f64]) -> Vec<f64> {
    resample_poly(x, 3, 1)
}

/// Stateful polyphase resampler for the realtime path.
///
/// [`to_native_rate`] is stateless: calling it on consecutive 10 ms chunks
/// restarts the filter every call, so each chunk boundary gets a zero-padded
/// edge and the output clicks at 100 Hz. `StreamResampler` keeps the last
/// `taps_per_phase - 1` input samples between calls, so feeding a signal in any
/// chunking produces bit-identical output to feeding it in one call.
///
/// Same filter design as [`resample_poly`] (Kaiser β=5.0, `half_len = 10 *
/// max(up, down)`), split into `up` polyphase branches. Unlike the offline
/// path it is causal: output lags input by [`StreamResampler::delay_out`]
/// samples (≈ 10 samples, well under 1 ms, at every supported rate pair).
///
/// Allocation-free in steady state once its internal buffer has grown to
/// `chunk + taps_per_phase` samples; `out` must be reused by the caller.
pub struct StreamResampler {
    up: usize,
    down: usize,
    /// `phases[p][j] = h[p + j * up]`.
    phases: Vec<Vec<f32>>,
    taps_per_phase: usize,
    /// Input history: `buf[0]` is absolute input sample `n_in - buf.len()`.
    buf: Vec<f32>,
    /// Total input samples consumed so far.
    n_in: u64,
    /// Upsampled-domain index of the next output sample (`m * down`).
    t: u64,
    half_len: usize,
}

impl StreamResampler {
    /// Resampler from `sr_in` to `sr_out`. Equal rates make it a zero-delay copy.
    pub fn new(sr_in: u32, sr_out: u32) -> Self {
        assert!(sr_in > 0 && sr_out > 0, "sample rates must be positive");
        let g = gcd(sr_in as usize, sr_out as usize);
        let up = sr_out as usize / g;
        let down = sr_in as usize / g;
        if up == down {
            return Self {
                up: 1,
                down: 1,
                phases: Vec::new(),
                taps_per_phase: 0,
                buf: Vec::new(),
                n_in: 0,
                t: 0,
                half_len: 0,
            };
        }
        let max_rate = up.max(down);
        let half_len = 10 * max_rate;
        let h = firwin_kaiser(2 * half_len + 1, 1.0 / max_rate as f64, 5.0);
        let taps_per_phase = h.len().div_ceil(up);
        let phases = (0..up)
            .map(|p| {
                (0..taps_per_phase)
                    .map(|j| h.get(p + j * up).map_or(0.0, |v| (v * up as f64) as f32))
                    .collect()
            })
            .collect();
        Self {
            up,
            down,
            phases,
            taps_per_phase,
            buf: Vec::new(),
            n_in: 0,
            t: 0,
            half_len,
        }
    }

    /// True when input and output rates are equal (no filtering, no delay).
    pub fn is_identity(&self) -> bool {
        self.up == self.down
    }

    /// Group delay in *output* samples (the filter is linear-phase, so every
    /// frequency is delayed by the same amount).
    pub fn delay_out(&self) -> f64 {
        self.half_len as f64 / self.down as f64
    }

    /// Resample `x` and append the result to `out`. Stateful across calls.
    pub fn process(&mut self, x: &[f32], out: &mut Vec<f32>) {
        if self.is_identity() {
            out.extend_from_slice(x);
            return;
        }
        self.buf.extend_from_slice(x);
        self.n_in += x.len() as u64;
        let base = self.n_in - self.buf.len() as u64;
        let (up, down) = (self.up as u64, self.down as u64);
        loop {
            let n = self.t / up; // newest input sample this output needs
            if n >= self.n_in {
                break;
            }
            let h = &self.phases[(self.t % up) as usize];
            let local_n = (n - base) as usize;
            let jmax = (local_n + 1).min(self.taps_per_phase);
            let mut acc = 0.0f32;
            for (j, tap) in h.iter().enumerate().take(jmax) {
                acc += tap * self.buf[local_n - j];
            }
            out.push(acc);
            self.t += down;
        }
        // Keep only the history the next output still needs.
        let next_n = self.t / up;
        let keep_from = next_n
            .saturating_sub(self.taps_per_phase as u64 - 1)
            .max(base);
        let drop = ((keep_from - base) as usize).min(self.buf.len());
        self.buf.drain(..drop);
    }
}
