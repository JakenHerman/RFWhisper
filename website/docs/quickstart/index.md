---
id: index
title: Quick Start
sidebar_position: 1
description: Get from zero to clean ham audio in five minutes.
slug: /quickstart/
---

# Quick Start

Five minutes from `git clone` to clean audio. Assumes you've finished [Installation](../installation/).

## 1. Denoise a file (offline A/B)

```bash
rfwhisper denoise \
  --input  samples/noisy_40m_ssb.wav \
  --output cleaned.wav \
  --model  deepfilternet3 \
  --profile ssb \
  --report report.json
```

`report.json` contains:

```json
{
  "model": "deepfilternet3@sha256:…",
  "profile": "ssb",
  "effective_snr_gain_db": 5.8,
  "pesq": { "raw": 2.1, "cleaned": 2.9 },
  "stoi": { "raw": 0.72, "cleaned": 0.84 },
  "latency": { "p50_ms": 34, "p99_ms": 71 },
  "rtf": 0.38,
  "spectrograms": "build/audio-reports/…png",
  "gates": { "cw_transient": "N/A", "ft8_regression": "N/A" }
}
```

Open `cleaned.wav` in Audacity or your player of choice and compare.

## 2. Real-time from rig → virtual cable → WSJT-X

```bash
# 1. List audio devices
rfwhisper audio list

# 2. Pipe rig audio → denoise → virtual cable in real time
rfwhisper denoise-live \
  --in  "USB Audio CODEC" \
  --out "BlackHole 2ch"        `# macOS — VB-Cable on Win, loopback on Linux` \
  --model deepfilternet3 \
  --profile ft8
```

3. In WSJT-X: **File → Settings → Audio → Input** = your virtual cable.
4. Hit Monitor. You should see decodes as normal — ideally with a few extra marginals.

:::tip A/B the difference live

Press **space** in the `denoise-live` terminal to toggle bypass. You'll hear the raw audio instantly, which is the fastest way to convince yourself (or a skeptical Elmer) that the denoiser is helping.

:::

## 3. Launch the GUI

```bash
rfwhisper gui
```

- Pick input + output devices
- Toggle A/B bypass with a big obvious button
- Record both raw and cleaned audio simultaneously
- Live latency / CPU / RTF display

## What to try first

| If you operate… | Try |
|---|---|
| **FT8 / FT4** | Replay a 15-min recording through `denoise-live` to WSJT-X; compare decode counts |
| **SSB (HF)** | Find a weak DX station, toggle A/B every 10 s |
| **CW** | Feed a 25 WPM recording under QRN — listen to the crashes knock down while dits stay crisp |
| **VHF FM** | Scratchy simplex on 2m — this is where FM listeners feel it first |

## Next: prove it properly

The **five-minute demo** is fun. But if you want to *know* whether RFWhisper is helping in your environment, run the formal acceptance suite → [v0.1 Test Guide](./v0_1-test-guide).
