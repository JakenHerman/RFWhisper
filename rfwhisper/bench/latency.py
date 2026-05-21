"""
Processing latency (file → file), not full audio-interface round trip.

Full round-trip (A4) needs impulse loop through hardware; see `scripts/v01_official_test.py`.
"""

from __future__ import annotations

import os
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import soundfile as sf
import typer
from rich import print

app = typer.Typer(help="Latency probes (offline / processing budget).")


@dataclass
class LatencyReport:
    p50_ms: float
    p99_ms: float
    n_chunks: int


def _measure_file(path: Path, model: str, block: int) -> LatencyReport:
    from rfwhisper.denoise.engine import select_engine

    if os.environ.get("RFWHISPER_FORCE_STUB") == "1":
        model = "spectral_stub"
    x, sr = sf.read(str(path), always_2d=True, dtype="float32")
    eng = select_engine(model)
    mono = x[:, 0]
    times: list[float] = []
    n = 0
    for i in range(0, len(mono) - block + 1, block):
        chunk = mono[i : i + block]
        t0 = time.perf_counter()
        _ = eng.process(chunk, int(sr))
        times.append((time.perf_counter() - t0) * 1000.0)
        n += 1
    if not times:
        return LatencyReport(0, 0, 0)
    times.sort()
    p50 = times[len(times) // 2]
    p99 = times[int(0.99 * (len(times) - 1))]
    return LatencyReport(p50, p99, n)


@app.command("file")
def latency_file(
    path: Path = typer.Argument(..., exists=True),
    model: str = "spectral_stub",
    block: int = 480,
) -> None:
    r = _measure_file(path, model, block)
    print(
        f"[green]p50={r.p50_ms:.2f} ms p99={r.p99_ms:.2f} ms n={r.n_chunks} (processing only)[/green]"
    )
    if r.p99_ms > 30 and model == "deepfilternet3":
        print(
            "[yellow]Note: A4 is end-to-end < 100 ms p99; this is chunk processing only.[/yellow]"
        )


def main() -> None:
    app()
