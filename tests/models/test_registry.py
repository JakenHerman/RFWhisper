"""Unit tests for the EP resolver and the name → Model factory in registry.py."""

from __future__ import annotations

import warnings
from pathlib import Path

import pytest

from rfwhisper.models import NullModel, load, resolve_providers
from rfwhisper.models.registry import REPO_ROOT

# --- EP resolver -----------------------------------------------------------------------

ALL_PROVIDERS = [
    "CoreMLExecutionProvider",
    "DmlExecutionProvider",
    "CUDAExecutionProvider",
    "XnnpackExecutionProvider",
    "CPUExecutionProvider",
]


def test_resolve_providers_macos_prefers_coreml() -> None:
    """macOS runners must put CoreML at the head of the EP priority list."""
    got = resolve_providers(system="Darwin", available=ALL_PROVIDERS)
    assert got[0] == "CoreMLExecutionProvider"
    assert got[-1] == "CPUExecutionProvider"


def test_resolve_providers_windows_prefers_directml() -> None:
    """Windows runners must prefer DirectML and never expose CoreML."""
    got = resolve_providers(system="Windows", available=ALL_PROVIDERS)
    assert got[0] == "DmlExecutionProvider"
    # CoreML must not appear on Windows.
    assert "CoreMLExecutionProvider" not in got


def test_resolve_providers_linux_prefers_cuda_then_xnnpack() -> None:
    """Linux has no OS-specific accelerator EP; prefer CUDA, then XNNPACK, then CPU."""
    got = resolve_providers(system="Linux", available=ALL_PROVIDERS)
    assert got[0] == "CUDAExecutionProvider"
    assert got[1] == "XnnpackExecutionProvider"
    assert got[-1] == "CPUExecutionProvider"


def test_resolve_providers_skips_missing_eps() -> None:
    """An EP that isn't in `available` is dropped silently — no crash."""
    # Linux box without CUDA / XNNPACK installed — should fall straight through to CPU.
    got = resolve_providers(system="Linux", available=["CPUExecutionProvider"])
    assert got == ["CPUExecutionProvider"]


def test_resolve_providers_macos_without_coreml_falls_through() -> None:
    """macOS without CoreML EP installed must still produce a usable provider list."""
    # macOS runner where CoreML EP wasn't built into onnxruntime.
    got = resolve_providers(
        system="Darwin",
        available=["XnnpackExecutionProvider", "CPUExecutionProvider"],
    )
    assert got == ["XnnpackExecutionProvider", "CPUExecutionProvider"]


def test_resolve_providers_empty_available_warns_and_returns_cpu() -> None:
    """Pathological empty `available` warns and returns CPU rather than an empty list."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        got = resolve_providers(system="Linux", available=[])
    assert got == ["CPUExecutionProvider"]
    assert any(issubclass(w.category, RuntimeWarning) for w in caught)


# --- Name → Model factory --------------------------------------------------------------


def test_load_null_no_weights_needed() -> None:
    """`load("null")` must succeed with numpy only — no weights, no onnxruntime call."""
    # A4 gate runs on NullModel — must work with only numpy in the env.
    m = load("null")
    assert isinstance(m, NullModel)


def test_load_unknown_name_raises() -> None:
    """An unknown model name must raise ValueError, not silently fall back."""
    with pytest.raises(ValueError, match="Unknown model"):
        load("not_a_real_model")


def test_load_missing_weights_falls_back_when_allowed(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    """`fallback_to_null=True` returns NullModel + warning when weights are missing."""
    # Point REPO_ROOT at a temp dir so the dfn3 weights file definitely doesn't exist.
    monkeypatch.setattr("rfwhisper.models.registry.REPO_ROOT", tmp_path)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        m = load("deepfilternet3", fallback_to_null=True)
    assert isinstance(m, NullModel)
    assert any(issubclass(w.category, RuntimeWarning) for w in caught)


def test_load_missing_weights_raises_without_fallback(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    """Without `fallback_to_null` the missing-weights case must raise FileNotFoundError."""
    monkeypatch.setattr("rfwhisper.models.registry.REPO_ROOT", tmp_path)
    with pytest.raises(FileNotFoundError):
        load("deepfilternet3")


def test_load_rnnoise_missing_weights_falls_back(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    """RNNoise also gets the missing-weights fallback path (mirrors DFN3)."""
    monkeypatch.setattr("rfwhisper.models.registry.REPO_ROOT", tmp_path)
    m = load("rnnoise", fallback_to_null=True)
    assert isinstance(m, NullModel)


def test_resolve_intra_op_threads_default(monkeypatch: pytest.MonkeyPatch) -> None:
    """With the env var unset, the resolver returns the AGENTS default (2)."""
    from rfwhisper.models.registry import _resolve_intra_op_threads

    monkeypatch.delenv("RFWHISPER_ORT_INTRA_OP", raising=False)
    assert _resolve_intra_op_threads() == 2


def test_resolve_intra_op_threads_env_override(monkeypatch: pytest.MonkeyPatch) -> None:
    """`RFWHISPER_ORT_INTRA_OP=N` overrides the default thread count."""
    from rfwhisper.models.registry import _resolve_intra_op_threads

    monkeypatch.setenv("RFWHISPER_ORT_INTRA_OP", "4")
    assert _resolve_intra_op_threads() == 4


def test_resolve_intra_op_threads_invalid_warns(monkeypatch: pytest.MonkeyPatch) -> None:
    """Non-integer / non-positive values warn and fall back to the default."""
    from rfwhisper.models.registry import _resolve_intra_op_threads

    monkeypatch.setenv("RFWHISPER_ORT_INTRA_OP", "not-an-int")
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        n = _resolve_intra_op_threads()
    assert n == 2
    assert any(issubclass(w.category, RuntimeWarning) for w in caught)

    monkeypatch.setenv("RFWHISPER_ORT_INTRA_OP", "0")
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        n = _resolve_intra_op_threads()
    assert n == 2
    assert any(issubclass(w.category, RuntimeWarning) for w in caught)


def test_artifact_table_paths_are_relative() -> None:
    """ARTIFACTS entries must use relative paths so fetch.py joins them under REPO_ROOT."""
    # fetch.py joins REPO_ROOT with relpath; absolute paths would silently break that.
    from rfwhisper.models import ARTIFACTS

    for art in ARTIFACTS:
        assert not Path(art.relpath).is_absolute()
        assert art.relpath.startswith("models/")
    # Sanity: REPO_ROOT resolves to the repo, not e.g. site-packages.
    assert (REPO_ROOT / "pyproject.toml").is_file()
