"""python -m rfwhisper.bench"""

import typer

from rfwhisper.bench import latency, rtf

app = typer.Typer(help="Benchmark helpers (A4/A5).")
app.add_typer(latency.app, name="latency")
app.add_typer(rtf.app, name="rtf")

if __name__ == "__main__":
    app()
