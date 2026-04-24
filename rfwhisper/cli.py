"""CLI entrypoint (expanded in later milestones)."""

from __future__ import annotations

import argparse


def main() -> None:
    parser = argparse.ArgumentParser(prog="rfwhisper", description="RFWhisper denoiser CLI")
    parser.add_argument("--version", action="version", version="rfwhisper 0.1.0")
    parser.parse_args()
    parser.print_help()
