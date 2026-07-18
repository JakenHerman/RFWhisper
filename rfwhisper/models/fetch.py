"""Fetch and verify model artifacts against a SHA-256-pinned manifest.

One-time pull plus verification. **Nothing here runs on a realtime path** — the audio
thread never touches the network, and `tests/models/test_no_runtime_network.py` enforces
that by importing the runtime modules with sockets disabled.

A corrupt artifact is deleted, not kept: a truncated download that silently stays on disk
would surface later as a baffling inference error rather than an obvious fetch failure.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import urllib.request
from dataclasses import dataclass
from pathlib import Path

from rich.console import Console

from rfwhisper.models.registry import REPO_ROOT

console = Console(stderr=True)

MANIFEST_PATH = Path(__file__).resolve().parent / "manifest.json"

# Names that resolve without any artifact at all.
BUILTIN_MODELS = frozenset({"null"})

_CHUNK = 1 << 20
_TIMEOUT_S = 120


@dataclass(frozen=True)
class ManifestEntry:
    """One downloadable artifact.

    ``sha256`` is None while the artifact is unpinned — fetch will still download it, but
    it prints the hash it saw and warns, rather than pretending the download was verified.
    """

    name: str
    relpath: str
    url: str
    sha256: str | None
    size_bytes: int | None
    license_note: str

    @property
    def pinned(self) -> bool:
        """True when a SHA-256 is recorded and can be enforced."""
        return self.sha256 is not None


class ManifestError(RuntimeError):
    """The manifest is missing, malformed, or internally inconsistent."""


def load_manifest(path: Path | None = None) -> dict[str, ManifestEntry]:
    """Parse the artifact manifest, keyed by model name."""
    source = path if path is not None else MANIFEST_PATH
    try:
        raw = json.loads(source.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise ManifestError(f"manifest not found at {source}") from exc
    except json.JSONDecodeError as exc:
        raise ManifestError(f"manifest at {source} is not valid JSON: {exc}") from exc

    entries: dict[str, ManifestEntry] = {}
    for item in raw.get("artifacts", []):
        try:
            entry = ManifestEntry(
                name=str(item["name"]),
                relpath=str(item["relpath"]),
                url=str(item["url"]),
                sha256=item["sha256"],
                size_bytes=item.get("size_bytes"),
                license_note=str(item.get("license_note", "")),
            )
        except KeyError as exc:
            raise ManifestError(f"manifest entry missing required key {exc}") from exc
        if entry.name in entries:
            raise ManifestError(f"duplicate manifest entry {entry.name!r}")
        entries[entry.name] = entry
    return entries


def sha256_file(path: Path) -> str:
    """SHA-256 of a file, read in chunks so a large model doesn't land in memory."""
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(_CHUNK), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _download(url: str, dest: Path) -> None:
    """Download to a `.part` file and move into place only once complete."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    part = dest.with_suffix(dest.suffix + ".part")
    console.print(f"Downloading {url} -> {dest}")
    request = urllib.request.Request(url, headers={"User-Agent": "rfwhisper-model-fetch/0.1"})
    try:
        with urllib.request.urlopen(request, timeout=_TIMEOUT_S) as response:  # noqa: S310
            with part.open("wb") as out:
                while True:
                    chunk = response.read(_CHUNK)
                    if not chunk:
                        break
                    out.write(chunk)
        part.replace(dest)
    finally:
        part.unlink(missing_ok=True)


def _status(entry: ManifestEntry, dest: Path) -> str:
    """One of ``missing``, ``ok``, ``unpinned``, ``corrupt``."""
    if not dest.is_file():
        return "missing"
    if not entry.pinned:
        return "unpinned"
    return "ok" if sha256_file(dest) == entry.sha256 else "corrupt"


def _select(
    manifest: dict[str, ManifestEntry], only: str | None
) -> tuple[list[ManifestEntry], int | None]:
    """Resolve ``--model`` to the entries to act on, or an exit code to return instead."""
    if only is None:
        return list(manifest.values()), None
    if only in BUILTIN_MODELS:
        console.print(f"{only}: built-in, nothing to fetch")
        return [], 0
    if only not in manifest:
        console.print(f"[red]Unknown model {only!r}. Known: {sorted([*BUILTIN_MODELS, *manifest])}")
        return [], 2
    return [manifest[only]], None


def main(argv: list[str] | None = None) -> int:
    """Entry point for ``python -m rfwhisper.models.fetch``."""
    parser = argparse.ArgumentParser(description="Fetch and verify model artifacts.")
    parser.add_argument("--model", help="Only act on this model (default: all).")
    parser.add_argument(
        "--dry-run", action="store_true", help="Report what would happen; download nothing."
    )
    parser.add_argument(
        "--force", action="store_true", help="Re-download even if a valid copy is present."
    )
    parser.add_argument("--no-network", action="store_true", help="Never download.")
    parser.add_argument("--verify-only", action="store_true", help="Check on-disk hashes and exit.")
    args = parser.parse_args(argv)

    try:
        manifest = load_manifest()
    except ManifestError as exc:
        console.print(f"[red]{exc}[/red]")
        return 1

    entries, early = _select(manifest, args.model)
    if early is not None:
        return early

    allow_missing = bool(os.environ.get("RFWHISPER_ALLOW_MISSING_MODEL"))
    failed = False

    for entry in entries:
        dest = REPO_ROOT / entry.relpath
        state = _status(entry, dest)

        if args.verify_only:
            failed |= _report_verify(entry, dest, state)
            continue

        if state == "corrupt":
            # Refuse to keep a file whose hash does not match what we pinned.
            console.print(
                f"[red]{entry.name}: SHA-256 mismatch at {dest} — expected {entry.sha256}, "
                f"got {sha256_file(dest)}. Removing the corrupt file.[/red]"
            )
            dest.unlink()
            state = "missing"

        if state in {"ok", "unpinned"} and not args.force:
            console.print(f"{entry.name}: OK ({dest})")
            if state == "unpinned":
                _warn_unpinned(entry, dest)
            continue

        if args.dry_run:
            size = f" (~{entry.size_bytes} bytes)" if entry.size_bytes else ""
            console.print(f"{entry.name}: would download {entry.url}{size} -> {dest}")
            continue

        if args.no_network:
            console.print(f"[red]{entry.name}: missing at {dest} and --no-network was given.[/red]")
            failed = failed or not allow_missing
            continue

        try:
            _download(entry.url, dest)
        except Exception as exc:
            console.print(f"[red]{entry.name}: fetch failed: {exc}[/red]")
            failed = failed or not allow_missing
            continue

        if not _verify_download(entry, dest):
            failed = True
            continue

        console.print(f"{entry.name}: fetched. License: {entry.license_note}")

    return 1 if failed else 0


def _verify_download(entry: ManifestEntry, dest: Path) -> bool:
    """Check a freshly downloaded artifact, deleting it if the hash is wrong."""
    got = sha256_file(dest)
    if not entry.pinned:
        _warn_unpinned(entry, dest, got)
        return True
    if got == entry.sha256:
        return True
    console.print(
        f"[red]{entry.name}: downloaded file failed verification — expected {entry.sha256}, "
        f"got {got}. Removing it.[/red]"
    )
    dest.unlink(missing_ok=True)
    return False


def _warn_unpinned(entry: ManifestEntry, dest: Path, digest: str | None = None) -> None:
    """Say plainly that an artifact is unverified, and give the hash to pin."""
    got = digest if digest is not None else sha256_file(dest)
    console.print(
        f"[yellow]{entry.name}: no SHA-256 pinned, so this artifact is UNVERIFIED. "
        f"Its hash is {got} — paste it into rfwhisper/models/manifest.json to pin it.[/yellow]"
    )


def _report_verify(entry: ManifestEntry, dest: Path, state: str) -> bool:
    """Print the outcome of --verify-only. Returns True if it counts as a failure."""
    if state == "missing":
        console.print(f"[red]{entry.name}: missing at {dest} (cannot verify)[/red]")
        return True
    if state == "unpinned":
        _warn_unpinned(entry, dest)
        return False
    if state == "corrupt":
        console.print(f"[red]{entry.name}: SHA-256 mismatch at {dest}[/red]")
        return True
    console.print(f"{entry.name}: verified ({dest})")
    return False


if __name__ == "__main__":
    raise SystemExit(main())
