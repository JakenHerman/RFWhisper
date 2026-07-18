"""CLI integration tests for `rfwhisper denoise` and `rfwhisper audio list` (refs A1)."""

from __future__ import annotations

import json
import types
from pathlib import Path
from typing import Any

import numpy as np
import pytest
from typer.testing import CliRunner

from rfwhisper.cli import EXIT_MODEL_LOAD, EXIT_OK, EXIT_UNSUPPORTED_INPUT, app

sf = pytest.importorskip("soundfile", reason="needs the [audio] extra")

runner = CliRunner()
SR = 48_000


@pytest.fixture(autouse=True)
def force_stub_engine(monkeypatch: pytest.MonkeyPatch) -> None:
    """Keep the CLI off torch/ONNX; these tests are about the front door, not the model."""
    monkeypatch.setenv("RFWHISPER_FORCE_STUB", "1")


def _tone(seconds: float = 1.0, sr: int = SR, seed: int = 42) -> np.ndarray:
    """A noisy tone — something with both signal and noise for the engine to chew on."""
    rng = np.random.default_rng(seed)
    t = np.arange(int(seconds * sr), dtype=np.float64) / sr
    clean = 0.5 * np.sin(2.0 * np.pi * 700.0 * t)
    return (clean + 0.1 * rng.standard_normal(t.size)).astype(np.float32)


def _write(path: Path, data: np.ndarray, sr: int = SR) -> Path:
    sf.write(str(path), data, sr, subtype="FLOAT")
    return path


def test_denoise_writes_output_and_report(tmp_path: Path) -> None:
    src = _write(tmp_path / "noisy.wav", _tone())
    out = tmp_path / "clean.wav"
    report = tmp_path / "report.json"

    result = runner.invoke(
        app, ["denoise", "-i", str(src), "-o", str(out), "--report", str(report)]
    )

    assert result.exit_code == EXIT_OK, result.output
    assert out.is_file()
    assert report.is_file()

    rep = json.loads(report.read_text(encoding="utf-8"))
    assert rep["model"] == "spectral_stub"
    assert rep["sr"] == SR
    assert rep["duration_s"] == pytest.approx(1.0, abs=0.01)
    assert rep["inference_time_ms"] > 0.0
    assert rep["rtf"] > 0.0
    assert rep["snr_gain_db"] is None
    assert rep["spectrogram_path"] is None


def test_report_schema_is_stable(tmp_path: Path) -> None:
    """docs/reports.md documents exactly these keys; adding one is a breaking change."""
    src = _write(tmp_path / "noisy.wav", _tone(0.5))
    out = tmp_path / "clean.wav"
    report = tmp_path / "report.json"

    runner.invoke(app, ["denoise", "-i", str(src), "-o", str(out), "--report", str(report)])
    rep = json.loads(report.read_text(encoding="utf-8"))

    assert set(rep) == {
        "model",
        "input",
        "output",
        "sr",
        "duration_s",
        "inference_time_ms",
        "rtf",
        "snr_gain_db",
        "spectrogram_path",
    }


def test_stdout_is_parseable_json_even_with_brackets_in_the_path(tmp_path: Path) -> None:
    """rich's print would treat `[ci]` as markup and mangle the JSON callers pipe out."""
    directory = tmp_path / "[ci] runs"
    directory.mkdir()
    src = _write(directory / "noisy.wav", _tone(0.5))
    out = directory / "clean.wav"

    result = runner.invoke(app, ["denoise", "-i", str(src), "-o", str(out)])

    assert result.exit_code == EXIT_OK, result.output
    rep = json.loads(result.stdout)
    assert "[ci] runs" in rep["input"]


def test_output_length_matches_input(tmp_path: Path) -> None:
    src = _write(tmp_path / "noisy.wav", _tone(2.0))
    out = tmp_path / "clean.wav"

    runner.invoke(app, ["denoise", "-i", str(src), "-o", str(out)])

    written, sr = sf.read(str(out), always_2d=True, dtype="float32")
    assert sr == SR
    assert written.shape[0] == pytest.approx(2 * SR, rel=0.001)


def test_reference_populates_snr_gain(tmp_path: Path) -> None:
    """With --reference the report carries a finite A1 number."""
    rng = np.random.default_rng(42)
    t = np.arange(SR, dtype=np.float64) / SR
    clean = (0.5 * np.sin(2.0 * np.pi * 700.0 * t)).astype(np.float32)
    noisy = (clean + 0.2 * rng.standard_normal(t.size)).astype(np.float32)

    src = _write(tmp_path / "noisy.wav", noisy)
    ref = _write(tmp_path / "clean_ref.wav", clean)
    out = tmp_path / "denoised.wav"
    report = tmp_path / "report.json"

    result = runner.invoke(
        app,
        [
            "denoise",
            "-i",
            str(src),
            "-o",
            str(out),
            "--reference",
            str(ref),
            "--report",
            str(report),
        ],
    )

    assert result.exit_code == EXIT_OK, result.output
    rep = json.loads(report.read_text(encoding="utf-8"))
    assert isinstance(rep["snr_gain_db"], float)


def test_reference_at_a_different_rate_is_skipped_not_fatal(tmp_path: Path) -> None:
    """A mismatched reference costs you the metric, not the denoise."""
    src = _write(tmp_path / "noisy.wav", _tone(1.0, sr=SR), sr=SR)
    ref = _write(tmp_path / "ref.wav", _tone(1.0, sr=16_000), sr=16_000)
    out = tmp_path / "clean.wav"
    report = tmp_path / "report.json"

    result = runner.invoke(
        app,
        [
            "denoise",
            "-i",
            str(src),
            "-o",
            str(out),
            "--reference",
            str(ref),
            "--report",
            str(report),
        ],
    )

    assert result.exit_code == EXIT_OK
    assert out.is_file()
    assert json.loads(report.read_text(encoding="utf-8"))["snr_gain_db"] is None


def test_multichannel_input_exits_2(tmp_path: Path) -> None:
    """Guessing which channel the operator meant would look like a broken denoiser."""
    stereo = np.stack([_tone(0.5), _tone(0.5, seed=7)], axis=1)
    src = _write(tmp_path / "stereo.wav", stereo)

    result = runner.invoke(app, ["denoise", "-i", str(src), "-o", str(tmp_path / "out.wav")])

    assert result.exit_code == EXIT_UNSUPPORTED_INPUT
    assert not (tmp_path / "out.wav").exists()


def test_unreadable_input_exits_2(tmp_path: Path) -> None:
    src = tmp_path / "not-audio.wav"
    src.write_bytes(b"this is not a wav file")

    result = runner.invoke(app, ["denoise", "-i", str(src), "-o", str(tmp_path / "out.wav")])

    assert result.exit_code == EXIT_UNSUPPORTED_INPUT


def test_missing_input_is_rejected_by_typer(tmp_path: Path) -> None:
    result = runner.invoke(
        app, ["denoise", "-i", str(tmp_path / "nope.wav"), "-o", str(tmp_path / "out.wav")]
    )
    assert result.exit_code != EXIT_OK


def test_model_load_failure_exits_3(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("RFWHISPER_FORCE_STUB", raising=False)
    src = _write(tmp_path / "noisy.wav", _tone(0.5))

    result = runner.invoke(
        app, ["denoise", "-i", str(src), "-o", str(tmp_path / "out.wav"), "-m", "no-such-model"]
    )

    assert result.exit_code == EXIT_MODEL_LOAD


def test_denoise_creates_missing_output_directory(tmp_path: Path) -> None:
    src = _write(tmp_path / "noisy.wav", _tone(0.5))
    out = tmp_path / "nested" / "dir" / "clean.wav"

    result = runner.invoke(app, ["denoise", "-i", str(src), "-o", str(out)])

    assert result.exit_code == EXIT_OK
    assert out.is_file()


def test_denoise_help_documents_every_flag_with_examples() -> None:
    result = runner.invoke(app, ["denoise", "--help"])
    assert result.exit_code == EXIT_OK
    for flag in ("--input", "--output", "--model", "--reference", "--report"):
        assert flag in result.output
    assert "rfwhisper denoise -i noisy.wav -o clean.wav" in result.output
    assert "Exit codes" in result.output


def _fake_sounddevice(devices: list[dict[str, Any]]) -> types.ModuleType:
    module = types.ModuleType("sounddevice")
    module.query_devices = lambda: devices  # type: ignore[attr-defined]
    module.query_hostapis = lambda: [{"name": "MME"}, {"name": "WASAPI"}]  # type: ignore[attr-defined]
    return module


def test_audio_list_renders_a_table(monkeypatch: pytest.MonkeyPatch) -> None:
    devices = [
        {
            "name": "Microphone (USB Audio)",
            "hostapi": 0,
            "max_input_channels": 2,
            "max_output_channels": 0,
            "default_samplerate": 48000.0,
        },
        {
            "name": "CABLE Input",
            "hostapi": 1,
            "max_input_channels": 0,
            "max_output_channels": 2,
            "default_samplerate": 44100.0,
        },
    ]
    monkeypatch.setitem(__import__("sys").modules, "sounddevice", _fake_sounddevice(devices))

    result = runner.invoke(app, ["audio", "list"])

    assert result.exit_code == EXIT_OK
    assert "Microphone" in result.output
    assert "CABLE Input" in result.output
    assert "WASAPI" in result.output
    assert "48000" in result.output


def test_audio_list_survives_having_no_devices(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(__import__("sys").modules, "sounddevice", _fake_sounddevice([]))
    result = runner.invoke(app, ["audio", "list"])
    assert result.exit_code == EXIT_OK
    assert "No audio devices found" in result.output
