"""
Model protocol shared by every denoiser wrapper (DFN3, RNNoise, NullModel).

The realtime path consumes models through this protocol — see #10 / #11 for concrete
implementations and `rfwhisper.realtime.processor` for the consumer side.
"""

from __future__ import annotations

from typing import Protocol, runtime_checkable

import numpy as np
from numpy.typing import NDArray

Frame = NDArray[np.float32]


@runtime_checkable
class Model(Protocol):
    """One-frame-in / one-frame-out denoiser at a fixed sample rate."""

    sample_rate: int
    hop: int

    def process(self, frame_48k: Frame) -> Frame:
        """Process a single mono float32 frame of length `hop`. Returns same shape."""
        ...
