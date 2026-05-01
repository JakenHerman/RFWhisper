"""
Lightweight broadband noise suppressor for CI + headless hosts (no torch, no huge ONNX).

Not a replacement for DeepFilterNet3 on air; use for pipeline / latency / regression wiring.
"""

from __future__ import annotations

import numpy as np


def wiener_like_denoise(x: np.ndarray, sr: int, noise_gate_db: float = -35.0) -> np.ndarray:
    """
    Short-time magnitude-domain Wiener-ish mask (stationary noise assumption).

    Improves SNR on synthetic tone+noise tests; does not target ham-specific QRM.
    """
    x = np.asarray(x, dtype=np.float32)
    if x.size == 0:
        return x
    n_fft = 512 if sr >= 16_000 else 256
    hop = n_fft // 2
    window = np.hanning(n_fft).astype(np.float32)
    pad = (n_fft - (len(x) % hop)) % hop
    if pad:
        x = np.pad(x, (0, pad))
    nframes = (len(x) - n_fft) // hop + 1
    if nframes <= 0:
        return x[: len(x) - pad] if pad else x
    spec = np.empty((nframes, n_fft // 2 + 1), dtype=np.complex64)
    idx = 0
    for i in range(nframes):
        frame = x[idx : idx + n_fft] * window
        spec[i] = np.fft.rfft(frame)
        idx += hop
    mag = np.abs(spec)
    noise_floor = np.percentile(mag, 10, axis=0, keepdims=True).astype(np.float32)
    snr_est = mag / (noise_floor + 1e-8)
    mask = (snr_est**2) / (snr_est**2 + 1.0)
    # gentle floor so we never zero bands (helps CW/FT8 stability vs hard gate)
    mask = np.clip(mask, 0.15, 1.0)
    clean = spec * mask
    out = np.zeros(len(x), dtype=np.float32)
    wsum = np.zeros(len(x), dtype=np.float32)
    idx = 0
    for i in range(nframes):
        frame = np.fft.irfft(clean[i], n=n_fft).astype(np.float32) * window
        out[idx : idx + n_fft] += frame
        wsum[idx : idx + n_fft] += window**2
        idx += hop
    nz = wsum > 1e-12
    out[nz] /= wsum[nz]
    if pad:
        out = out[: -pad]
    return out.astype(np.float32, copy=False)
