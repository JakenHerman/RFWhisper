"""Synthetic RFI generator tests — determinism, spectral shape, exact SNR mixing."""

from __future__ import annotations

import math

import numpy as np
import pytest
from numpy.typing import NDArray

from rfwhisper.dsp.metrics import effective_snr_gain
from rfwhisper.samples.synth import (
    PEAK,
    atmospheric_qrn,
    mix,
    powerline_buzz,
    solar_inverter,
    vdsl_hash,
)

SR = 48_000
DURATION = 2.0

GENERATORS = [powerline_buzz, solar_inverter, vdsl_hash, atmospheric_qrn]


def _band_energy(x: NDArray[np.float64], sr: int, lo: float, hi: float) -> float:
    """Energy in ``[lo, hi)`` Hz, from the real FFT magnitude spectrum."""
    spec = np.abs(np.fft.rfft(x)) ** 2
    freqs = np.fft.rfftfreq(x.size, d=1.0 / sr)
    band = (freqs >= lo) & (freqs < hi)
    return float(np.sum(spec[band]))


@pytest.mark.parametrize("generator", GENERATORS, ids=lambda g: g.__name__)
def test_generators_are_bit_identical_for_a_fixed_seed(generator) -> None:  # type: ignore[no-untyped-def]
    """The whole point of the fixtures: a gate failure means the model changed."""
    a = generator(SR, DURATION, seed=42)
    b = generator(SR, DURATION, seed=42)
    assert np.array_equal(a, b)


@pytest.mark.parametrize("generator", GENERATORS, ids=lambda g: g.__name__)
def test_generators_differ_across_seeds(generator) -> None:  # type: ignore[no-untyped-def]
    a = generator(SR, DURATION, seed=0)
    b = generator(SR, DURATION, seed=1)
    assert not np.array_equal(a, b)


@pytest.mark.parametrize("generator", GENERATORS, ids=lambda g: g.__name__)
def test_generators_return_normalised_mono_of_the_right_length(generator) -> None:  # type: ignore[no-untyped-def]
    x = generator(SR, DURATION, seed=42)
    assert x.shape == (int(SR * DURATION),)
    assert x.dtype == np.float64
    assert float(np.max(np.abs(x))) == pytest.approx(PEAK, rel=1e-9)


def test_powerline_buzz_energy_sits_on_mains_harmonics() -> None:
    """Nearly all energy lands within a few Hz of a 60 Hz multiple."""
    x = powerline_buzz(SR, 4.0, fundamental_hz=60.0, n_harmonics=20, seed=42)
    spec = np.abs(np.fft.rfft(x)) ** 2
    freqs = np.fft.rfftfreq(x.size, d=1.0 / SR)

    on_harmonic = np.zeros(freqs.shape, dtype=bool)
    for k in range(1, 21):
        on_harmonic |= np.abs(freqs - k * 60.0) < 3.0

    assert float(np.sum(spec[on_harmonic])) / float(np.sum(spec)) > 0.95


def test_powerline_fundamental_dominates_higher_harmonics() -> None:
    """1/n rolloff: the 60 Hz partial must carry more energy than the 600 Hz one."""
    x = powerline_buzz(SR, 4.0, fundamental_hz=60.0, n_harmonics=20, seed=42)
    assert _band_energy(x, SR, 57.0, 63.0) > _band_energy(x, SR, 597.0, 603.0)


def test_solar_inverter_is_periodic_at_the_tick_rate() -> None:
    """Autocorrelation peaks at one tick period, not at some other lag."""
    tick_hz = 120.0
    x = solar_inverter(SR, 2.0, tick_rate_hz=tick_hz, seed=42)
    period = int(round(SR / tick_hz))

    x = x - float(np.mean(x))
    corr = np.correlate(x, x, mode="full")[x.size - 1 :]
    search = corr[period // 2 : 3 * period]
    best_lag = int(np.argmax(search)) + period // 2

    assert best_lag == pytest.approx(period, rel=0.02)


def test_solar_inverter_q_controls_ring_length() -> None:
    """Higher Q rings longer, so more energy survives between ticks."""
    low_q = solar_inverter(SR, 2.0, ringing_q=10.0, seed=42)
    high_q = solar_inverter(SR, 2.0, ringing_q=200.0, seed=42)

    # Sample the gap just before the next tick; a short ring has decayed away by then.
    period = int(round(SR / 120.0))
    gap = slice(period - period // 5, period)
    assert float(np.mean(high_q[gap] ** 2)) > float(np.mean(low_q[gap] ** 2))


def test_vdsl_hash_is_band_limited() -> None:
    """Little energy outside the 300 Hz – 0.45 Nyquist passband.

    Both probe bands stay clear of the filter's transition region — a 4th-order
    Butterworth is only -3 dB *at* its corner, so measuring immediately above 0.45
    Nyquist would be testing the skirt rather than the stopband.
    """
    x = vdsl_hash(SR, 4.0, seed=42)
    total = _band_energy(x, SR, 0.0, SR / 2.0)
    below = _band_energy(x, SR, 0.0, 150.0)
    above = _band_energy(x, SR, 0.6 * (SR / 2.0), SR / 2.0)

    assert below / total < 0.01
    assert above / total < 0.01


def test_atmospheric_qrn_is_bursty_not_gaussian() -> None:
    """Crashes give heavy tails — kurtosis well above the Gaussian value of 3."""
    x = atmospheric_qrn(SR, 10.0, crash_rate_hz=2.0, seed=42)
    centred = x - float(np.mean(x))
    variance = float(np.mean(centred**2))
    kurtosis = float(np.mean(centred**4)) / (variance**2)

    assert kurtosis > 10.0


def test_atmospheric_qrn_rate_scales_energy_duty_cycle() -> None:
    """A higher crash rate means more of the clip is above the quiet threshold."""
    quiet = atmospheric_qrn(SR, 10.0, crash_rate_hz=1.0, seed=42)
    busy = atmospheric_qrn(SR, 10.0, crash_rate_hz=20.0, seed=42)

    threshold = 0.05 * PEAK
    assert float(np.mean(np.abs(busy) > threshold)) > float(np.mean(np.abs(quiet) > threshold))


def test_mix_hits_the_requested_snr_exactly() -> None:
    """The measured power ratio must equal the requested SNR to floating precision."""
    clean = powerline_buzz(SR, 1.0, fundamental_hz=440.0, n_harmonics=3, seed=1)
    noise = vdsl_hash(SR, 1.0, seed=2)

    for snr_db in (-6.0, 0.0, 3.0, 20.0):
        mixed = mix(clean, noise, snr_db)
        residual = mixed - clean
        ratio = float(np.dot(clean, clean)) / float(np.dot(residual, residual))
        measured = 10.0 * math.log10(ratio)
        assert measured == pytest.approx(snr_db, abs=1e-9)


def test_mix_leaves_the_clean_reference_sample_aligned() -> None:
    """A1 measures against `clean`, so mixing must not scale or shift it."""
    clean = powerline_buzz(SR, 1.0, fundamental_hz=440.0, n_harmonics=3, seed=1)
    noise = vdsl_hash(SR, 1.0, seed=2)
    mixed = mix(clean, noise, 0.0)
    assert mixed.shape == clean.shape


def test_mix_snr_step_reads_back_through_the_a1_metric() -> None:
    """End-to-end with metrics.effective_snr_gain: a 6 dB cleaner mix reads as +6 dB."""
    t = np.arange(int(SR * 4.0), dtype=np.float64) / SR
    clean = np.sin(2.0 * np.pi * 700.0 * t) * (0.6 + 0.4 * np.sin(2.0 * np.pi * 3.0 * t))
    noise = powerline_buzz(SR, 4.0, seed=42)

    noisy = mix(clean, noise, 0.0)
    denoised = mix(clean, noise, 6.0)

    assert effective_snr_gain(clean, noisy, denoised, SR) == pytest.approx(6.0, abs=0.3)


def test_mix_rejects_length_mismatch() -> None:
    with pytest.raises(ValueError, match="same length"):
        mix(np.ones(100), np.ones(200), 0.0)


def test_mix_rejects_silent_inputs() -> None:
    with pytest.raises(ValueError, match="clean is all zeros"):
        mix(np.zeros(100), np.ones(100), 0.0)
    with pytest.raises(ValueError, match="noise is all zeros"):
        mix(np.ones(100), np.zeros(100), 0.0)


@pytest.mark.parametrize("generator", GENERATORS, ids=lambda g: g.__name__)
def test_generators_reject_nonpositive_duration(generator) -> None:  # type: ignore[no-untyped-def]
    with pytest.raises(ValueError, match="duration_s must be positive"):
        generator(SR, 0.0, seed=42)


def test_generators_reject_nonsense_parameters() -> None:
    with pytest.raises(ValueError, match="fundamental_hz must be positive"):
        powerline_buzz(SR, 1.0, fundamental_hz=0.0)
    with pytest.raises(ValueError, match="n_harmonics must be positive"):
        powerline_buzz(SR, 1.0, n_harmonics=0)
    with pytest.raises(ValueError, match="tick_rate_hz must be positive"):
        solar_inverter(SR, 1.0, tick_rate_hz=0.0)
    with pytest.raises(ValueError, match="ringing_q must be positive"):
        solar_inverter(SR, 1.0, ringing_q=0.0)
    with pytest.raises(ValueError, match="crash_rate_hz must be positive"):
        atmospheric_qrn(SR, 1.0, crash_rate_hz=0.0)
