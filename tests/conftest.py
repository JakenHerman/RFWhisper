"""Shared pytest configuration.

The audio-quality gates (A1–A6) are slow and are not part of the fast unit lane. They
are marked ``runslow`` and skipped unless ``--runslow`` is passed, which is exactly what
the ``audio-quality`` job in ``.github/workflows/basic-ci.yml`` does.
"""

from __future__ import annotations

import pytest


def pytest_addoption(parser: pytest.Parser) -> None:
    parser.addoption(
        "--runslow",
        action="store_true",
        default=False,
        help="run the slow audio-quality acceptance gates (A1-A6)",
    )


def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]) -> None:
    if config.getoption("--runslow"):
        return
    skip_slow = pytest.mark.skip(reason="needs --runslow (audio-quality acceptance gate)")
    for item in items:
        if "runslow" in item.keywords:
            item.add_marker(skip_slow)
