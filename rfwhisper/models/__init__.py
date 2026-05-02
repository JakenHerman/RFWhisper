"""Model registry, fetch, and ONNX metadata."""

from __future__ import annotations

from rfwhisper.models.base import Frame, Model
from rfwhisper.models.null_model import NullModel
from rfwhisper.models.registry import (
    ARTIFACTS,
    REPO_ROOT,
    ModelArtifact,
    load_model,
    make_session_options,
    model_path,
    resolve_providers,
)


def load(name: str, *, fallback_to_null: bool = False) -> Model:
    """Resolve `name` (`null` | `deepfilternet3` | `rnnoise`) to a Model instance."""
    return load_model(name, fallback_to_null=fallback_to_null)


__all__ = [
    "ARTIFACTS",
    "REPO_ROOT",
    "Frame",
    "Model",
    "ModelArtifact",
    "NullModel",
    "load",
    "load_model",
    "make_session_options",
    "model_path",
    "resolve_providers",
]
