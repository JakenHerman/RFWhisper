"""Framing, periodic Hann windows, overlap-add, and offline STFT-frame helpers."""

from __future__ import annotations

import numpy as np
from numpy.lib.stride_tricks import sliding_window_view
from numpy.typing import NDArray

# DeepFilterNet3-style framing: 10 ms hop at native rates (ROADMAP / A4).
HOP_48K: int = 480
WIN_48K: int = 960
HOP_16K: int = 160
WIN_16K: int = 320


def hann_window(n: int) -> NDArray[np.float64]:
    """Periodic Hann of length ``n`` (``fftbins=True`` / DFN3 reference convention).

    Matches ``scipy.signal.get_window("hann", n, fftbins=True)`` (``sym=False``) within
    floating error, including SciPy's length-1 special case (all-ones window).
    """
    if n <= 0:
        raise ValueError("n must be positive")
    if n == 1:
        return np.ones(1, dtype=np.float64)
    idx = np.arange(n, dtype=np.float64)
    return 0.5 * (1.0 - np.cos(2.0 * np.pi * idx / float(n)))


class FrameBuffer:
    """Streaming sqrt-Hann analysis + COLA synthesis (WOLA) for one channel.

    * ``push`` ingests ``hop`` new samples.
    * When ``ready()``, ``next_frame`` returns ``x * sqrt(w)`` where ``w`` is the
      periodic Hann (``fftbins=True``); advancing the FIFO by ``hop``.
    * ``overlap_add`` does ``ola += processed * sqrt(w)`` and returns the next ``hop``
      samples from the accumulator (50 % overlap ⇒ overlapping Hann weights sum to 1).

    Samples are held in a fixed-capacity ring buffer (``win_size + hop``) so ``push``
    does not grow or concatenate on each hop—important for long streams and for
    eventual realtime use (see AGENTS.md: avoid per-callback allocations).
    """

    __slots__ = ("_cap", "_fifo", "_hop", "_n", "_ola", "_rd", "_win_sqrt", "_win_size")

    def __init__(self, win_size: int, hop: int) -> None:
        if win_size <= 0 or hop <= 0:
            raise ValueError("win_size and hop must be positive")
        if hop > win_size:
            raise ValueError("hop must not exceed win_size")
        self._win_size = win_size
        self._hop = hop
        w = hann_window(win_size)
        self._win_sqrt = np.sqrt(w)
        self._cap = win_size + hop
        self._fifo = np.zeros(self._cap, dtype=np.float64)
        self._rd = 0
        self._n = 0
        self._ola = np.zeros(win_size, dtype=np.float64)

    @property
    def win_size(self) -> int:
        return self._win_size

    @property
    def hop(self) -> int:
        return self._hop

    def push(self, hop_samples: NDArray[np.floating]) -> None:
        h = np.asarray(hop_samples, dtype=np.float64)
        if h.ndim != 1 or h.shape[0] != self._hop:
            raise ValueError(f"expected 1-D hop_samples with shape ({self._hop},), got {h.shape}")
        if self._n + self._hop > self._cap:
            raise ValueError(
                "analysis fifo full: call next_frame() whenever ready() before pushing more hops"
            )
        wr = (self._rd + self._n) % self._cap
        room = self._cap - wr
        if room >= self._hop:
            self._fifo[wr : wr + self._hop] = h
        else:
            self._fifo[wr:] = h[:room]
            self._fifo[: self._hop - room] = h[room:]
        self._n += self._hop

    def ready(self) -> bool:
        return self._n >= self._win_size

    def next_frame(self) -> NDArray[np.float64]:
        if not self.ready():
            raise RuntimeError("not enough samples for a full frame")
        wsz = self._win_size
        x = np.empty(wsz, dtype=np.float64)
        first = min(wsz, self._cap - self._rd)
        x[:first] = self._fifo[self._rd : self._rd + first]
        if first < wsz:
            x[first:] = self._fifo[: wsz - first]
        self._rd = (self._rd + self._hop) % self._cap
        self._n -= self._hop
        x *= self._win_sqrt
        return x

    def overlap_add(self, processed_frame: NDArray[np.floating]) -> NDArray[np.float64]:
        p = np.asarray(processed_frame, dtype=np.float64)
        if p.ndim != 1 or p.shape[0] != self._win_size:
            raise ValueError(
                f"expected 1-D processed_frame with shape ({self._win_size},), got {p.shape}"
            )
        self._ola += p * self._win_sqrt
        out: NDArray[np.float64] = np.asarray(self._ola[: self._hop].copy(), dtype=np.float64)
        # Shift accumulator left by one hop; zero the new tail for the next overlap.
        w = self._win_size
        h = self._hop
        self._ola[: w - h] = self._ola[h:w]
        self._ola[w - h :] = 0.0
        return out


def stft_frames(x: NDArray[np.floating], win_size: int, hop: int) -> NDArray[np.float64]:
    """Offline stack of sqrt-Hann analysis frames (matches :class:`FrameBuffer`).

    Frame ``k`` is ``x[k * hop : k * hop + win_size] * sqrt(hann_window(win_size))``.
    Only complete frames are returned, shape ``(n_frames, win_size)``.
    """
    if win_size <= 0 or hop <= 0:
        raise ValueError("win_size and hop must be positive")
    if hop > win_size:
        raise ValueError("hop must not exceed win_size")
    x = np.asarray(x, dtype=np.float64)
    if x.ndim != 1:
        raise ValueError(
            "expected a 1-D signal (shape (n,)); pass one channel at a time for multi-channel audio"
        )
    if x.size < win_size:
        return np.zeros((0, win_size), dtype=np.float64)
    w_sqrt = np.sqrt(hann_window(win_size))
    n_frames = 1 + (x.size - win_size) // hop
    if n_frames <= 0:
        return np.zeros((0, win_size), dtype=np.float64)
    windows = sliding_window_view(x, win_size)[::hop]
    if windows.shape[0] > n_frames:
        windows = windows[:n_frames]
    return windows * w_sqrt
