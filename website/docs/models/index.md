---
id: index
title: Models & Training
sidebar_position: 1
description: DeepFilterNet3 primary, RNNoise fallback, and how to train your own.
slug: /models/
---

# Models & Training

RFWhisper ships two model families out of the box and makes it easy to add your own.

| Model | Flavor | Typical use | Size | CPU RTF (M1) | CPU RTF (RPi 5) |
|---|---|---|---|---|---|
| **DeepFilterNet3** (ham-tuned) | FP32 | Default on laptops/desktops | ~3.1 MB | 0.12 | 0.41 |
| **DeepFilterNet3** (INT8 QDQ) | INT8 | Laptops + RPi 5 | ~0.9 MB | 0.07 | 0.22 |
| **RNNoise-ham** (retuned) | FP32 | Pi 4 / Zero 2 W / anywhere CPU is tight | ~180 KB | 0.03 | 0.09 |
| **NullModel** (identity) | — | CI / dev runners without weights — `--model null` | 0 | n/a | n/a |
| Your fine-tune (v0.5) | FP32 / INT8 | Tuned to your QTH's noise | any | varies | varies |

:::note Every model ships with

- **SHA-256 pinned** in source (`rfwhisper/models/registry.py` — `ARTIFACTS`)
- A **model card** in [`docs/models/model-cards`](./model-cards) (architecture, training data, license, known failure modes, maintainer)
- **PyTorch ↔ ONNX parity** verification (RMS diff ≤ 1e-3, max ≤ 1e-2)
- **CW + FT8 regression** test results on the reference eval set
- Latency **p50 / p99** on reference hardware

:::

## Pick a model

```bash
# Default — pipeline will choose based on your hardware
rfwhisper denoise-live --model auto

# Explicit
rfwhisper denoise-live --model deepfilternet3 --model-variant int8
rfwhisper denoise-live --model rnnoise-ham
rfwhisper denoise-live --model my-qth-tuned-v1     # your fine-tune
```

## Model registry

Models are registered in [`rfwhisper/models/registry.py`](https://github.com/jakenherman/rfwhisper/blob/master/rfwhisper/models/registry.py) as a tuple of `ModelArtifact` dataclasses:

```python
ARTIFACTS: tuple[ModelArtifact, ...] = (
    ModelArtifact(
        name="DeepFilterNet3 ONNX (community export — verify before production)",
        relpath="models/deepfilternet3/dfn3.onnx",
        url="https://huggingface.co/aufklarer/DeepFilterNet3-ONNX/resolve/main/deepfilter.onnx",
        sha256="VERIFY_ON_FIRST_RUN",
        license_note="Upstream DeepFilterNet: MIT OR Apache-2.0. Third-party export; test parity.",
    ),
    # ...
)
```

`python -m rfwhisper.models.fetch` downloads, verifies, and caches these under the repo's `models/` directory. The first run captures the artefact's SHA-256 in a `.sha256.local` sidecar — pin that value into `ARTIFACTS` once you've validated parity.

### Loading a model in Python

```python
from rfwhisper.models import load

model = load("null")                                 # identity pass-through (CI / dev)
model = load("deepfilternet3")                       # raises if weights missing
model = load("deepfilternet3", fallback_to_null=True)  # warns and returns NullModel
```

The factory picks ONNX Runtime execution providers in priority order — **CoreML** on macOS, **DirectML** on Windows, then **CUDA → XNNPACK → CPU** on every platform — and applies `intra_op_num_threads=2`, `enable_cpu_mem_arena=True`, `ORT_ENABLE_ALL` per the AGENTS.md realtime budget. Missing EPs are skipped with a `RuntimeWarning`, never a crash.

## The non-regression gates (every model must pass)

1. **[CW keying transients](../quickstart/v0_1-test-guide#a3--cw-keying-transient-preservation)** — RMS within ±1 dB of raw on every transient.
2. **[FT8 decodes](../quickstart/v0_1-test-guide#a2--ft8-decode-non-regression)** — denoised decodes ≥ raw, zero false decodes.
3. **[No-op sanity](../quickstart/v0_1-test-guide#a6--no-op-sanity)** — clean in, clean out. PESQ drop ≤ 0.3, STOI drop ≤ 0.02.

A model that fails any of these is not merged, even if it scores higher on SNR gain.

## Next

- [Training](./training) — build a dataset + train from scratch
- [Fine-tuning](./fine-tuning) — adapt a stock model to your shack in ~30 minutes
- [Model Cards](./model-cards) — what's in each model, who maintains it, and how it was evaluated
