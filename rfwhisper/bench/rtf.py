"""Real-time factor for offline `process_file` (A5 gate helper)."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Annotated

import soundfile as sf
import typer
from rich import print

app = typer.Typer(help="RTF (wall/audio) on a WAV file.")


@app.command("file")
def rtf_file(
    path: Annotated[Path, typer.Argument(exists=True)],
    model: str = "spectral_stub",
) -> None:
    from rfwhisper.denoise.engine import select_engine

    if os.environ.get("RFWHISPER_FORCE_STUB") == "1":
        model = "spectral_stub"
    x, sr = sf.read(str(path), always_2d=True, dtype="float32")
    eng = select_engine(model)
    _, st = eng.process_file(x[:, 0], int(sr))
    print(f"RTF={st.rtf:.4f} (A5: < 0.5 on reference CPU) seconds={st.seconds_audio:.2f}")


def main() -> None:
    app()
