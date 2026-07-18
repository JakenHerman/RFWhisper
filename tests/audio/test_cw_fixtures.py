"""Tests for the synthetic CW fixture generator itself.

The A3 gate trusts `synth_cw` to report exactly where the key went down. If those indices
are wrong the gate measures noise and passes regardless, so the fixture gets its own tests.
"""

from __future__ import annotations

import numpy as np
import pytest

from tests.audio._cw_fixtures import MORSE, CWClip, synth_cw

SR = 48_000


def test_dit_length_follows_the_paris_standard() -> None:
    """25 WPM means a 48 ms dit (1.2 / wpm)."""
    clip = synth_cw(wpm=25, sr=SR)
    assert clip.dit_samples == pytest.approx(0.048 * SR, rel=1e-6)


def test_onsets_land_on_actual_key_downs() -> None:
    """Every reported onset must sit at a rising edge, with silence just before it."""
    clip = synth_cw(wpm=25, snr_db=60.0)
    envelope = np.abs(clip.clean)
    peak = float(np.max(envelope))

    for onset in clip.onsets:
        i = int(onset)
        if i > 10:
            assert float(np.max(envelope[i - 10 : i])) < 0.05 * peak
        # The raised-cosine ramp means energy appears shortly after, not instantly.
        assert float(np.max(envelope[i : i + clip.dit_samples])) > 0.5 * peak


def test_onset_count_matches_the_message() -> None:
    """One onset per morse symbol across the whole message."""
    message = "CQ DE W1AW"
    expected = sum(len(MORSE[c]) for c in message.replace(" ", ""))
    clip = synth_cw(message=message)
    assert clip.onsets.size == expected


def test_keying_is_click_free() -> None:
    """A raised-cosine ramp, not a hard gate: the first sample after key-down is small."""
    clip = synth_cw(wpm=25, snr_db=60.0)
    peak = float(np.max(np.abs(clip.clean)))
    first = int(clip.onsets[0])
    assert abs(float(clip.clean[first])) < 0.01 * peak


def test_tone_frequency_is_where_it_was_asked_for() -> None:
    clip = synth_cw(wpm=25, freq_hz=600.0, snr_db=60.0)
    spec = np.abs(np.fft.rfft(clip.clean))
    freqs = np.fft.rfftfreq(clip.clean.size, d=1.0 / clip.sr)
    assert float(freqs[int(np.argmax(spec))]) == pytest.approx(600.0, abs=5.0)


def test_noisy_is_the_clean_signal_plus_noise_at_the_requested_snr() -> None:
    clip = synth_cw(wpm=25, snr_db=5.0)
    residual = clip.noisy - clip.clean
    ratio = float(np.dot(clip.clean, clip.clean)) / float(np.dot(residual, residual))
    assert 10.0 * np.log10(ratio) == pytest.approx(5.0, abs=1e-9)


def test_generation_is_deterministic() -> None:
    a = synth_cw(wpm=25, snr_db=5.0, seed=7)
    b = synth_cw(wpm=25, snr_db=5.0, seed=7)
    assert np.array_equal(a.noisy, b.noisy)
    assert np.array_equal(a.onsets, b.onsets)


def test_qrn_noise_differs_from_flat_noise() -> None:
    flat = synth_cw(wpm=25, snr_db=5.0, qrn=False, seed=7)
    crashes = synth_cw(wpm=25, snr_db=5.0, qrn=True, seed=7)
    assert np.array_equal(flat.clean, crashes.clean)
    assert not np.array_equal(flat.noisy, crashes.noisy)


def test_duration_truncation_drops_partial_elements() -> None:
    """Onsets whose element got cut off must not be reported."""
    full = synth_cw(wpm=25, snr_db=5.0)
    short = synth_cw(wpm=25, snr_db=5.0, duration_s=1.0)
    assert short.clean.size == SR
    assert short.onsets.size < full.onsets.size
    assert int(short.onsets[-1]) + short.dit_samples <= SR


def test_duration_padding_extends_with_silence() -> None:
    clip = synth_cw(wpm=25, snr_db=60.0, duration_s=30.0)
    assert clip.clean.size == 30 * SR


def test_returns_a_cw_clip_with_aligned_arrays() -> None:
    clip = synth_cw(wpm=25, snr_db=5.0)
    assert isinstance(clip, CWClip)
    assert clip.clean.shape == clip.noisy.shape


def test_rejects_unmappable_characters() -> None:
    with pytest.raises(ValueError, match="no morse mapping"):
        synth_cw(message="HELLO!")


def test_rejects_nonsense_parameters() -> None:
    with pytest.raises(ValueError, match="wpm must be positive"):
        synth_cw(wpm=0)
    with pytest.raises(ValueError, match="freq_hz must be in"):
        synth_cw(freq_hz=0.0)
    with pytest.raises(ValueError, match="freq_hz must be in"):
        synth_cw(freq_hz=SR)
