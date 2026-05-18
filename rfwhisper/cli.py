"""Typer CLI — `rfwhisper` entrypoint."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Annotated, Any

import soundfile as sf
import typer
from rich import print

from rfwhisper.constants import DEFAULT_BLOCKSIZE
from rfwhisper.denoise.engine import select_engine

app = typer.Typer(no_args_is_help=True, add_completion=False)
models_app = typer.Typer(help="Model download and verification.")
app.add_typer(models_app, name="models")


def _resolve_model(name: str) -> str:
    if os.environ.get("RFWHISPER_FORCE_STUB") == "1":
        return "spectral_stub"
    return name


@app.command("denoise")
def denoise_cmd(
    input: Annotated[Path, typer.Option("--input", "-i", exists=True, dir_okay=False)],
    output: Annotated[Path, typer.Option("--output", "-o")],
    model: Annotated[str, typer.Option("--model", "-m")] = "deepfilternet3",
    report: Annotated[Path | None, typer.Option("--report")] = None,
) -> None:
    """Offline WAV denoise (float PCM; resamples to model rate internally)."""
    x, sr = sf.read(str(input), always_2d=True, dtype="float32")
    eng = select_engine(_resolve_model(model))
    y, stats = eng.process_file(x[:, 0], int(sr))
    sf.write(str(output), y, int(sr), subtype="FLOAT")
    rep: dict[str, Any] = {
        "model": _resolve_model(model),
        "input": str(input),
        "output": str(output),
        "rtf": stats.rtf,
        "seconds": stats.seconds_audio,
    }
    print(json.dumps(rep, indent=2))
    if report:
        report.write_text(json.dumps(rep, indent=2))


@app.command("denoise-live")
def denoise_live(
    in_dev: Annotated[int | None, typer.Option("--in")] = None,
    out_dev: Annotated[int | None, typer.Option("--out")] = None,
    model: Annotated[str, typer.Option("--model", "-m")] = "deepfilternet3",
    blocksize: Annotated[int, typer.Option("--blocksize")] = DEFAULT_BLOCKSIZE,
) -> None:
    """Real-time: input device index -> output device (virtual cable)."""
    from rfwhisper.realtime.processor import stream_denoise

    stream_denoise(in_dev, out_dev, model=_resolve_model(model), blocksize=blocksize)


audio_app = typer.Typer(help="PortAudio device helpers.")
app.add_typer(audio_app, name="audio")


@audio_app.command("list")
def audio_list() -> None:
    import sounddevice as sd

    print(sd.query_devices())


@app.command("gui")
def gui_cmd() -> None:
    print(
        "GUI: use `rfwhisper denoise-live` for v0.1; PySide6 GUI is planned (v0.4). "
        "See website/docs for virtual cable setup."
    )


@models_app.command("fetch")
def models_fetch(
    no_network: Annotated[bool, typer.Option("--no-network")] = False,
    verify_only: Annotated[bool, typer.Option("--verify-only")] = False,
) -> None:
    from rfwhisper.models import fetch as fetch_mod

    argv: list[str] = []
    if no_network:
        argv.append("--no-network")
    if verify_only:
        argv.append("--verify-only")
    raise typer.Exit(fetch_mod.main(argv))


def main() -> None:
    app()


# setuptools entry
def _entry() -> None:
    main()


if __name__ == "__main__":
    main()
