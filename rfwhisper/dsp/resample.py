"""Offline resampling to model-native rate (polyphase via scipy; not scipy.signal.resample)."""

from __future__ import annotations

import numpy as np
from scipy.signal import resample_poly


def to_native_rate(x: np.ndarray, sr_in: int, sr_out: int) -> np.ndarray:
    """Resample 1D float audio with polyphase `resample_poly` (good for streaming prep)."""
    if sr_in == sr_out:
        return np.asarray(x, dtype=np.float32)
    g = np.gcd(sr_in, sr_out)
    up = sr_out // g
    down = sr_in // g
    y = resample_poly(np.asarray(x, dtype=np.float64), up, down)
    return y.astype(np.float32, copy=False)


def next_chunk_size(block: int, sr_in: int, sr_out: int) -> int:
    """Approximate input samples needed for `block` output samples after resample."""
    if sr_in == sr_out:
        return block
    return int(np.ceil(block * sr_in / sr_out))
