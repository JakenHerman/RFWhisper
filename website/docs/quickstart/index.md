---
id: index
title: Quick Start
sidebar_position: 1
description: Clone, generate a noisy signal, denoise it, and open a before/after report — no downloads.
slug: /quickstart/
---

# Quick Start

From `git clone` to a before/after report in a few minutes. Everything here is
reproducible from a seed — no sample pack, no model download.

:::info Alpha status — read this first
RFWhisper is early alpha. The default `deepfilternet3` model **falls back to a
classical spectral-subtraction stub** (~+1 dB) unless you build with the `dfn`
feature (below). The stub is for wiring and CI; the real neural denoiser is opt-in
while it stabilises. See the [project status](https://github.com/JakenHerman/RFWhisper#-early-alpha--the-neural-denoiser-is-not-built-yet).
:::

## 1. Build

```bash
git clone https://github.com/JakenHerman/RFWhisper.git
cd RFWhisper
cargo build --release          # single binary at target/release/rfwhisper
```

## 2. Make a noisy signal

No downloads: the fixtures are generated from a seed, so these commands produce
byte-identical audio on any machine.

```bash
rfwhisper samples list          # signals, noise types, and named presets

# An S3 SSB signal buried under an S7 powerline buzz (-24 dB SNR).
# --out is the noisy mix; --clean-out is the reference it was mixed from.
rfwhisper samples synth --kind mix --preset ssb_powerline_s3_s7 \
  --out ssb.wav --clean-out ssb.clean.wav
```

## 3. Denoise it, with a before/after report

```bash
rfwhisper denoise \
  --input       ssb.wav \
  --output      cleaned.wav \
  --reference   ssb.clean.wav \
  --model       spectral_stub \
  --report      report.json \
  --spectrogram report.html
```

`report.json` is the machine-readable summary — these are the real fields:

```json
{
  "model": "spectral_stub",
  "input": "ssb.wav",
  "output": "cleaned.wav",
  "sr": 48000,
  "duration_s": 5.0,
  "inference_time_ms": 7.03,
  "rtf": 0.0014,
  "snr_gain_db": 0.41,
  "spectrogram_path": "report.html"
}
```

`report.html` is a **self-contained** before/after page — SNR tiles, side-by-side
spectrograms on a shared scale, and a median-spectrum chart. Open it in any
browser, offline. Listen to `ssb.clean.wav`, `ssb.wav`, and `cleaned.wav` back to
back too.

:::note Why the gain is small here
`spectral_stub` is a placeholder that measures around +0.4 dB on this preset. That
is expected — it exists to exercise the pipeline, not to clean up your audio.
Switch on the real model next.
:::

## 4. Switch on the real DeepFilterNet3 model

The neural backend is opt-in because it pulls a larger dependency tree:

```bash
cargo build --release --features dfn      # ~4 min the first time (pulls tract)

rfwhisper denoise -i ssb.wav -o cleaned_dfn.wav \
  --model deepfilternet3 --spectrogram report_dfn.html
```

The DeepFilterNet3 model is bundled in the binary — no download. It runs at about
**RTF 0.017 (≈ 60× faster than real time)** on a modern laptop CPU.

:::caution DeepFilterNet3 is a speech model
It denoises real speech well, but it correctly removes the **synthetic** tone-stack
signal above as non-speech — so on a `samples synth` clip its measured "gain" is
negative. To see it shine, feed it a real voice recording (see
[samples/](https://github.com/JakenHerman/RFWhisper/tree/master/samples)). This is
why the A1 SNR-gain gate needs real audio, not synthetic fixtures.
:::

## 5. Real-time from rig → virtual cable (implemented, not yet gate-verified)

```bash
rfwhisper audio list            # find your device indices

# Pipe input device 3 → denoiser → output device 5 (a virtual cable)
rfwhisper denoise-live --in 3 --out 5 --model spectral_stub --blocksize 480
```

Point WSJT-X / fldigi / JS8Call at the virtual cable as their input. See the
[virtual-cable setup](../installation/) for VB-Cable / BlackHole / loopback.

## What exists today

| Command | Status |
|---|---|
| `samples synth` / `samples list` | ✅ works |
| `denoise` (offline WAV → WAV) | ✅ works |
| `denoise --spectrogram` (before/after HTML) | ✅ works |
| `--model deepfilternet3` (with `--features dfn`) | ✅ works |
| `denoise-live` (realtime) | ⚙️ implemented, not yet gate-verified |
| `gui` | 🚧 prints a roadmap pointer; native GUI is v0.4 |

## Next: prove it properly

Want to *measure* it rather than eyeball it? See the
[v0.1 Test Guide](./v0_1-test-guide) for the acceptance gates and how to run them.
