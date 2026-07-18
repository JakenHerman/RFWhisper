# Models

Where model artifacts live, how to fetch them, and how to swap in your own.

## The short version

```console
$ rfwhisper models fetch
$ python -m rfwhisper.models.fetch --dry-run     # see what it would do first
```

Artifacts land under `models/` at the repo root. Nothing is downloaded at runtime — see
[Local-first](#local-first) below.

## The manifest

[`rfwhisper/models/manifest.json`](../rfwhisper/models/manifest.json) is the single source
of truth for what can be fetched. Each entry carries:

| Field          | Meaning                                                           |
| -------------- | ----------------------------------------------------------------- |
| `name`         | What you pass to `--model`                                        |
| `relpath`      | Where it lands, relative to the repo root                         |
| `url`          | Download URL (https only)                                         |
| `sha256`       | Pinned hash, or `null` if not yet pinned                          |
| `size_bytes`   | Advisory, shown in `--dry-run`; never enforced                    |
| `license_note` | Upstream licence, printed after a successful fetch                |

### Why JSON and not YAML

The original issue (#12) called for `manifest.yaml`. It is JSON because `models fetch` is
the *first* command a new user runs, and JSON parses with the standard library. YAML would
mean either adding `pyyaml` to the base dependencies — which pulls against #91's goal of a
numpy-only base install — or making the first-run command require an optional extra. If
you'd rather have YAML, that's a reasonable call to overrule; it's a small change.

### Pinned vs unpinned

A `null` hash means **the artifact is not verified**. Fetch will still download it, but it
prints the hash it received and warns loudly:

```
deepfilternet3: no SHA-256 pinned, so this artifact is UNVERIFIED.
Its hash is 9f2c… — paste it into rfwhisper/models/manifest.json to pin it.
```

Both shipped entries are currently unpinned, because hosting hasn't been settled and the
URLs point at third-party community exports. **Pin them before any release.** Verify what
you got before trusting it — the licence note on each entry is not a parity guarantee.

## Commands

| Flag            | Effect                                                    |
| --------------- | ---------------------------------------------------------- |
| *(none)*        | Download anything missing or hash-mismatched               |
| `--model NAME`  | Act on one artifact. `--model null` is a no-op, exits 0    |
| `--dry-run`     | Report what would happen; download nothing                 |
| `--force`       | Re-download even if a valid copy is present                |
| `--no-network`  | Never download; fail if something is missing               |
| `--verify-only` | Check on-disk hashes and exit                              |

Exit code is `0` on success, `1` if anything failed, `2` for an unknown `--model`.

Set `RFWHISPER_ALLOW_MISSING_MODEL=1` to downgrade "artifact missing" from a failure to a
warning — CI uses this so the unit lane stays green without weights.

### Corrupt artifacts are deleted

If an on-disk file does not match its pinned hash, fetch removes it and exits non-zero.
The same applies to a download that arrives corrupt. This is deliberate: a truncated file
that quietly stays on disk resurfaces later as a baffling inference error rather than an
obvious fetch failure.

## Local-first

No RFWhisper module reaches the network outside `models fetch`. This is enforced, not
merely documented: `tests/models/test_no_runtime_network.py` imports every runtime module
in a subprocess with `socket.socket` disabled and fails if any of them tries to connect.
That test also checks its own guard trips on a real socket call, so it cannot pass
vacuously.

If you add a module that legitimately needs the network, it does not belong on the audio
path — and the audit list in that test is where to justify the exception.

## Swapping in your own model

`select_engine()` accepts a path ending in `.onnx`, so any compatible graph works without
touching the manifest:

```console
$ rfwhisper denoise -i noisy.wav -o clean.wav -m /path/to/your.onnx
```

`RFWHISPER_ONNX=/path/to/your.onnx` does the same thing for the `deepfilternet3` name when
the torch stack isn't installed.

## Exporting DFN3 to ONNX yourself

**Not yet written.** #12 also asks for `scripts/export_dfn3_onnx.py` — PyTorch → ONNX at
opset 17 with a parity check against the PyTorch outputs. It needs the upstream
DeepFilterNet checkpoint and a working torch install to write honestly, so it is
deliberately left for someone who can actually run it rather than shipped unvalidated.

Until then the manifest points at a community export, which is exactly why those entries
are unpinned and flagged.
