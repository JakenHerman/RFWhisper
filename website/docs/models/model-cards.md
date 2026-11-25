---
id: model-cards
title: Model Cards
sidebar_position: 4
description: Every RFWhisper model ships with a transparent model card.
---

# Model Cards

Every model RFWhisper ships — stock or community-contributed — publishes a **model card** with architecture, training data, evaluation, license, and known failure modes. This page indexes them and gives you the template for submitting your own.

## Shipped in v0.1.0-beta.1

### DeepFilterNet3 (stock, upstream)

- **ID:** `deepfilternet3` (the default `--model`)
- **Source:** upstream [DeepFilterNet](https://github.com/Rikorose/DeepFilterNet)
  v0.5.6 (Schröter et al.), its bundled DeepFilterNet3 weights, run by upstream's
  own Rust engine (`deep_filter` crate on the `tract` runtime). Compiled into the
  RFWhisper binary; nothing is downloaded.
- **License:** MIT OR Apache-2.0 (upstream)
- **Architecture:** DeepFilterNet3: ERB-band gain stage plus a deep-filtering
  stage on the low-frequency bins; full-band 48 kHz, 10 ms hop.
- **Training data:** upstream's speech-enhancement training set (see the
  DeepFilterNet papers and repository). **No ham-radio-specific data.**
- **Measured by RFWhisper** (Apple Silicon laptop, CPU only):
  - RTF ≈ 0.01–0.02 (A5 gate: < 0.5 ✅)
  - Processing per 10 ms block: p50 ≈ 0.1–0.5 ms, p99 < 1 ms
  - Algorithmic latency: 30 ms (STFT overlap + 2 frames of look-ahead)
  - On a real noisy speech recording: correlation with the clean reference
    0.895 → 0.911 (stub: no meaningful change) — see
    [PR #120](https://github.com/JakenHerman/RFWhisper/pull/120)
- **Non-regression gates:** CW (A3) ❌ not passed · FT8 (A2) ❌ not passed ·
  No-op sanity (A6) not yet measured · SNR gain on real ham audio (A1) not yet
  measured
- **Known failure modes:**
  - It's a speech model. **It removes CW tones and FT8/FT4/RTTY/PSK carriers**
    as non-speech (−5.5 dB "gain" on a synthetic tone fixture, #117). Use
    `--model passthrough` for those modes.
  - It also partly removes the synthetic formant "SSB" fixture, so synthetic SNR
    numbers understate it on real voices.
  - Not yet heard on real on-air ham recordings at scale; reports welcome.
- **Recommended for:** SSB / AM / FM voice.
- **Maintainer:** upstream model by the DeepFilterNet authors; integration by
  the RFWhisper core team.

### Passthrough

- **ID:** `passthrough` (alias `none`). Output equals input. For CW, digital
  modes, and as an A/B reference. No failure modes, no gains.

## Planned (not yet available)

These are the targets, not results. Their cards get filled in with measured
numbers when they exist.

- **DeepFilterNet3, ham-tuned (FP32 and INT8 QDQ):** fine-tuned on the
  RFWhisper ham-noise catalog (powerline, inverters, PLC, plasma, atmospheric
  QRN) mixed at −10 to +20 dB SNR, with the CW / FT8 / no-op gates as release
  conditions. v0.5 training pipeline.
- **RNNoise-ham (BSD-3-Clause):** GRU-based fallback for Raspberry Pi 4 / Zero 2 W
  ([#11](https://github.com/JakenHerman/RFWhisper/issues/11)).

## Community fine-tunes (v1.0 launch)

The model hub launches with v1.0. Pre-launch, community fine-tunes are tracked in [Discussions → Model Hub](https://github.com/jakenherman/rfwhisper/discussions/categories/model-hub).

## Model card template

When you submit a fine-tune, include a card like this (`docs/models/model-cards/<id>.md`):

```markdown
# <Model name>

- **ID:** `<unique-id>@<variant>-<version>`
- **License:** <SPDX identifier>
- **Architecture:** <e.g., DeepFilterNet3 LoRA rank-16>
- **Base model:** <e.g., deepfilternet3 (stock, upstream v0.5.6)>
- **Training data:**
  - Clean: <sources>
  - Noise: <sources, QTH description, time of day, band>
  - Total samples: <N>
  - SNR range: <-X to +Y dB>
- **Training recipe:**
  - Epochs: <N>
  - Learning rate: <X>
  - Batch size: <X>
  - Hardware: <GPU / CPU>
  - Wall time: <X hours>
- **Evaluation (reference hardware: <hw>):**
  - Effective SNR gain: <+X dB avg, +Y on powerline clips>
  - PESQ: <raw → cleaned>
  - STOI: <raw → cleaned>
  - RTF: <X>
  - Latency p50/p99: <X / Y ms>
- **Non-regression gates:**
  - CW transient: ✅ / ❌ (RMS delta)
  - FT8 decodes: ✅ / ❌ (raw / denoised / false decodes)
  - No-op sanity: ✅ / ❌ (PESQ / STOI drop)
- **PyTorch ↔ ONNX parity:** RMS diff <X>, max diff <Y>
- **Known failure modes:** <be honest>
- **Maintainer:** <callsign or GitHub handle, contact preference>
- **Version history:**
  - v1 — <date> — initial release
```

Honest failure modes are more valuable than ambitious claims. Every model breaks on something; telling us where it breaks helps the next operator pick the right tool.
