"""
Polyphase resampling helpers between common rates.

`to_native_rate` is the streaming-prep workhorse used by [engine.py](rfwhisper/denoise/engine.py)
to land arbitrary input rates on the model's native sample rate. `resample_48k_to_16k` /
`resample_16k_to_48k` are fixed-ratio convenience wrappers used by tests and any caller
that knows it only ever moves between those two rates (the v0.1 model I/O pair).

Both APIs use `scipy.signal.resample_poly` rather than FFT-based `resample` so we avoid
edge artefacts that would surface in the streaming path.
"""

from __future__ import annotations

import numpy as np
from numpy.typing import NDArray
from scipy.signal import resample_poly


def _require_mono_1d(x: np.ndarray) -> np.ndarray:
    """
    Validate that `x` is a 1-D mono signal; raise with a clear message otherwise.

    Multi-channel audio must be resampled per channel — silently flattening it would
    produce subtly wrong output instead of a loud failure.
    """
    if x.ndim != 1:
        raise ValueError(
            "expected a 1-D mono signal (shape (n,)); multi-channel audio must be "
            "resampled per channel"
        )
    return x


def to_native_rate(x: np.ndarray, sr_in: int, sr_out: int) -> np.ndarray:
    """
    Resample 1-D float audio from `sr_in` to `sr_out` with polyphase `resample_poly`.

    Returns float32 because the realtime path consumes float32 throughout. When
    `sr_in == sr_out` the input is returned as a float32 view (no copy where possible).
    """
    if sr_in == sr_out:
        return np.asarray(x, dtype=np.float32)
    g = np.gcd(sr_in, sr_out)
    up = sr_out // g
    down = sr_in // g
    y = resample_poly(np.asarray(x, dtype=np.float64), up, down)
    return y.astype(np.float32, copy=False)


def next_chunk_size(block: int, sr_in: int, sr_out: int) -> int:
    """
    Estimate input samples required to produce `block` output samples after resampling.

    Used by ring-buffer sizing in the streaming pipeline so we can pre-allocate the
    correct amount of input memory before each resample call.
    """
    if sr_in == sr_out:
        return block
    return int(np.ceil(block * sr_in / sr_out))


def resample_48k_to_16k(x: NDArray[np.floating]) -> NDArray[np.float64]:
    """48 kHz → 16 kHz via polyphase ``resample_poly`` (``up=1``, ``down=3``)."""
    x = _require_mono_1d(np.asarray(x, dtype=np.float64))
    return np.asarray(resample_poly(x, up=1, down=3), dtype=np.float64)


def resample_16k_to_48k(x: NDArray[np.floating]) -> NDArray[np.float64]:
    """16 kHz → 48 kHz via polyphase ``resample_poly`` (``up=3``, ``down=1``)."""
    x = _require_mono_1d(np.asarray(x, dtype=np.float64))
    return np.asarray(resample_poly(x, up=3, down=1), dtype=np.float64)
