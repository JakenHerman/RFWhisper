"""CLI entrypoint (expanded in later milestones)."""

from __future__ import annotations

import argparse

from rfwhisper import __version__


def main() -> None:
    parser = argparse.ArgumentParser(prog="rfwhisper", description="RFWhisper denoiser CLI")
    parser.add_argument("--version", action="version", version=f"%(prog)s {__version__}")
    parser.parse_args()
    parser.print_help()
