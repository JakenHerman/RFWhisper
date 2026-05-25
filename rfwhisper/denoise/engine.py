"""
Denoise engines: DeepFilterNet (optional), ONNXRuntime, and spectral stub.
"""

from __future__ import annotations

import abc
import os
import time
import warnings
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np

from rfwhisper.constants import NATIVE_DFN_SR_HZ
from rfwhisper.denoise.backends.spectral_stub import wiener_like_denoise
from rfwhisper.dsp.resample import to_native_rate


@dataclass
class ProcessStats:
    seconds_audio: float
    wall_seconds: float

    @property
    def rtf(self) -> float:
        return self.wall_seconds / self.seconds_audio if self.seconds_audio > 0 else float("inf")


class DenoiseEngine(abc.ABC):
    native_sr: int = NATIVE_DFN_SR_HZ

    @abc.abstractmethod
    def process(self, x: np.ndarray, sr: int) -> np.ndarray:
        """Mono float32 PCM; `sr` may differ from native — resample inside if needed."""

    def process_file(self, x: np.ndarray, sr: int) -> tuple[np.ndarray, ProcessStats]:
        t0 = time.perf_counter()
        y = self.process(x, sr)
        dt = time.perf_counter() - t0
        return y, ProcessStats(seconds_audio=float(len(x) / sr), wall_seconds=dt)


class SpectralStubEngine(DenoiseEngine):
    """CI / no-model path; maps to acceptance wiring, not production quality."""

    def process(self, x: np.ndarray, sr: int) -> np.ndarray:
        if x.ndim > 1:
            x = x[:, 0]
        x = np.asarray(x, dtype=np.float32)
        y = to_native_rate(x, sr, self.native_sr)
        y = wiener_like_denoise(y, self.native_sr)
        y = to_native_rate(y, self.native_sr, sr)
        return y.astype(np.float32, copy=False)


class TorchDfEngine(DenoiseEngine):
    """Uses upstream `deepfilternet` + torch (see https://github.com/Rikorose/DeepFilterNet)."""

    def __init__(self) -> None:
        from df import enhance, init_df  # type: ignore[import-untyped]

        self._enhance = enhance
        # Shipped default is usually DeepFilterNet2/3 per wheel; set env / checkpoint in training.
        self._model, self._state, _ = init_df()
        self.native_sr = int(self._state.sr())

    def process(self, x: np.ndarray, sr: int) -> np.ndarray:
        if x.ndim > 1:
            x = x[:, 0]
        x = np.asarray(x, dtype=np.float32)
        target = self.native_sr
        if sr != target:
            x = to_native_rate(x, sr, target)
        # enhance expects torch tensor [C, T]
        import torch

        t = torch.from_numpy(x).unsqueeze(0)
        with torch.inference_mode():
            out = self._enhance(self._model, self._state, t)
        y = out.squeeze(0).cpu().numpy().astype(np.float32)
        if sr != target:
            y = to_native_rate(y, target, sr)
        return y


class OnnxOrtEngine(DenoiseEngine):
    """
    Placeholder ONNXRuntime path for community end-to-end ONNX exports.

    Full DFN3 graphs need ERB + aux files; we run a simple graph if I/O are 1xTx1 float.
    """

    def __init__(self, onnx_path: Path) -> None:
        import onnxruntime as ort  # type: ignore[import-untyped]

        opts = ort.SessionOptions()
        opts.intra_op_num_threads = int(os.environ.get("RFWHISPER_ORT_INTRA_OP", "2"))
        opts.inter_op_num_threads = 1
        self._sess = ort.InferenceSession(
            str(onnx_path), providers=["CPUExecutionProvider"], sess_options=opts
        )

    def process(self, x: np.ndarray, sr: int) -> np.ndarray:
        if x.ndim > 1:
            x = x[:, 0]
        x = np.asarray(x, dtype=np.float32)
        if sr != self.native_sr:
            x = to_native_rate(x, sr, self.native_sr)
        inp = self._sess.get_inputs()[0]
        name = inp.name
        shape = inp.shape
        # Try (1, T) or (T,)
        xb = x.reshape(1, -1) if len(shape) == 2 else x
        feeds: dict[str, Any] = {name: xb.astype(np.float32)}
        out = self._sess.run(None, feeds)[0]
        y = np.squeeze(out).astype(np.float32)
        if sr != self.native_sr:
            y = to_native_rate(y, self.native_sr, sr)
        return y


def select_engine(model: str) -> DenoiseEngine:
    """
    Model names: `deepfilternet3` | `spectral_stub` | path to .onnx

    Env: RFWHISPER_FORCE_STUB=1 forces stub (CI without torch).
    """
    if os.environ.get("RFWHISPER_FORCE_STUB") == "1":
        return SpectralStubEngine()
    if model == "spectral_stub":
        return SpectralStubEngine()
    if model.endswith(".onnx"):
        return OnnxOrtEngine(Path(model))
    if model == "deepfilternet3":
        try:
            return TorchDfEngine()
        except (ImportError, ModuleNotFoundError) as exc:
            # Optional df / torch stack isn't installed — fall through to ONNX or stub.
            # Any other exception (e.g. corrupt checkpoint, GPU init failure) should
            # surface so it isn't silently masked by a stub.
            warnings.warn(
                f"TorchDfEngine unavailable; falling back: {exc}",
                RuntimeWarning,
                stacklevel=2,
            )
        p = Path(os.environ.get("RFWHISPER_ONNX", ""))
        if p.is_file():
            return OnnxOrtEngine(p)
        # Last resort: stub with warning
        warnings.warn(
            "deepfilternet3 requested but no torch/onnx backend available; "
            "using SpectralStubEngine (CI / no-model path)",
            RuntimeWarning,
            stacklevel=2,
        )
        return SpectralStubEngine()
    raise ValueError(f"Unknown model {model!r}")
