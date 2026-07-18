"""Manifest parsing and fetch/verify behaviour (refs #12).

Nothing here touches the network: downloads are faked, so the tests stay hermetic and
fast. The one thing they must prove is that a bad artifact is *rejected*, because an
artifact that silently stays on disk surfaces later as a baffling inference error.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from rfwhisper.models import fetch as fetch_mod
from rfwhisper.models.fetch import (
    ManifestEntry,
    ManifestError,
    load_manifest,
    main,
    sha256_file,
)

PAYLOAD = b"pretend this is an onnx graph"
PAYLOAD_SHA = hashlib.sha256(PAYLOAD).hexdigest()


def _manifest(tmp_path: Path, sha: str | None = PAYLOAD_SHA, name: str = "toy") -> Path:
    path = tmp_path / "manifest.json"
    path.write_text(
        json.dumps(
            {
                "version": 1,
                "artifacts": [
                    {
                        "name": name,
                        "relpath": "models/toy/toy.onnx",
                        "url": "https://example.invalid/toy.onnx",
                        "sha256": sha,
                        "size_bytes": len(PAYLOAD),
                        "license_note": "test fixture",
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    return path


@pytest.fixture
def sandbox(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """Point the module at a temp repo root and a temp manifest."""
    monkeypatch.setattr(fetch_mod, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(fetch_mod, "MANIFEST_PATH", _manifest(tmp_path))
    return tmp_path


@pytest.fixture
def fake_download(monkeypatch: pytest.MonkeyPatch) -> list[str]:
    """Replace the network with a local write; returns the list of URLs requested."""
    calls: list[str] = []

    def _fake(url: str, dest: Path) -> None:
        calls.append(url)
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(PAYLOAD)

    monkeypatch.setattr(fetch_mod, "_download", _fake)
    return calls


# --- Manifest parsing -----------------------------------------------------------------


def test_shipped_manifest_parses() -> None:
    """The real manifest must always load — `models fetch` is a first-run command."""
    manifest = load_manifest()
    assert "deepfilternet3" in manifest
    assert all(isinstance(e, ManifestEntry) for e in manifest.values())


def test_shipped_manifest_entries_have_usable_urls() -> None:
    for entry in load_manifest().values():
        assert entry.url.startswith("https://"), f"{entry.name} must be fetched over https"
        assert entry.relpath.startswith("models/")


def test_entry_is_unpinned_when_sha_is_null(tmp_path: Path) -> None:
    entry = load_manifest(_manifest(tmp_path, sha=None))["toy"]
    assert entry.pinned is False


def test_entry_is_pinned_when_sha_is_present(tmp_path: Path) -> None:
    assert load_manifest(_manifest(tmp_path))["toy"].pinned is True


def test_missing_manifest_is_a_clear_error(tmp_path: Path) -> None:
    with pytest.raises(ManifestError, match="manifest not found"):
        load_manifest(tmp_path / "nope.json")


def test_malformed_manifest_is_a_clear_error(tmp_path: Path) -> None:
    path = tmp_path / "manifest.json"
    path.write_text("{not json", encoding="utf-8")
    with pytest.raises(ManifestError, match="not valid JSON"):
        load_manifest(path)


def test_manifest_entry_missing_a_key_is_a_clear_error(tmp_path: Path) -> None:
    path = tmp_path / "manifest.json"
    path.write_text(json.dumps({"artifacts": [{"name": "toy"}]}), encoding="utf-8")
    with pytest.raises(ManifestError, match="missing required key"):
        load_manifest(path)


def test_duplicate_manifest_entries_are_rejected(tmp_path: Path) -> None:
    path = tmp_path / "manifest.json"
    entry = {
        "name": "toy",
        "relpath": "models/toy/toy.onnx",
        "url": "https://example.invalid/toy.onnx",
        "sha256": None,
    }
    path.write_text(json.dumps({"artifacts": [entry, entry]}), encoding="utf-8")
    with pytest.raises(ManifestError, match="duplicate manifest entry"):
        load_manifest(path)


def test_sha256_file_matches_hashlib(tmp_path: Path) -> None:
    target = tmp_path / "blob.bin"
    target.write_bytes(PAYLOAD)
    assert sha256_file(target) == PAYLOAD_SHA


# --- Acceptance criteria --------------------------------------------------------------


def test_fetching_the_null_model_is_a_noop(sandbox: Path) -> None:
    """Acceptance: `--model null` exits 0 and downloads nothing."""
    assert main(["--model", "null"]) == 0
    assert not (sandbox / "models").exists()


def test_a_corrupted_byte_is_refused_and_removed(sandbox: Path) -> None:
    """Acceptance: a hand-corrupted artifact makes fetch fail rather than load it."""
    dest = sandbox / "models" / "toy" / "toy.onnx"
    dest.parent.mkdir(parents=True)
    dest.write_bytes(PAYLOAD[:-1] + b"X")

    assert main(["--verify-only"]) == 1

    # Without --verify-only the corrupt copy is deleted, not left to confuse the loader.
    assert main(["--no-network"]) == 1
    assert not dest.exists()


def test_a_download_that_fails_verification_is_deleted(
    sandbox: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """A server that serves the wrong bytes must not leave them behind."""

    def _bad(url: str, dest: Path) -> None:
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(b"wrong bytes entirely")

    monkeypatch.setattr(fetch_mod, "_download", _bad)

    assert main([]) == 1
    assert not (sandbox / "models" / "toy" / "toy.onnx").exists()


# --- Flags ----------------------------------------------------------------------------


def test_fetch_downloads_a_missing_artifact(sandbox: Path, fake_download: list[str]) -> None:
    assert main([]) == 0
    assert fake_download == ["https://example.invalid/toy.onnx"]
    assert (sandbox / "models" / "toy" / "toy.onnx").read_bytes() == PAYLOAD


def test_a_valid_artifact_is_not_redownloaded(sandbox: Path, fake_download: list[str]) -> None:
    assert main([]) == 0
    assert main([]) == 0
    assert len(fake_download) == 1


def test_force_redownloads_a_valid_artifact(sandbox: Path, fake_download: list[str]) -> None:
    assert main([]) == 0
    assert main(["--force"]) == 0
    assert len(fake_download) == 2


def test_dry_run_downloads_nothing(sandbox: Path, fake_download: list[str]) -> None:
    assert main(["--dry-run"]) == 0
    assert fake_download == []
    assert not (sandbox / "models" / "toy" / "toy.onnx").exists()


def test_no_network_fails_when_the_artifact_is_absent(
    sandbox: Path, fake_download: list[str]
) -> None:
    assert main(["--no-network"]) == 1
    assert fake_download == []


def test_no_network_succeeds_when_the_artifact_is_already_there(
    sandbox: Path, fake_download: list[str]
) -> None:
    main([])
    assert main(["--no-network"]) == 0


def test_allow_missing_env_downgrades_a_missing_artifact(
    sandbox: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """CI without weights still needs a green run."""
    monkeypatch.setenv("RFWHISPER_ALLOW_MISSING_MODEL", "1")
    assert main(["--no-network"]) == 0


def test_one_failure_is_not_erased_by_a_later_success(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Two artifacts, the first unfetchable: the run must still report failure."""
    path = tmp_path / "manifest.json"
    path.write_text(
        json.dumps(
            {
                "artifacts": [
                    {
                        "name": "broken",
                        "relpath": "models/broken/broken.onnx",
                        "url": "https://example.invalid/broken.onnx",
                        "sha256": PAYLOAD_SHA,
                        "license_note": "",
                    },
                    {
                        "name": "fine",
                        "relpath": "models/fine/fine.onnx",
                        "url": "https://example.invalid/fine.onnx",
                        "sha256": PAYLOAD_SHA,
                        "license_note": "",
                    },
                ]
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(fetch_mod, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(fetch_mod, "MANIFEST_PATH", path)

    def _selective(url: str, dest: Path) -> None:
        if "broken" in url:
            raise OSError("connection reset")
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(PAYLOAD)

    monkeypatch.setattr(fetch_mod, "_download", _selective)

    assert main([]) == 1
    assert (tmp_path / "models" / "fine" / "fine.onnx").is_file()


def test_unknown_model_exits_2(sandbox: Path) -> None:
    assert main(["--model", "not-a-model"]) == 2


def test_model_flag_limits_what_is_fetched(sandbox: Path, fake_download: list[str]) -> None:
    assert main(["--model", "toy"]) == 0
    assert len(fake_download) == 1


def test_verify_only_passes_for_a_good_artifact(sandbox: Path, fake_download: list[str]) -> None:
    main([])
    assert main(["--verify-only"]) == 0


def test_verify_only_fails_when_the_artifact_is_missing(sandbox: Path) -> None:
    assert main(["--verify-only"]) == 1


# --- Unpinned artifacts ---------------------------------------------------------------


def test_an_unpinned_artifact_downloads_but_says_it_is_unverified(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    monkeypatch.setattr(fetch_mod, "REPO_ROOT", tmp_path)
    monkeypatch.setattr(fetch_mod, "MANIFEST_PATH", _manifest(tmp_path, sha=None))

    def _fake(url: str, dest: Path) -> None:
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(PAYLOAD)

    monkeypatch.setattr(fetch_mod, "_download", _fake)

    assert main([]) == 0
    err = capsys.readouterr().err
    assert "UNVERIFIED" in err
    assert PAYLOAD_SHA in err, "the message must give the hash to paste into the manifest"
