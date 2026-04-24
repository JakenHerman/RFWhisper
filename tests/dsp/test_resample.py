"""Polyphase resampler tests."""

from __future__ import annotations

import numpy as np
from scipy.signal import correlate

import pytest

from rfwhisper.dsp.resample import resample_16k_to_48k, resample_48k_to_16k


def test_resample_rejects_non_1d() -> None:
    stereo = np.zeros((100, 2), dtype=np.float64)
    with pytest.raises(ValueError, match="1-D mono"):
        resample_48k_to_16k(stereo)


def test_resample_roundtrip_1khz_under_60_db() -> None:
    sr_in = 48_000
    duration = 6.0
    f0 = 1_000.0
    t = np.arange(int(sr_in * duration), dtype=np.float64) / sr_in
    x = np.sin(2.0 * np.pi * f0 * t)

    y = resample_48k_to_16k(x)
    assert y.shape == (int(sr_in * duration / 3),)
    z = resample_16k_to_48k(y)
    assert z.shape == x.shape

    trim = sr_in // 2
    x_mid = x[trim:-trim]
    z_mid = z[trim:-trim]

    corr = correlate(z_mid, x_mid, mode="full")
    lag = int(np.argmax(np.abs(corr)) - (z_mid.size - 1))

    if lag >= 0:
        m = min(z_mid.size - lag, x_mid.size)
        z_a = z_mid[lag : lag + m]
        x_a = x_mid[:m]
    else:
        lag_abs = -lag
        m = min(z_mid.size, x_mid.size - lag_abs)
        z_a = z_mid[:m]
        x_a = x_mid[lag_abs : lag_abs + m]

    gain = float(np.dot(z_a, x_a) / (np.dot(x_a, x_a) + 1e-20))
    err = z_a - gain * x_a
    rms_sig = float(np.sqrt(np.mean(x_a**2)))
    rms_err = float(np.sqrt(np.mean(err**2)))
    err_db = 20.0 * np.log10(rms_err / (rms_sig + 1e-20))
    assert err_db < -60.0
