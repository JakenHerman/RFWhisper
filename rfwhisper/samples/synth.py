"""Synthetic RFI generators for the acceptance gates (A1, A3, A6).

Every generator is a pure function of ``(sr, duration_s, ..., seed)`` — same arguments in,
bit-identical samples out, on any machine. That is what lets CI assert hard dB thresholds:
a gate failure means the denoiser changed, never that the fixture drifted.

The four noise types mirror the RFI sources called out in the README:

* :func:`powerline_buzz`   — switch-mode supplies, LED drivers, arcing insulators
* :func:`solar_inverter`   — MPPT / inverter switching, rhythmic buzz across HF
* :func:`vdsl_hash`        — PLC / Ethernet-over-powerline, a raised broadband floor
* :func:`atmospheric_qrn`  — static crashes on the low bands

:func:`mix` combines any of them with a clean signal at an exact SNR.

Generators return float64 in ``[-1, 1]``, peak-normalised so a clip can be written
straight to a WAV without clipping. Absolute level carries no meaning — :func:`mix`
sets the level that matters.
"""

from __future__ import annotations

import math

import numpy as np
from numpy.typing import NDArray
from scipy.signal import butter, sosfilt

# Peak headroom for generated clips: loud enough to hear, short of full scale.
PEAK: float = 0.95


def _n_samples(sr: int, duration_s: float) -> int:
    """Validate rate/duration and return the sample count."""
    if sr <= 0:
        raise ValueError("sr must be positive")
    if duration_s <= 0.0:
        raise ValueError("duration_s must be positive")
    n = int(round(sr * duration_s))
    if n <= 0:
        raise ValueError("duration_s is too short to produce a single sample at this sr")
    return n


def _normalise(x: NDArray[np.float64]) -> NDArray[np.float64]:
    """Peak-normalise to ``PEAK``; all-zero input is returned unchanged."""
    peak = float(np.max(np.abs(x)))
    if peak <= 0.0:
        return x
    return x * (PEAK / peak)


def powerline_buzz(
    sr: int,
    duration_s: float,
    fundamental_hz: float = 60.0,
    n_harmonics: int = 30,
    seed: int = 0,
) -> NDArray[np.float64]:
    """Harmonic comb from mains-borne RFI — the classic S7 buzz across a quiet band.

    A stack of ``n_harmonics`` partials of ``fundamental_hz`` with 1/n amplitude rolloff,
    randomised phase, and a slow per-harmonic amplitude wobble (real buzz breathes with
    the load rather than sitting perfectly still). Harmonics above Nyquist are dropped.
    """
    if fundamental_hz <= 0.0:
        raise ValueError("fundamental_hz must be positive")
    if n_harmonics <= 0:
        raise ValueError("n_harmonics must be positive")
    n = _n_samples(sr, duration_s)
    rng = np.random.default_rng(seed)
    t = np.arange(n, dtype=np.float64) / sr

    out = np.zeros(n, dtype=np.float64)
    nyquist = sr / 2.0
    for k in range(1, n_harmonics + 1):
        f = k * fundamental_hz
        if f >= nyquist:
            break
        phase = rng.uniform(0.0, 2.0 * np.pi)
        wobble_hz = rng.uniform(0.2, 1.5)
        wobble = 1.0 + 0.25 * np.sin(2.0 * np.pi * wobble_hz * t + rng.uniform(0.0, 2.0 * np.pi))
        out += (1.0 / k) * wobble * np.sin(2.0 * np.pi * f * t + phase)
    return _normalise(out)


def solar_inverter(
    sr: int,
    duration_s: float,
    tick_rate_hz: float = 120.0,
    ringing_q: float = 50.0,
    seed: int = 0,
) -> NDArray[np.float64]:
    """Rhythmic switching buzz — an impulse train that rings a resonance.

    Each tick at ``tick_rate_hz`` (twice mains, i.e. rectified) excites a damped sinusoid
    whose decay is set by ``ringing_q``: ``tau = Q / (pi * f0)``. Higher Q rings longer and
    sounds more tonal. Tick amplitude and centre frequency jitter slightly per tick, which
    is what makes an inverter sound different from a clean square wave.
    """
    if tick_rate_hz <= 0.0:
        raise ValueError("tick_rate_hz must be positive")
    if ringing_q <= 0.0:
        raise ValueError("ringing_q must be positive")
    n = _n_samples(sr, duration_s)
    rng = np.random.default_rng(seed)

    out = np.zeros(n, dtype=np.float64)
    period = sr / tick_rate_hz
    centre_hz = min(3_000.0, sr / 4.0)
    n_ticks = int(n / period)
    for i in range(n_ticks + 1):
        start = int(round(i * period))
        if start >= n:
            break
        f0 = centre_hz * rng.uniform(0.9, 1.1)
        tau = ringing_q / (np.pi * f0)
        # Truncate each ring at ~5 tau; beyond that it is below -43 dB and just costs time.
        length = min(n - start, int(round(5.0 * tau * sr)) + 1)
        if length <= 0:
            continue
        tt = np.arange(length, dtype=np.float64) / sr
        amp = rng.uniform(0.7, 1.0)
        out[start : start + length] += amp * np.exp(-tt / tau) * np.sin(2.0 * np.pi * f0 * tt)
    return _normalise(out)


def vdsl_hash(sr: int, duration_s: float, seed: int = 0) -> NDArray[np.float64]:
    """Wideband raised noise floor — PLC / VDSL / Ethernet-over-powerline hash.

    Band-limited white noise (300 Hz – 0.45 Nyquist) with a shallow tilt toward the high
    end, plus a few weak stationary carriers. Perceptually this is the "the band just got
    10 dB louder and there is nothing to null out" case, and it is the hardest of the four
    for a classical notch to touch — which is the point of testing against it.
    """
    n = _n_samples(sr, duration_s)
    rng = np.random.default_rng(seed)

    noise = rng.standard_normal(n)
    low = 300.0
    high = 0.45 * (sr / 2.0)
    if low >= high:
        raise ValueError("sr is too low to synthesise VDSL hash (needs > ~1.4 kHz)")
    sos = butter(4, [low, high], btype="bandpass", fs=sr, output="sos")
    shaped: NDArray[np.float64] = np.asarray(sosfilt(sos, noise), dtype=np.float64)

    # A shallow high-frequency tilt, then a handful of DMT-ish residual carriers.
    t = np.arange(n, dtype=np.float64) / sr
    for f in (2_100.0, 3_400.0, 5_600.0):
        if f < 0.45 * (sr / 2.0):
            shaped += 0.06 * np.sin(2.0 * np.pi * f * t + rng.uniform(0.0, 2.0 * np.pi))
    return _normalise(shaped)


def atmospheric_qrn(
    sr: int,
    duration_s: float,
    crash_rate_hz: float = 2.0,
    seed: int = 0,
) -> NDArray[np.float64]:
    """Static crashes — Poisson-timed broadband bursts with exponential decay.

    ``crash_rate_hz`` is the mean crash rate; inter-arrival times are exponential, so the
    clustering is realistic rather than metronomic. Each crash is filtered white noise with
    a 3–25 ms decay, the range that reads as "distant storm" through an SSB filter.
    """
    if crash_rate_hz <= 0.0:
        raise ValueError("crash_rate_hz must be positive")
    n = _n_samples(sr, duration_s)
    rng = np.random.default_rng(seed)

    out = np.zeros(n, dtype=np.float64)
    high = min(6_000.0, 0.45 * (sr / 2.0))
    sos = butter(2, high, btype="lowpass", fs=sr, output="sos")

    t_s = float(rng.exponential(1.0 / crash_rate_hz))
    while t_s < duration_s:
        start = int(round(t_s * sr))
        tau = rng.uniform(0.003, 0.025)
        length = min(n - start, int(round(6.0 * tau * sr)) + 1)
        if length > 0:
            tt = np.arange(length, dtype=np.float64) / sr
            burst = rng.standard_normal(length) * np.exp(-tt / tau)
            out[start : start + length] += rng.uniform(0.5, 1.0) * burst
        t_s += float(rng.exponential(1.0 / crash_rate_hz))

    filtered: NDArray[np.float64] = np.asarray(sosfilt(sos, out), dtype=np.float64)
    return _normalise(filtered)


def mix(
    clean: NDArray[np.floating],
    noise: NDArray[np.floating],
    snr_db: float,
) -> NDArray[np.float64]:
    """Add ``noise`` to ``clean`` at exactly ``snr_db``, by signal-to-noise power ratio.

    The noise is rescaled — the clean signal is never touched — so the caller's reference
    stays sample-aligned with the mix, which is what :func:`rfwhisper.dsp.metrics.
    effective_snr_gain` needs. The result is *not* normalised: peak-normalising here would
    silently change the SNR the caller just asked for. Clip-check before writing a WAV.
    """
    c = np.asarray(clean, dtype=np.float64)
    n_arr = np.asarray(noise, dtype=np.float64)
    if c.ndim != 1 or n_arr.ndim != 1:
        raise ValueError("clean and noise must be 1-D mono (shape (n,))")
    if c.shape != n_arr.shape:
        raise ValueError(
            f"clean and noise must be the same length, got {c.shape} and {n_arr.shape}"
        )

    clean_power = float(np.dot(c, c))
    noise_power = float(np.dot(n_arr, n_arr))
    if clean_power <= 0.0:
        raise ValueError("clean is all zeros; SNR is undefined")
    if noise_power <= 0.0:
        raise ValueError("noise is all zeros; SNR is undefined")

    target = clean_power / (10.0 ** (snr_db / 10.0))
    scale = math.sqrt(target / noise_power)
    return c + scale * n_arr
