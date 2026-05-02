"""
Download pre-converted models with optional SHA-256 verification.

**No network in realtime paths** — this is a one-time pull + verify (AGENTS.md).
"""

from __future__ import annotations

import argparse
import hashlib
import os
import sys
import urllib.request
from pathlib import Path

from rich.console import Console

from rfwhisper.models.registry import ARTIFACTS, REPO_ROOT

console = Console(stderr=True)

LOCAL_SHA_SUFFIX = ".sha256.local"


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_suffix(dest.suffix + ".part")
    console.print(f"Downloading {url!s} -> {dest}")
    req = urllib.request.Request(url, headers={"User-Agent": "rfwhisper-model-fetch/0.1"})
    with urllib.request.urlopen(req, timeout=120) as r:  # noqa: S310 — curated URLs only
        tmp.write_bytes(r.read())
    tmp.replace(dest)


def _verify_or_bless(path: Path, expected: str) -> bool:
    got = _sha256_file(path)
    if expected == "VERIFY_ON_FIRST_RUN":
        local = path.with_name(path.name + LOCAL_SHA_SUFFIX)
        if not local.is_file():
            local.write_text(got + "\n")
            console.print(
                f"[yellow]Wrote {local} with SHA256 {got}. "
                "Pin this in models/registry.py for reproducible builds.[/yellow]"
            )
        return True
    return got == expected


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--no-network", action="store_true", help="Do not download")
    p.add_argument("--verify-only", action="store_true", help="Verify on-disk SHAs only")
    args = p.parse_args(argv)
    allow_network = not args.no_network
    verify_only = args.verify_only
    for art in ARTIFACTS:
        dest = REPO_ROOT / art.relpath
        if verify_only:
            if not dest.is_file():
                console.print(f"[red]Missing {dest} (cannot verify)[/red]")
                return 1
            if art.sha256 == "VERIFY_ON_FIRST_RUN":
                console.print(
                    f"{dest}: [green]{_sha256_file(dest)}[/green] (no expected hash pinned yet)"
                )
            elif _sha256_file(dest) != art.sha256:
                console.print(f"[red]SHA256 mismatch for {dest}[/red]")
                return 1
            continue
        on_disk_ok = dest.is_file() and (
            art.sha256 == "VERIFY_ON_FIRST_RUN" or _sha256_file(dest) == art.sha256
        )
        if on_disk_ok:
            if art.sha256 == "VERIFY_ON_FIRST_RUN":
                _verify_or_bless(dest, art.sha256)
            console.print(f"OK: {dest}")
            continue
        if not allow_network:
            console.print(
                f"[red]Missing {dest} (use --no-network only if files are already present).[/red]"
            )
            return 0 if os.environ.get("RFWHISPER_ALLOW_MISSING_MODEL") else 1
        try:
            _download(art.url, dest)
        except Exception as e:
            console.print(f"[red]Fetch failed: {e} — place artefact manually or try later[/red]")
            return 0 if os.environ.get("RFWHISPER_ALLOW_MISSING_MODEL") else 1
        if not _verify_or_bless(dest, art.sha256):
            console.print(f"[red]Hash mismatch {dest}[/red]")
            return 1
        console.print(f"License: {art.license_note}")
    return 0
