---
id: training
title: Training from Scratch
sidebar_position: 2
description: Build a ham-noise dataset and train DeepFilterNet3 from scratch.
---

# Training from Scratch

:::info Target release

v0.5 ships the `rfwhisper train` CLI end-to-end. The steps below describe the intended recipe; pieces marked **(v0.5)** are not yet stable. Help shape them — see [Contributing](../contributing).

:::

Most contributors should [fine-tune](./fine-tuning) an existing model on their own noise. Training from scratch is only worth it if you're:

- Researching architectural variants
- Targeting a very different mode family (e.g. digital voice, RTTY-only)
- Building a quantized tiny model for Pi Zero 2 W

## Dataset composition

A good ham denoiser dataset has three components:

1. **Clean speech** — studio-quality, license-clean (LibriSpeech + VCTK + CC-BY community ham-speech contributions).
2. **Isolated ham noise** — recorded off-air with a dummy load or during band silence:
   - Powerline / corona buzz
   - Switch-mode + inverter hash
   - PLC / VDSL combs
   - Plasma / LED drivers
   - Atmospheric QRN (curated from seasonal recordings)
   - Birdies / narrow carriers
3. **Room / rig impulse responses** — so the synthesis reflects a typical mic + shack chain.

See [CONTRIBUTING.md § Submitting RF/Audio Samples](https://github.com/jakenherman/rfwhisper/blob/main/CONTRIBUTING.md#submitting-rfaudio-samples) for the metadata schema we use.

## Synthesis

```bash
# (v0.5) mix clean speech with ham noise at controlled SNRs
rfwhisper train build-dataset \
  --clean  data/clean/ \
  --noise  data/noise/ \
  --out    data/train/ \
  --snr-range -10 20 \
  --samples 200000 \
  --rate 48000
```

Per-sample metadata is written alongside so we can stratify evaluation by noise class.

## Architecture

RFWhisper uses the stock DeepFilterNet3 architecture with minor modifications:

- **Input:** 48 kHz mono, 10 ms frames, log-power spectrogram + phase for deep-filter stage
- **Stage 1 (ERB):** coarse magnitude enhancement over ERB-scaled bands
- **Stage 2 (Deep Filter):** fine-grained complex-valued deep filtering for phase-aware refinement
- **Output:** denoised 48 kHz mono audio

Reference: [DeepFilterNet3 paper](https://arxiv.org/abs/2305.08227).

## Train

```bash
# (v0.5)
rfwhisper train fit \
  --arch deepfilternet3 \
  --data data/train/ \
  --val  data/val/ \
  --epochs 200 \
  --batch 32 \
  --lr 5e-4 \
  --amp bf16 \
  --out runs/dfn3-ham-v1/
```

Checkpoints, TensorBoard logs, and model cards are written under `runs/`.

## Evaluate

```bash
rfwhisper train evaluate \
  --ckpt runs/dfn3-ham-v1/best.pt \
  --eval data/eval/
```

This runs:

- PESQ / STOI / SI-SDR on the eval set
- Ham-specific effective SNR gain
- CW transient gate
- FT8 decode regression gate
- RTF + latency on the reference hardware of the runner

**Any gate failure here is a stop condition.**

## Export

```bash
rfwhisper train export \
  --ckpt runs/dfn3-ham-v1/best.pt \
  --out  models/dfn3-ham-v2/ \
  --variants fp32 int8 \
  --opset 17 \
  --simplify
```

Produces:

- `model.fp32.onnx` + sha256
- `model.int8.onnx` + sha256 (QDQ calibration using the eval set)
- `parity.json` — PyTorch vs ORT diff (RMS ≤ 1e-3, max ≤ 1e-2)
- `card.md` — auto-filled model card (you'll still want to hand-edit the "known failure modes" section)

## Publish

Open a PR with:

- The exported `.onnx` and `.json` pinned via Git LFS
- An entry in `rfwhisper/models/registry.yaml`
- The model card under `docs/models/model-cards/`
- Eval numbers in the PR description

See [Model Cards](./model-cards) for the template.
