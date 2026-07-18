"""Typer CLI — `rfwhisper` entrypoint."""

from __future__ import annotations

import json
import math
import os
from pathlib import Path
from typing import Annotated, Any

import numpy as np
import typer
from numpy.typing import NDArray
from rich import print
from rich.console import Console
from rich.table import Table

from rfwhisper.constants import DEFAULT_BLOCKSIZE

# Exit codes are part of the CLI contract — scripts and CI gates branch on them.
EXIT_OK: int = 0
EXIT_UNSUPPORTED_INPUT: int = 2
EXIT_MODEL_LOAD: int = 3

app = typer.Typer(no_args_is_help=True, add_completion=False)
models_app = typer.Typer(help="Model download and verification.")
app.add_typer(models_app, name="models")


def _resolve_model(name: str) -> str:
    if os.environ.get("RFWHISPER_FORCE_STUB") == "1":
        return "spectral_stub"
    return name


def _err(message: str) -> None:
    """Print an error to stderr; stdout stays machine-readable for report JSON."""
    Console(stderr=True).print(f"[red]error:[/red] {message}")


def read_mono_wav(path: Path) -> tuple[NDArray[np.float32], int]:
    """Read a WAV as mono float32.

    Multi-channel input is an error rather than a silent downmix: which channel carries
    the signal is the operator's decision, and guessing wrong looks like a broken
    denoiser rather than a broken invocation.
    """
    import soundfile as sf

    try:
        data, sr = sf.read(str(path), always_2d=True, dtype="float32")
    except Exception as exc:  # soundfile raises RuntimeError / LibsndfileError
        _err(f"cannot read {path}: {exc}")
        raise typer.Exit(EXIT_UNSUPPORTED_INPUT) from exc

    if data.shape[1] != 1:
        _err(
            f"{path} has {data.shape[1]} channels; v0.1 handles mono only. "
            f"Extract one channel first, e.g. `sox {path.name} mono.wav remix 1`."
        )
        raise typer.Exit(EXIT_UNSUPPORTED_INPUT)
    if data.shape[0] == 0:
        _err(f"{path} contains no audio samples")
        raise typer.Exit(EXIT_UNSUPPORTED_INPUT)

    return np.asarray(data[:, 0], dtype=np.float32), int(sr)


def _load_engine(model: str) -> Any:
    """Resolve a model name to an engine, mapping any failure to exit code 3."""
    from rfwhisper.denoise.engine import select_engine

    try:
        return select_engine(model)
    except Exception as exc:
        _err(f"could not load model {model!r}: {exc}")
        raise typer.Exit(EXIT_MODEL_LOAD) from exc


def _snr_gain_db(
    reference: Path,
    noisy: NDArray[np.float32],
    denoised: NDArray[np.float32],
    sr: int,
) -> float | None:
    """Effective SNR gain against a clean reference (A1), or None if it can't be computed.

    A reference that doesn't line up with the input is a usage error worth reporting, but
    not worth discarding an otherwise good denoise over — so this warns and returns None
    rather than failing the run.
    """
    from rfwhisper.dsp.metrics import effective_snr_gain

    clean, ref_sr = read_mono_wav(reference)
    if ref_sr != sr:
        _err(f"reference is {ref_sr} Hz but input is {sr} Hz; skipping SNR gain")
        return None
    try:
        gain = effective_snr_gain(clean, noisy, denoised, sr)
    except ValueError as exc:
        _err(f"could not compute SNR gain: {exc}")
        return None
    return gain if math.isfinite(gain) else None


@app.command("denoise")
def denoise_cmd(
    input: Annotated[
        Path, typer.Option("--input", "-i", exists=True, dir_okay=False, help="Source WAV (mono).")
    ],
    output: Annotated[Path, typer.Option("--output", "-o", help="Destination WAV.")],
    model: Annotated[
        str, typer.Option("--model", "-m", help="Model name, or a path to an .onnx file.")
    ] = "deepfilternet3",
    reference: Annotated[
        Path | None,
        typer.Option(
            "--reference",
            exists=True,
            dir_okay=False,
            help="Clean reference WAV; adds snr_gain_db to the report (criterion A1).",
        ),
    ] = None,
    report: Annotated[
        Path | None, typer.Option("--report", help="Write the JSON report here as well as stdout.")
    ] = None,
) -> None:
    """Denoise a WAV file offline and write a JSON report.

    Mono only; the model resamples internally if the source is not at its native rate,
    and the output is written back at the source rate.

    Examples:

      rfwhisper denoise -i noisy.wav -o clean.wav

      rfwhisper denoise -i noisy.wav -o clean.wav -m spectral_stub

      rfwhisper denoise -i noisy.wav -o clean.wav --report report.json

      rfwhisper denoise -i mix.wav -o clean.wav --reference truth.wav

    Exit codes: 0 success, 2 unsupported input, 3 model load failure.
    """
    import soundfile as sf

    resolved = _resolve_model(model)
    x, sr = read_mono_wav(input)
    engine = _load_engine(resolved)

    y, stats = engine.process_file(x, sr)
    y = np.asarray(y, dtype=np.float32)

    output.parent.mkdir(parents=True, exist_ok=True)
    sf.write(str(output), y, sr, subtype="FLOAT")

    rep: dict[str, Any] = {
        "model": resolved,
        "input": str(input),
        "output": str(output),
        "sr": sr,
        "duration_s": round(stats.seconds_audio, 6),
        "inference_time_ms": round(stats.wall_seconds * 1000.0, 3),
        "rtf": round(stats.rtf, 6),
        "snr_gain_db": None,
        "spectrogram_path": None,
    }
    if reference is not None:
        gain = _snr_gain_db(reference, x, y, sr)
        rep["snr_gain_db"] = round(gain, 3) if gain is not None else None

    text = json.dumps(rep, indent=2)
    # typer.echo, not rich's print: rich would interpret square brackets in a path as
    # markup and silently mangle the JSON that callers pipe out of stdout.
    typer.echo(text)
    if report:
        report.parent.mkdir(parents=True, exist_ok=True)
        report.write_text(text, encoding="utf-8")


@app.command("denoise-live")
def denoise_live(
    in_dev: Annotated[int | None, typer.Option("--in", help="Input device index.")] = None,
    out_dev: Annotated[int | None, typer.Option("--out", help="Output device index.")] = None,
    model: Annotated[str, typer.Option("--model", "-m")] = "deepfilternet3",
    blocksize: Annotated[int, typer.Option("--blocksize")] = DEFAULT_BLOCKSIZE,
) -> None:
    """Real-time denoise from an input device to an output device (virtual cable).

    Examples:

      rfwhisper audio list

      rfwhisper denoise-live --in 3 --out 7
    """
    from rfwhisper.realtime.processor import stream_denoise

    stream_denoise(in_dev, out_dev, model=_resolve_model(model), blocksize=blocksize)


audio_app = typer.Typer(help="PortAudio device helpers.")
app.add_typer(audio_app, name="audio")


def _device_rows(devices: list[dict[str, Any]], hostapis: list[dict[str, Any]]) -> list[list[str]]:
    """Flatten sounddevice's query output into printable rows."""
    rows: list[list[str]] = []
    for idx, dev in enumerate(devices):
        api_idx = int(dev.get("hostapi", -1))
        api = hostapis[api_idx]["name"] if 0 <= api_idx < len(hostapis) else "?"
        rows.append(
            [
                str(idx),
                str(dev.get("name", "?")),
                str(api),
                str(int(dev.get("max_input_channels", 0))),
                str(int(dev.get("max_output_channels", 0))),
                f"{float(dev.get('default_samplerate', 0.0)):.0f}",
            ]
        )
    return rows


@audio_app.command("list")
def audio_list() -> None:
    """List audio devices with host API, channel counts, and default sample rate.

    Example:

      rfwhisper audio list
    """
    try:
        import sounddevice as sd
    except (ImportError, OSError) as exc:
        # OSError: PortAudio present as a Python package but no system library behind it.
        _err(f"sounddevice unavailable ({exc}); install the audio extra: pip install -e '.[audio]'")
        raise typer.Exit(EXIT_UNSUPPORTED_INPUT) from exc

    devices = [dict(d) for d in sd.query_devices()]
    hostapis = [dict(a) for a in sd.query_hostapis()]

    table = Table(title="Audio devices")
    for column in ("#", "Name", "Host API", "In", "Out", "Default SR"):
        table.add_column(column)
    for row in _device_rows(devices, hostapis):
        table.add_row(*row)

    console = Console()
    console.print(table)
    if not devices:
        console.print("[yellow]No audio devices found.[/yellow]")


@app.command("gui")
def gui_cmd() -> None:
    """Placeholder for the v0.4 GUI."""
    print(
        "GUI: use `rfwhisper denoise-live` for v0.1; PySide6 GUI is planned (v0.4). "
        "See website/docs for virtual cable setup."
    )


@models_app.command("fetch")
def models_fetch(
    no_network: Annotated[bool, typer.Option("--no-network")] = False,
    verify_only: Annotated[bool, typer.Option("--verify-only")] = False,
) -> None:
    """Download and verify model artifacts."""
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
