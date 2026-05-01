"""
Identity pass-through model.

Used so CI can exercise the realtime path on matrix runners without Git LFS weights.
Honors the same `hop` / `sample_rate` contract as DFN3 / RNNoise so framing,
ring-buffer, and overlap-add machinery upstream is exercised end-to-end. Output is
bit-equal to input — gates A3 / A4 / A5 / A7 use this; A1 / A2 / A6 must skip-with-reason
since identity cannot improve SNR or decode counts.
"""

from __future__ import annotations

import numpy as np

from rfwhisper.constants import DFN_FRAME_SAMPLES, NATIVE_DFN_SR_HZ
from rfwhisper.models.base import Frame


class NullModel:
    sample_rate: int = NATIVE_DFN_SR_HZ
    hop: int = DFN_FRAME_SAMPLES

    def process(self, frame_48k: Frame) -> Frame:
        if frame_48k.shape != (self.hop,):
            raise ValueError(f"NullModel expects shape ({self.hop},); got {frame_48k.shape}")
        if frame_48k.dtype != np.float32:
            raise TypeError(f"NullModel expects float32; got {frame_48k.dtype}")
        return frame_48k
