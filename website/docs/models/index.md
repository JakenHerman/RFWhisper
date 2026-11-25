---
id: index
title: Models & Training
sidebar_position: 1
description: What the beta ships (DeepFilterNet3, passthrough), what's planned, and how to train your own.
slug: /models/
---

# Models & Training

## What the beta ships

| Model (`--model`) | What it is | Use it for |
|---|---|---|
| `deepfilternet3` *(default)* | Stock DeepFilterNet3 (MIT/Apache-2.0), run by upstream's Rust engine on `tract`; weights bundled in the binary | SSB / AM / FM voice |
| `passthrough` (alias `none`) | No processing | CW, FT8/FT4, RTTY, PSK; A/B reference |
| `spectral_stub` | Classical spectral subtraction, about +1 dB | CI, and `--no-default-features` builds |

Measured on an Apple Silicon laptop: RTF ≈ 0.02 (about 50× faster than real
time), 0.1–0.5 ms per 10 ms frame, and a fixed 30 ms of algorithmic latency (the
model's look-ahead).

## Planned

| Model | Flavor | Typical use | Status |
|---|---|---|---|
| **DeepFilterNet3, ham-tuned** | FP32 / INT8 | Default once trained | v0.5 training pipeline |
| **RNNoise-ham** | FP32 | Pi 4 / Zero 2 W / anywhere CPU is tight | [#11](https://github.com/JakenHerman/RFWhisper/issues/11) |
| Your fine-tune | FP32 / INT8 | Tuned to your QTH's noise | v0.5 |
| Custom ONNX (`--model path/to/model.onnx`) | any | Community models | reserved; errors until the ONNX backend lands |

:::note What every shipped model must come with

- A **model card** in [Model Cards](./model-cards.md) (architecture, training data, license, known failure modes, maintainer)
- **CW + FT8 regression** results on the reference eval set (A2 / A3)
- Latency **p50 / p99** on reference hardware
- For downloaded (non-bundled) models: a **SHA-256 pinned** in source and, for ONNX exports, **PyTorch ↔ ONNX parity** (RMS diff ≤ 1e-3, max ≤ 1e-2)

The stock DeepFilterNet3 in the beta meets the bundling, license, and latency
items; its CW/FT8 results are the known limitation above.

:::

## Pick a model

```bash
rfwhisper denoise-live                         # deepfilternet3 (voice)
rfwhisper denoise-live --model passthrough     # digital modes / CW
rfwhisper denoise -i in.wav -o out.wav --model deepfilternet3
```

## Model registry

Models are registered in [`src/models/registry.rs`](https://github.com/jakenherman/rfwhisper/blob/master/src/models/registry.rs) as an array of `ModelArtifact` structs:

```rust
pub const ARTIFACTS: [ModelArtifact; 2] = [
    ModelArtifact {
        name: "DeepFilterNet3 ONNX (community export — verify before production)",
        relpath: "models/deepfilternet3/dfn3.onnx",
        url: "https://huggingface.co/aufklarer/DeepFilterNet3-ONNX/resolve/main/deepfilter.onnx",
        sha256: "VERIFY_ON_FIRST_RUN",
        license_note: "Upstream DeepFilterNet: MIT OR Apache-2.0. Third-party export; test parity.",
    },
    // ...
];
```

These entries are for the future ONNX path; the beta's default model is bundled and needs no fetch. `rfwhisper models fetch` downloads, verifies, and caches registry entries under the repo's `models/` directory. The first run captures the artefact's SHA-256 in a `.sha256.local` sidecar — pin that value into `ARTIFACTS` (src/models/registry.rs) once you've validated parity.

### Inside the code

`rfwhisper::denoise::select_engine(name)` returns a `DenoiseEngine`. Every
engine implements both an offline path (`process`, whole buffer, sample-aligned)
and a realtime path (`process_stream`, stateful, any chunk size), which is what
`denoise-live` uses. The ONNX registry (`src/models/registry.rs`) and
execution-provider selection are wired and tested for future ONNX models; the
beta's DeepFilterNet3 doesn't go through them.

## The non-regression gates (every model must pass)

1. **[CW keying transients](../quickstart/v0_1-test-guide.md#status-at-a-glance)** — RMS within ±1 dB of raw on every transient.
2. **[FT8 decodes](../quickstart/v0_1-test-guide.md#status-at-a-glance)** — denoised decodes ≥ raw, zero false decodes.
3. **[No-op sanity](../quickstart/v0_1-test-guide.md#status-at-a-glance)** — clean in, clean out. PESQ drop ≤ 0.3, STOI drop ≤ 0.02.

A model that fails any of these is not merged as a default, even if it scores higher on SNR gain. (The beta's stock model is the exception that proves the rule: it is shipped for voice with an explicit "use passthrough for CW/digital" limitation, pending the gates on real recordings.)

## Next

- [Training](./training.md) — build a dataset + train from scratch
- [Fine-tuning](./fine-tuning.md) — adapt a stock model to your shack in ~30 minutes
- [Model Cards](./model-cards.md) — what's in each model, who maintains it, and how it was evaluated
