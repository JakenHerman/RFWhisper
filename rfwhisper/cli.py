"""CLI entrypoint — placeholder Typer app; subcommands land in later milestones."""

from __future__ import annotations

import typer

from rfwhisper import __version__

app = typer.Typer(
    name="rfwhisper",
    help="RFWhisper — real-time ML noise reduction for amateur radio.",
    add_completion=False,
)


@app.callback(invoke_without_command=True)
def main(
    ctx: typer.Context,
    version: bool = typer.Option(
        False,
        "--version",
        help="Show the version and exit.",
    ),
) -> None:
    """RFWhisper CLI."""
    if version:
        typer.echo(f"rfwhisper {__version__}")
        raise typer.Exit()
    if ctx.invoked_subcommand is None:
        typer.echo(ctx.get_help())
