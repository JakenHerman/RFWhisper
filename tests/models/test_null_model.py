from __future__ import annotations

import numpy as np
import pytest

from rfwhisper.constants import DFN_FRAME_SAMPLES, NATIVE_DFN_SR_HZ
from rfwhisper.models import NullModel, load


def test_null_model_attributes() -> None:
    m = NullModel()
    assert m.sample_rate == NATIVE_DFN_SR_HZ
    assert m.hop == DFN_FRAME_SAMPLES


def test_null_model_identity_bit_equal() -> None:
    rng = np.random.default_rng(0xC0FFEE)
    m = NullModel()
    for _ in range(8):
        x = rng.standard_normal(DFN_FRAME_SAMPLES, dtype=np.float32)
        y = m.process(x)
        # Bit-equal — array_equal compares element-wise without tolerance.
        assert np.array_equal(y, x)
        assert y.dtype == np.float32
        assert y.shape == (DFN_FRAME_SAMPLES,)


def test_null_model_rejects_wrong_shape() -> None:
    m = NullModel()
    with pytest.raises(ValueError, match="expects shape"):
        m.process(np.zeros(DFN_FRAME_SAMPLES - 1, dtype=np.float32))


def test_null_model_rejects_wrong_dtype() -> None:
    m = NullModel()
    with pytest.raises(TypeError, match="float32"):
        m.process(np.zeros(DFN_FRAME_SAMPLES, dtype=np.float64))  # type: ignore[arg-type]


def test_load_null_returns_null_model() -> None:
    m = load("null")
    assert isinstance(m, NullModel)
