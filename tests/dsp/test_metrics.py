"""Shared acceptance-gate metric tests (refs A1, A3, A5, A6)."""

from __future__ import annotations

import math

import numpy as np
import pytest
from numpy.typing import NDArray

from rfwhisper.dsp.metrics import (
    alignment_lag,
    effective_snr_gain,
    keying_onset_rms,
    pesq_score,
    rtf,
    stoi_score,
)

SR = 48_000


def _speech_like(n: int, seed: int = 42) -> NDArray[np.float64]:
    """Band-limited, amplitude-modulated tone stack — stands in for ham SSB audio."""
    rng = np.random.default_rng(seed)
    t = np.arange(n, dtype=np.float64) / SR
    x = np.zeros(n, dtype=np.float64)
    for f0 in (220.0, 480.0, 910.0, 1_600.0):
        x += rng.uniform(0.5, 1.0) * np.sin(2.0 * np.pi * f0 * t + rng.uniform(0.0, 2.0 * np.pi))
    envelope = 0.6 + 0.4 * np.sin(2.0 * np.pi * 3.0 * t)
    return x * envelope


def test_snr_gain_of_perfect_reconstruction_is_inf() -> None:
    """denoised == clean leaves no residual, so the gain sentinel is +inf."""
    clean = _speech_like(SR)
    rng = np.random.default_rng(42)
    noisy = clean + 0.5 * rng.standard_normal(clean.size)
    assert effective_snr_gain(clean, noisy, clean, SR) == math.inf


def test_snr_gain_is_nan_when_input_was_already_clean() -> None:
    """No noise to remove means "gain" is undefined rather than zero."""
    clean = _speech_like(SR)
    assert math.isnan(effective_snr_gain(clean, clean, clean, SR))


def test_snr_gain_matches_synthetic_3_db_improvement() -> None:
    """Halving noise power (-3 dB) must read back as +3 dB gain within ±0.3 dB."""
    clean = _speech_like(4 * SR)
    rng = np.random.default_rng(42)
    noise = rng.standard_normal(clean.size)
    noise *= math.sqrt(float(np.dot(clean, clean)) / float(np.dot(noise, noise)))  # 0 dB SNR

    noisy = clean + noise
    denoised = clean + noise * (10.0 ** (-3.0 / 20.0))

    assert effective_snr_gain(clean, noisy, denoised, SR) == pytest.approx(3.0, abs=0.3)


def test_snr_gain_tolerates_denoiser_delay_and_gain() -> None:
    """Algorithmic delay and a level change are alignment/scale, not noise."""
    clean = _speech_like(4 * SR)
    rng = np.random.default_rng(42)
    noise = 0.3 * rng.standard_normal(clean.size)
    delay = int(0.02 * SR)  # 20 ms, well inside MAX_ALIGN_MS

    noisy = clean + noise
    delayed = np.concatenate([np.zeros(delay), 2.0 * (clean + 0.1 * noise)])

    assert effective_snr_gain(clean, noisy, delayed, SR) == pytest.approx(
        20.0 * math.log10(1.0 / 0.1), abs=0.5
    )


def test_snr_gain_rejects_multichannel() -> None:
    clean = _speech_like(SR)
    stereo = np.zeros((clean.size, 2), dtype=np.float64)
    with pytest.raises(ValueError, match="1-D mono"):
        effective_snr_gain(clean, stereo, clean, SR)


@pytest.mark.parametrize("delay", [0, 1, 137, int(0.02 * SR)])
def test_alignment_lag_recovers_a_known_delay(delay: int) -> None:
    """A3 and A1 both depend on this: a delayed copy must report exactly its delay."""
    ref = _speech_like(2 * SR)
    delayed = np.concatenate([np.zeros(delay), ref])
    assert alignment_lag(delayed, ref, SR) == delay


def test_alignment_lag_is_sign_correct_for_early_signals() -> None:
    """Negative lag means the signal arrives before the reference."""
    ref = _speech_like(2 * SR)
    early = ref[240:]
    assert alignment_lag(early, ref, SR) == -240


def test_alignment_lag_is_bounded_by_the_search_window() -> None:
    """A delay past the window cannot be reported as if it were found."""
    ref = _speech_like(2 * SR)
    delayed = np.concatenate([np.zeros(int(0.2 * SR)), ref])
    lag = alignment_lag(delayed, ref, SR, max_align_ms=10.0)
    assert abs(lag) <= int(round(10.0 * SR / 1000.0))


def test_keying_onset_rms_matches_analytical_click_train() -> None:
    """Rectangular bursts of known amplitude read back as that amplitude within 1 %."""
    width = int(round(5.0 * SR / 1000.0))
    onsets = np.array([0, SR // 2, SR], dtype=np.int64)
    amplitudes = np.array([1.0, 0.5, 0.25])

    x = np.zeros(2 * SR, dtype=np.float64)
    for onset, amp in zip(onsets, amplitudes, strict=True):
        x[onset : onset + width] = amp

    measured = keying_onset_rms(x, onsets, window_ms=5.0, sr=SR)
    assert measured == pytest.approx(amplitudes, rel=0.01)


def test_keying_onset_rms_window_scales_with_sample_rate() -> None:
    """A 5 ms window at 16 kHz is 80 samples, so a 40-sample burst reads as -3 dB RMS."""
    sr = 16_000
    x = np.zeros(sr, dtype=np.float64)
    x[100:140] = 1.0
    measured = keying_onset_rms(x, np.array([100]), window_ms=5.0, sr=sr)
    assert measured[0] == pytest.approx(math.sqrt(40.0 / 80.0), rel=1e-9)


def test_keying_onset_rms_rejects_window_past_end() -> None:
    x = np.zeros(1_000, dtype=np.float64)
    with pytest.raises(ValueError, match="falls outside the signal"):
        keying_onset_rms(x, np.array([900]), window_ms=5.0, sr=SR)


def test_rtf_reports_wall_seconds_per_audio_second() -> None:
    assert rtf(10.0, 2.5) == pytest.approx(0.25)
    with pytest.raises(ValueError, match="input_duration_s must be positive"):
        rtf(0.0, 1.0)


def test_pesq_rejects_unsupported_sample_rate() -> None:
    """Guard runs before the lazy import, so this holds without the eval extra."""
    x = np.zeros(1_000, dtype=np.float64)
    with pytest.raises(ValueError, match="8000 or 16000"):
        pesq_score(x, x, 48_000)


@pytest.mark.parametrize("sr", [8_000, 16_000])
def test_pesq_clean_to_clean_near_ceiling(sr: int) -> None:
    """Identical signals should score at or near the PESQ ceiling (A6 reference floor)."""
    pytest.importorskip("pesq", reason="needs the [eval] extra")
    rng = np.random.default_rng(42)
    t = np.arange(3 * sr, dtype=np.float64) / sr
    x = 0.3 * np.sin(2.0 * np.pi * 300.0 * t) + 0.02 * rng.standard_normal(t.size)
    assert pesq_score(x, x, sr) >= 4.0


def test_stoi_clean_to_clean_is_near_one() -> None:
    pytest.importorskip("pystoi", reason="needs the [eval] extra")
    rng = np.random.default_rng(42)
    sr = 16_000
    t = np.arange(3 * sr, dtype=np.float64) / sr
    x = 0.3 * np.sin(2.0 * np.pi * 300.0 * t) + 0.02 * rng.standard_normal(t.size)
    assert stoi_score(x, x, sr) >= 0.99
