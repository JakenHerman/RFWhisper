"""
Model registry: artifact paths, execution-provider auto-select, ONNX session options.

`ARTIFACTS` and `model_path` describe on-disk locations for the fetch script (#12).
`resolve_providers`, `make_session_options`, and `load_model` wire those weights into
the realtime path with the correct EP priority for each OS.

Licensing: DeepFilterNet upstream is MIT or Apache-2.0; ONNX conversions inherit that.
See models/README.md and docs/datasets/SAMPLE_TRAINING_DATASET.md.
"""

from __future__ import annotations

import importlib
import platform
import warnings
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from rfwhisper.models.base import Model
from rfwhisper.models.null_model import NullModel


def _import_onnxruntime() -> Any:
    """
    Import onnxruntime via importlib so mypy doesn't choke on the missing stubs.

    onnxruntime ships no type information; the project's other call sites use
    `# type: ignore[import-untyped]`, but the right ignore-code differs between
    mypy 1.10 and 1.20. Routing through importlib avoids the mismatch.
    """
    return importlib.import_module("onnxruntime")


REPO_ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class ModelArtifact:
    """
    On-disk descriptor for a downloadable model artifact.

    Consumed by [fetch.py](rfwhisper/models/fetch.py) (#12). `sha256` is the literal
    `"VERIFY_ON_FIRST_RUN"` until a release pins a hash — see `models/README.md` for
    the bless-on-first-run flow.
    """

    name: str
    relpath: str
    url: str
    sha256: str  # set to "VERIFY_ON_FIRST_RUN" until release pins a hash
    license_note: str


ARTIFACTS: tuple[ModelArtifact, ...] = (
    ModelArtifact(
        name="DeepFilterNet3 ONNX (community export — verify before production)",
        relpath="models/deepfilternet3/dfn3.onnx",
        url="https://huggingface.co/aufklarer/DeepFilterNet3-ONNX/resolve/main/deepfilter.onnx",
        sha256="VERIFY_ON_FIRST_RUN",
        license_note="Upstream DeepFilterNet: MIT OR Apache-2.0. Third-party export; test parity.",
    ),
    ModelArtifact(
        name="ERB + window auxiliary (if required by your ONNX pack)",
        relpath="models/deepfilternet3/deepfilter-auxiliary.bin",
        url="https://huggingface.co/aufklarer/DeepFilterNet3-ONNX/resolve/main/deepfilter-auxiliary.bin",
        sha256="VERIFY_ON_FIRST_RUN",
        license_note="See HuggingFace model card; must match the ONNX you run.",
    ),
)


def model_path(name: str) -> Path:
    """Return the absolute on-disk directory for a given model name (e.g. `deepfilternet3`)."""
    return REPO_ROOT / "models" / name


# --- Execution-provider auto-select ----------------------------------------------------

# Per-OS EP priority. CUDA / XNNPACK / CPU are appended on every OS so the resolver
# falls through cleanly when the preferred EP isn't installed.
_OS_PREFERRED: dict[str, tuple[str, ...]] = {
    "Darwin": ("CoreMLExecutionProvider",),
    "Windows": ("DmlExecutionProvider",),
    "Linux": (),
}
_COMMON_FALLBACK: tuple[str, ...] = (
    "CUDAExecutionProvider",
    "XnnpackExecutionProvider",
    "CPUExecutionProvider",
)


def resolve_providers(
    *,
    system: str | None = None,
    available: list[str] | None = None,
) -> list[str]:
    """
    Return ONNX Runtime execution-provider names in priority order.

    Picks the OS-preferred EP first (CoreML on macOS, DirectML on Windows), then falls
    back through CUDA → XNNPACK → CPU. EPs not present in `available` are skipped, so a
    machine without DirectML installed silently falls through to the next option rather
    than crashing.

    `system` and `available` are injected for tests; production calls leave them None.
    """
    sys_name = system if system is not None else platform.system()
    avail = available if available is not None else _query_available_providers()

    priority: list[str] = []
    priority.extend(_OS_PREFERRED.get(sys_name, ()))
    priority.extend(_COMMON_FALLBACK)

    selected = [p for p in priority if p in avail]
    if not selected:
        # No EPs matched — onnxruntime will always advertise CPU, so this is only
        # reachable when caller mocked an empty list. Force CPU so we never hand
        # InferenceSession an empty providers= list.
        warnings.warn(
            "No matching ONNX execution providers; defaulting to CPUExecutionProvider",
            RuntimeWarning,
            stacklevel=2,
        )
        return ["CPUExecutionProvider"]
    return selected


def _query_available_providers() -> list[str]:
    """
    Query installed ONNX Runtime EPs, or return `["CPUExecutionProvider"]` when ORT
    isn't installed at all (NullModel-only environments).
    """
    try:
        ort = _import_onnxruntime()
    except ImportError:
        return ["CPUExecutionProvider"]
    providers: list[str] = list(ort.get_available_providers())
    return providers


def make_session_options() -> Any:
    """
    Build SessionOptions tuned for realtime per AGENTS §Real-Time Constraints.

    Returns ort.SessionOptions; typed as Any so this module imports without
    onnxruntime present (NullModel path on lean CI runners).
    """
    ort = _import_onnxruntime()
    opts = ort.SessionOptions()
    opts.intra_op_num_threads = 2
    opts.inter_op_num_threads = 1
    opts.enable_cpu_mem_arena = True
    opts.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
    return opts


# --- Name → Model factory --------------------------------------------------------------

# Relative path (under REPO_ROOT) to the primary weights file for each named model.
# Presence on disk is what gates "have weights" vs "need fallback".
_WEIGHTS: dict[str, str] = {
    "deepfilternet3": "models/deepfilternet3/dfn3.onnx",
    "rnnoise": "models/rnnoise/model.onnx",
}

ModelFactory = Callable[[Path], Model]


def _load_dfn3(weights: Path) -> Model:
    """Construct the DFN3 wrapper (#10). Raises ImportError until that wrapper lands."""
    from rfwhisper.models.dfn3 import DFN3  # type: ignore[import-not-found]

    return DFN3(  # type: ignore[no-any-return]
        weights,
        providers=resolve_providers(),
        session_options=make_session_options(),
    )


def _load_rnnoise(weights: Path) -> Model:
    """Construct the RNNoise wrapper (#11). Raises ImportError until that wrapper lands."""
    from rfwhisper.models.rnnoise import RNNoise  # type: ignore[import-not-found]

    return RNNoise(  # type: ignore[no-any-return]
        weights,
        providers=resolve_providers(),
        session_options=make_session_options(),
    )


_FACTORIES: dict[str, ModelFactory] = {
    "deepfilternet3": _load_dfn3,
    "rnnoise": _load_rnnoise,
}


def load_model(name: str, *, fallback_to_null: bool = False) -> Model:
    """
    Resolve `name` to a Model instance.

    Names: `null`, `deepfilternet3` (#10), `rnnoise` (#11). When weights or the wrapper
    module aren't present, `fallback_to_null=True` returns NullModel (with a warning);
    otherwise FileNotFoundError or ImportError propagates.
    """
    if name == "null":
        return NullModel()

    if name not in _FACTORIES:
        raise ValueError(f"Unknown model: {name!r}. Known: {sorted(['null', *_FACTORIES])}")

    weights = REPO_ROOT / _WEIGHTS[name]
    if not weights.is_file():
        return _fallback_or_raise(
            name,
            FileNotFoundError(f"Weights for {name!r} not found at {weights}"),
            fallback_to_null,
        )

    try:
        return _FACTORIES[name](weights)
    except ImportError as exc:
        return _fallback_or_raise(name, exc, fallback_to_null)


def _fallback_or_raise(name: str, exc: Exception, fallback_to_null: bool) -> Model:
    """Return NullModel with a warning when fallback is allowed; otherwise re-raise `exc`."""
    if fallback_to_null:
        warnings.warn(
            f"load_model({name!r}) falling back to NullModel: {exc}",
            RuntimeWarning,
            stacklevel=3,
        )
        return NullModel()
    raise exc
