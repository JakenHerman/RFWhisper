"""Shared audio-quality metrics for the ROADMAP acceptance gates (A1, A3, A5, A6).

Every gate that needs a number gets it from here, so a threshold can never drift
between two tests that claim to measure the same thing.

``pesq`` / ``pystoi`` live in the ``[eval]`` extra and are imported lazily — importing
this module never requires them.
"""

from __future__ import annotations

import math

import numpy as np
from numpy.typing import NDArray
from scipy.signal import correlate

# Alignment search window for the matched filter. Denoisers introduce algorithmic
# delay (WOLA framing, lookahead); 50 ms covers every v0.1 path with margin.
MAX_ALIGN_MS: float = 50.0


def _as_1d(x: NDArray[np.floating], name: str) -> NDArray[np.float64]:
    """Coerce to a 1-D float64 array, rejecting multi-channel or empty input."""
    a = np.asarray(x, dtype=np.float64)
    if a.ndim != 1:
        raise ValueError(f"{name} must be 1-D mono (shape (n,)); pass one channel at a time")
    if a.size == 0:
        raise ValueError(f"{name} must be non-empty")
    return a


def _best_lag(x: NDArray[np.float64], ref: NDArray[np.float64], max_lag: int) -> int:
    """Lag (in samples) of ``x`` relative to ``ref`` maximising |cross-correlation|."""
    corr = correlate(x, ref, mode="full", method="fft")
    zero = ref.size - 1
    lo = max(0, zero - max_lag)
    hi = min(corr.size, zero + max_lag + 1)
    return int(np.argmax(np.abs(corr[lo:hi]))) + lo - zero


def alignment_lag(
    x: NDArray[np.floating],
    ref: NDArray[np.floating],
    sr: int,
    max_align_ms: float = MAX_ALIGN_MS,
) -> int:
    """Samples by which ``x`` lags ``ref``, searched over ``±max_align_ms``.

    Any gate that compares a denoised stream against its raw input needs this: a model
    with algorithmic latency shifts every feature in the signal, and comparing at fixed
    indices would score that delay as damage. Positive means ``x`` arrives later.
    """
    if sr <= 0:
        raise ValueError("sr must be positive")
    if max_align_ms <= 0.0:
        raise ValueError("max_align_ms must be positive")
    a = _as_1d(x, "x")
    r = _as_1d(ref, "ref")
    return _best_lag(a, r, int(round(max_align_ms * sr / 1000.0)))


def _matched_filter_snr_db(x: NDArray[np.float64], ref: NDArray[np.float64], sr: int) -> float:
    """SNR of ``x`` against clean ``ref``, in dB.

    Aligns ``x`` to ``ref`` by cross-correlation, projects it onto the reference to
    recover the best-fit scale (so gain differences are not counted as noise), and
    reports ``10 log10(||projection||^2 / ||residual||^2)``.

    Returns ``+inf`` when the residual is exactly zero (``x`` is a scaled copy of
    ``ref``) — see :func:`effective_snr_gain` for what that means for callers.
    """
    max_lag = int(round(MAX_ALIGN_MS * sr / 1000.0))
    lag = _best_lag(x, ref, max_lag)
    if lag >= 0:
        n = min(x.size - lag, ref.size)
        x_seg, ref_seg = x[lag : lag + n], ref[:n]
    else:
        n = min(x.size, ref.size + lag)
        x_seg, ref_seg = x[:n], ref[-lag : -lag + n]
    if n <= 0:
        raise ValueError("signals do not overlap after alignment; check lengths and sample rate")

    ref_energy = float(np.dot(ref_seg, ref_seg))
    if ref_energy <= 0.0:
        raise ValueError("clean reference is all zeros; SNR is undefined")
    scale = float(np.dot(x_seg, ref_seg)) / ref_energy
    signal = scale * ref_seg
    residual = x_seg - signal

    signal_energy = float(np.dot(signal, signal))
    residual_energy = float(np.dot(residual, residual))
    if signal_energy <= 0.0:
        return -math.inf
    if residual_energy <= 0.0:
        return math.inf
    return 10.0 * math.log10(signal_energy / residual_energy)


def effective_snr_gain(
    clean: NDArray[np.floating],
    noisy: NDArray[np.floating],
    denoised: NDArray[np.floating],
    sr: int,
) -> float:
    """Effective SNR gain in dB from denoising, measured against a clean reference (A1).

    Both ``noisy`` and ``denoised`` are matched-filtered against ``clean``; the result is
    ``snr(denoised) - snr(noisy)``. Positive means the denoiser helped.

    Sentinels, so callers can assert on them rather than on NaN:

    * ``+inf`` — ``denoised`` is a scaled copy of ``clean`` (perfect reconstruction).
    * ``-inf`` — ``denoised`` retains no component of ``clean`` at all.
    * ``nan``  — ``noisy`` was already a perfect copy of ``clean``, so there was no
      noise to remove and "gain" has no meaning.
    """
    if sr <= 0:
        raise ValueError("sr must be positive")
    c = _as_1d(clean, "clean")
    n = _as_1d(noisy, "noisy")
    d = _as_1d(denoised, "denoised")

    snr_noisy = _matched_filter_snr_db(n, c, sr)
    snr_denoised = _matched_filter_snr_db(d, c, sr)
    if math.isinf(snr_noisy) and snr_noisy > 0.0:
        return math.nan
    return snr_denoised - snr_noisy


def keying_onset_rms(
    signal: NDArray[np.floating],
    onset_samples: NDArray[np.integer],
    window_ms: float = 5.0,
    sr: int = 48_000,
) -> NDArray[np.float64]:
    """RMS of the first ``window_ms`` after each key-down, one value per onset (A3).

    A3 compares this array between raw and denoised audio: a denoiser that softens CW
    keying edges shows up here as a drop, even when broadband metrics look fine.
    """
    if sr <= 0:
        raise ValueError("sr must be positive")
    if window_ms <= 0.0:
        raise ValueError("window_ms must be positive")
    x = _as_1d(signal, "signal")
    onsets = np.asarray(onset_samples, dtype=np.int64)
    if onsets.ndim != 1:
        raise ValueError("onset_samples must be 1-D")

    width = int(round(window_ms * sr / 1000.0))
    if width <= 0:
        raise ValueError("window_ms is too short to cover a single sample at this sample rate")

    out = np.empty(onsets.size, dtype=np.float64)
    for i, onset in enumerate(onsets):
        start = int(onset)
        if start < 0 or start + width > x.size:
            raise ValueError(
                f"onset {start} with a {width}-sample window falls outside the signal "
                f"(length {x.size})"
            )
        window = x[start : start + width]
        out[i] = math.sqrt(float(np.dot(window, window)) / width)
    return out


def pesq_score(ref: NDArray[np.floating], deg: NDArray[np.floating], sr: int) -> float:
    """PESQ (ITU-T P.862) of ``deg`` against ``ref`` (A6). Needs the ``[eval]`` extra.

    ``sr`` must be 8000 (narrowband) or 16000 (wideband) — the standard defines no other.
    """
    if sr not in (8_000, 16_000):
        raise ValueError(f"PESQ is defined for 8000 or 16000 Hz only, got {sr}")
    r = _as_1d(ref, "ref")
    d = _as_1d(deg, "deg")
    try:
        from pesq import pesq as _pesq
    except ImportError as exc:  # pragma: no cover - exercised only without the extra
        raise ImportError(
            "pesq is not installed; install the eval extra: pip install -e '.[eval]'"
        ) from exc
    mode = "wb" if sr == 16_000 else "nb"
    return float(_pesq(sr, r, d, mode))


def stoi_score(ref: NDArray[np.floating], deg: NDArray[np.floating], sr: int) -> float:
    """STOI intelligibility of ``deg`` against ``ref`` (A6). Needs the ``[eval]`` extra."""
    if sr <= 0:
        raise ValueError("sr must be positive")
    r = _as_1d(ref, "ref")
    d = _as_1d(deg, "deg")
    try:
        from pystoi import stoi as _stoi
    except ImportError as exc:  # pragma: no cover - exercised only without the extra
        raise ImportError(
            "pystoi is not installed; install the eval extra: pip install -e '.[eval]'"
        ) from exc
    return float(_stoi(r, d, sr, extended=False))


def rtf(input_duration_s: float, process_wall_s: float) -> float:
    """Real-time factor: wall-clock seconds spent per second of audio (A5).

    ``< 1.0`` is faster than real time; A5 wants ``< 0.5`` to leave the rest of the
    shack's software some headroom.
    """
    if input_duration_s <= 0.0:
        raise ValueError("input_duration_s must be positive")
    if process_wall_s < 0.0:
        raise ValueError("process_wall_s must not be negative")
    return process_wall_s / input_duration_s
