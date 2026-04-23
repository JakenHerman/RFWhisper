---
id: model-cards
title: Model Cards
sidebar_position: 4
description: Every RFWhisper model ships with a transparent model card.
---

# Model Cards

Every model RFWhisper ships — stock or community-contributed — publishes a **model card** with architecture, training data, evaluation, license, and known failure modes. This page indexes them and gives you the template for submitting your own.

## Stock models

### DeepFilterNet3 (ham-tuned, FP32)

- **ID:** `deepfilternet3@fp32-v1`
- **License:** MIT / Apache-2.0 (inherits from upstream)
- **Architecture:** DeepFilterNet3 (ERB stage + deep filter stage)
- **Training data:** LibriSpeech + VCTK + RFWhisper ham-noise catalog v1 (powerline, inverters, PLC, plasma, atmospheric QRN), mixed at SNR −10 to +20 dB
- **Eval (reference M1):** SNR gain +5.8 dB avg, PESQ +0.8, RTF 0.12, p99 latency 48 ms
- **Non-regression gates:** CW ✅ · FT8 ✅ · No-op sanity ✅
- **Known failure modes:** Strong (>S9+20) narrowband carriers can survive if they sit on a phone-frequency formant. Use adaptive notch (v0.3).
- **Maintainer:** RFWhisper core team

### DeepFilterNet3 (ham-tuned, INT8)

- **ID:** `deepfilternet3@int8-v1`
- Same as FP32 except: INT8 QDQ quantization calibrated on the eval set
- **Eval (reference M1):** SNR gain +5.6 dB avg, PESQ +0.7, RTF 0.07, p99 latency 31 ms
- **Quality delta vs FP32:** −0.2 dB SNR gain, −0.1 PESQ — acceptable
- **Recommended for:** Raspberry Pi 5, low-power laptops

### RNNoise-ham (retuned)

- **ID:** `rnnoise-ham@v1`
- **License:** BSD-3-Clause
- **Architecture:** RNNoise (GRU, 42-dim bark features)
- **Training data:** Same as DFN3 variants
- **Eval (reference M1):** SNR gain +3.1 dB avg, PESQ +0.3, RTF 0.03, p99 latency 12 ms
- **Non-regression gates:** CW ✅ · FT8 ✅ · No-op sanity ✅
- **Known failure modes:** Less effective on highly impulsive noise (atmospheric crashes on 160m). Consider DFN3 if your noise is crash-dominated.
- **Recommended for:** Raspberry Pi 4, Pi Zero 2 W, anywhere CPU is tight

## Community fine-tunes (v1.0 launch)

The model hub launches with v1.0. Pre-launch, community fine-tunes are tracked in [Discussions → Model Hub](https://github.com/jakenherman/rfwhisper/discussions/categories/model-hub).

## Model card template

When you submit a fine-tune, include a card like this (`docs/models/model-cards/<id>.md`):

```markdown
# <Model name>

- **ID:** `<unique-id>@<variant>-<version>`
- **License:** <SPDX identifier>
- **Architecture:** <e.g., DeepFilterNet3 LoRA rank-16>
- **Base model:** <e.g., deepfilternet3@fp32-v1>
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
