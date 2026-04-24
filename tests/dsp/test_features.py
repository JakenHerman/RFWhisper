"""Tests for framing, Hann windows, overlap-add, and STFT frame stacking."""

from __future__ import annotations

import numpy as np
import pytest
from numpy.typing import NDArray

from rfwhisper.dsp.features import (
    HOP_16K,
    HOP_48K,
    WIN_16K,
    WIN_48K,
    FrameBuffer,
    hann_window,
    stft_frames,
)


def test_hann_matches_scipy_fftbins() -> None:
    scipy_signal = pytest.importorskip("scipy.signal", reason="scipy required for reference")
    get_window = scipy_signal.get_window
    for n in (320, 960, 100):
        ref = get_window("hann", n, fftbins=True).astype(np.float64)
        got = hann_window(n)
        np.testing.assert_allclose(got, ref, rtol=0, atol=1e-15)


def test_constants_ten_ms_hop() -> None:
    assert HOP_48K == pytest.approx(0.01 * 48_000)
    assert WIN_48K == 2 * HOP_48K
    assert HOP_16K == pytest.approx(0.01 * 16_000)
    assert WIN_16K == 2 * HOP_16K


def test_ola_hann_half_overlap_sums_to_unity() -> None:
    """WOLA gain: two 50 %-overlapped Hann windows sum to 1 (linear COLA)."""
    w = hann_window(WIN_48K)
    hop = HOP_48K
    assert hop * 2 == WIN_48K
    for i in range(hop, WIN_48K):
        s = float(w[i] + w[i - hop])
        assert s == pytest.approx(1.0, abs=1e-6)


def test_frame_buffer_sine_roundtrip_steady_state() -> None:
    sr = 48_000
    duration = 1.0
    f0 = 1_000.0
    t = np.arange(int(sr * duration), dtype=np.float64) / sr
    x = np.sin(2.0 * np.pi * f0 * t)

    fb = FrameBuffer(WIN_48K, HOP_48K)
    out_chunks: list[NDArray[np.float64]] = []

    def feed_hops(sig: NDArray[np.float64]) -> None:
        hop = HOP_48K
        n = sig.size
        pad = (hop - (n % hop)) % hop
        if pad:
            sig = np.concatenate((sig, np.zeros(pad, dtype=np.float64)))
        for i in range(0, sig.size, hop):
            fb.push(sig[i : i + hop])
            while fb.ready():
                frame = fb.next_frame()
                out_chunks.append(fb.overlap_add(frame))

    feed_hops(x)
    warmup = WIN_48K - HOP_48K
    # One extra window of synthesis tail beyond input length (COLA drain).
    need = warmup + x.size + WIN_48K
    while sum(c.size for c in out_chunks) < need:
        fb.push(np.zeros(HOP_48K, dtype=np.float64))
        while fb.ready():
            frame = fb.next_frame()
            out_chunks.append(fb.overlap_add(frame))

    y = np.concatenate(out_chunks)
    y_ss = y[warmup : warmup + x.size]
    # With F = (N - W) / H + 1 frames, COLA emits F * H = N - (W - H) steady-state
    # samples that match the input; the last (W - H) samples need further tail flush
    # beyond this acceptance test's scope.
    steady = x.size - (WIN_48K - HOP_48K)
    np.testing.assert_allclose(y_ss[:steady], x[:steady], rtol=1e-3, atol=5e-3)


def test_stft_frames_shape_and_matches_manual() -> None:
    x = np.random.default_rng(0).standard_normal(5_000).astype(np.float64)
    win, hop = 320, 160
    frames = stft_frames(x, win, hop)
    w_sqrt = np.sqrt(hann_window(win))
    n_frames = 1 + (x.size - win) // hop
    assert frames.shape == (n_frames, win)
    np.testing.assert_allclose(frames[3], x[3 * hop : 3 * hop + win] * w_sqrt, rtol=0, atol=1e-15)
