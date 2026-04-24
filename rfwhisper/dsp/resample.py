"""Stateless polyphase resampling between 48 kHz and 16 kHz model I/O rates."""

from __future__ import annotations

import numpy as np
from numpy.typing import NDArray
from scipy.signal import resample_poly


def _require_mono_1d(x: np.ndarray) -> np.ndarray:
    if x.ndim != 1:
        raise ValueError(
            "expected a 1-D mono signal (shape (n,)); multi-channel audio must be "
            "resampled per channel"
        )
    return x


def resample_48k_to_16k(x: NDArray[np.floating]) -> NDArray[np.float64]:
    """48 kHz → 16 kHz via polyphase ``resample_poly`` (``up=1``, ``down=3``)."""
    x = _require_mono_1d(np.asarray(x, dtype=np.float64))
    return np.asarray(resample_poly(x, up=1, down=3), dtype=np.float64)


def resample_16k_to_48k(x: NDArray[np.floating]) -> NDArray[np.float64]:
    """16 kHz → 48 kHz via polyphase ``resample_poly`` (``up=3``, ``down=1``)."""
    x = _require_mono_1d(np.asarray(x, dtype=np.float64))
    return np.asarray(resample_poly(x, up=3, down=1), dtype=np.float64)
