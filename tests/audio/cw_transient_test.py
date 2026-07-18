"""A3 gate: denoising must not soften CW keying transients.

One of the two hard non-regression gates (the other is A2, FT8 decodes). A denoiser that
rounds off key-down edges destroys CW readability even while every broadband speech
metric improves — so this gate is a merge blocker, not an advisory. If it goes red, the
model change is wrong; do not relax the threshold.

Thresholds (ROADMAP A3): every onset within ±1.0 dB of raw, mean within ±0.5 dB.
"""

from __future__ import annotations

import json
import math
from pathlib import Path

import numpy as np
import pytest
from numpy.typing import NDArray

from rfwhisper.dsp.metrics import alignment_lag, keying_onset_rms
from rfwhisper.models.base import Model
from rfwhisper.models.registry import load_model
from tests.audio._cw_fixtures import synth_cw

# The gate itself is slow (full-length clips through a model) and carries `runslow`.
# The two harness self-checks below deliberately do not — they are cheap, and they are
# most useful running in the fast lane on every PR.
pytestmark = pytest.mark.a3

PER_ONSET_TOLERANCE_DB: float = 1.0
MEAN_TOLERANCE_DB: float = 0.5
ONSET_WINDOW_MS: float = 5.0

REPORT_DIR = Path(__file__).resolve().parents[2] / "build" / "audio-reports"


def _run_model(model: Model, signal: NDArray[np.float64]) -> NDArray[np.float64]:
    """Push ``signal`` through ``model`` one hop at a time, per the Model protocol.

    Whole hops only — a trailing partial hop is dropped rather than zero-padded, since
    padding would invent a key-up transient the source never had.
    """
    hop = model.hop
    n_hops = signal.size // hop
    out = np.empty(n_hops * hop, dtype=np.float64)
    for i in range(n_hops):
        chunk = signal[i * hop : (i + 1) * hop].astype(np.float32)
        out[i * hop : (i + 1) * hop] = model.process(chunk).astype(np.float64)
    return out


def _write_report(model_name: str, payload: dict[str, object]) -> None:
    """Persist the per-onset numbers for the release notes (CI uploads this directory)."""
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    path = REPORT_DIR / f"cw_transient_{model_name}.json"
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


@pytest.fixture(scope="module")
def model_under_test() -> tuple[str, Model]:
    """The shipped DFN3 model when its weights are present, else the identity NullModel.

    Falling back is deliberate. With NullModel the gate must pass trivially, which is how
    we know the harness itself is sound — if it can fail against a bit-exact identity, the
    measurement is broken, not the model.
    """
    model = load_model("deepfilternet3", fallback_to_null=True)
    name = type(model).__name__.lower()
    return name, model


@pytest.mark.runslow
@pytest.mark.parametrize("qrn", [False, True], ids=["flat_noise", "qrn"])
def test_cw_keying_onsets_survive_denoising(model_under_test: tuple[str, Model], qrn: bool) -> None:
    """Per-onset keying RMS must stay within ±1 dB, and the mean within ±0.5 dB."""
    model_name, model = model_under_test
    clip = synth_cw(wpm=25, freq_hz=600.0, snr_db=5.0, qrn=qrn)

    raw = _run_model(load_model("null"), clip.noisy)
    denoised = _run_model(model, clip.noisy)

    # A model with algorithmic latency shifts every edge; comparing at fixed indices
    # would score that delay as transient damage.
    lag = alignment_lag(denoised, raw, clip.sr)
    if lag > 0:
        denoised = denoised[lag:]
    elif lag < 0:
        raw = raw[-lag:]
    usable = min(raw.size, denoised.size)
    raw, denoised = raw[:usable], denoised[:usable]

    window = int(round(ONSET_WINDOW_MS * clip.sr / 1000.0))
    onsets = clip.onsets[clip.onsets + window <= usable]
    assert onsets.size >= 10, f"need a meaningful number of onsets to gate on, got {onsets.size}"

    rms_raw = keying_onset_rms(raw, onsets, window_ms=ONSET_WINDOW_MS, sr=clip.sr)
    rms_denoised = keying_onset_rms(denoised, onsets, window_ms=ONSET_WINDOW_MS, sr=clip.sr)

    with np.errstate(divide="ignore"):
        deltas_db = 20.0 * np.log10(rms_denoised / rms_raw)

    _write_report(
        model_name,
        {
            "model": model_name,
            "criterion": "A3",
            "wpm": clip.wpm,
            "noise": "qrn" if qrn else "flat",
            "snr_db": 5.0,
            "alignment_lag_samples": lag,
            "window_ms": ONSET_WINDOW_MS,
            "per_onset_delta_db": [round(float(d), 4) for d in deltas_db],
            "mean_delta_db": round(float(np.mean(deltas_db)), 4),
            "max_abs_delta_db": round(float(np.max(np.abs(deltas_db))), 4),
            "per_onset_tolerance_db": PER_ONSET_TOLERANCE_DB,
            "mean_tolerance_db": MEAN_TOLERANCE_DB,
        },
    )

    offenders = [
        f"onset {int(onset)} at t={onset / clip.sr:.3f} s: {delta:+.2f} dB "
        f"— exceeds ±{PER_ONSET_TOLERANCE_DB:.1f} dB"
        for onset, delta in zip(onsets, deltas_db, strict=True)
        if abs(float(delta)) > PER_ONSET_TOLERANCE_DB or not math.isfinite(float(delta))
    ]
    assert not offenders, (
        f"{model_name} damaged {len(offenders)} of {onsets.size} keying onsets:\n"
        + "\n".join(offenders)
    )

    mean_delta = float(np.mean(deltas_db))
    assert abs(mean_delta) <= MEAN_TOLERANCE_DB, (
        f"{model_name} mean keying-onset RMS shifted {mean_delta:+.2f} dB "
        f"— exceeds ±{MEAN_TOLERANCE_DB:.1f} dB"
    )


def test_harness_detects_a_softened_transient() -> None:
    """The gate must actually be able to fail.

    A gate that passes unconditionally is worse than no gate, so this feeds it a
    deliberately damaged signal — attack ramps stretched over 20 ms — and asserts the
    measurement notices. This is the check that keeps A3 honest.
    """
    clip = synth_cw(wpm=25, freq_hz=600.0, snr_db=20.0)
    window = int(round(ONSET_WINDOW_MS * clip.sr / 1000.0))

    damaged = clip.noisy.copy()
    ramp_len = int(round(0.020 * clip.sr))
    for onset in clip.onsets:
        end = min(int(onset) + ramp_len, damaged.size)
        ramp = np.linspace(0.0, 1.0, end - int(onset), dtype=np.float64)
        damaged[int(onset) : end] *= ramp

    onsets = clip.onsets[clip.onsets + window <= damaged.size]
    rms_raw = keying_onset_rms(clip.noisy, onsets, window_ms=ONSET_WINDOW_MS, sr=clip.sr)
    rms_damaged = keying_onset_rms(damaged, onsets, window_ms=ONSET_WINDOW_MS, sr=clip.sr)
    deltas_db = 20.0 * np.log10(rms_damaged / rms_raw)

    assert np.any(np.abs(deltas_db) > PER_ONSET_TOLERANCE_DB), (
        "softening every attack by 20 ms went undetected — the A3 measurement is broken"
    )


def test_null_model_is_bit_exact_through_the_harness() -> None:
    """Sanity check on `_run_model` itself: identity in, identity out."""
    clip = synth_cw(wpm=25, duration_s=2.0, snr_db=5.0)
    out = _run_model(load_model("null"), clip.noisy)
    assert np.array_equal(out, clip.noisy[: out.size].astype(np.float32).astype(np.float64))
