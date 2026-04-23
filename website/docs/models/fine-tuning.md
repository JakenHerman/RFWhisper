---
id: fine-tuning
title: Fine-Tuning for Your QTH
sidebar_position: 3
description: Record your own noise, fine-tune DeepFilterNet3, and deploy — a 30-minute recipe.
---

# Fine-Tuning for Your QTH

> *"My 40m noise floor isn't yours. The stock model is good. A model that's seen my powerline buzz is better."*

Target outcome: **+2 dB extra effective SNR gain** vs the stock model on *your* noise environment, without regressing on anything else.

:::info v0.5 target

The `rfwhisper train fine-tune` workflow ships in v0.5. If you're reading this pre-v0.5, the scripts live on a feature branch — see the [Fine-tuning milestone](../roadmap#v05--fine-tuning-tools--dataset-generator).

:::

## Step 1 — Record isolated noise (5–10 min)

Point your rig at a clear part of the band where your normal noise is present but no signals are (between 40m calling frequencies, for example), and record to the dummy-load or your rig's audio output:

```bash
rfwhisper train record-noise \
  --input "USB Audio CODEC" \
  --rate 48000 \
  --duration 300 \
  --label powerline-evening \
  --out data/my-noise/
```

Tip: record **different times of day** and **multiple bands**. Inverters change with the sun; PLC noise changes with household activity.

## Step 2 — Build a fine-tuning dataset (2 min)

```bash
rfwhisper train build-dataset \
  --clean  pretrained-clean/        # ships with rfwhisper
  --noise  data/my-noise/
  --out    data/my-train/
  --snr-range -10 20
  --samples 20000
```

## Step 3 — Fine-tune (20–90 min depending on hardware)

### On a GPU

```bash
rfwhisper train fine-tune \
  --base   deepfilternet3 \
  --data   data/my-train/ \
  --out    runs/my-qth-v1/ \
  --epochs 10 \
  --lr 1e-4 \
  --method lora       # or 'full' for deeper adaptation
```

Wall time on an RTX 3060: ~45 min for a LoRA fine-tune on 20 000 mixes.

### On CPU (laptop / RPi 5)

```bash
rfwhisper train fine-tune --base deepfilternet3 --data data/my-train/ \
  --out runs/my-qth-v1/ --epochs 3 --lr 1e-4 --method lora \
  --device cpu --batch 4
```

Wall time on an i5-12xxx: ~2 hours. RPi 5: ~12 hours for a tiny model.

## Step 4 — Evaluate (5 min)

```bash
rfwhisper train evaluate --ckpt runs/my-qth-v1/best.pt --eval data/my-eval/
```

What must pass:

- **≥ +2 dB** additional SNR gain on your noise vs the stock DFN3
- **No regressions** on a held-out general speech+noise set
- **CW transient gate** green
- **FT8 regression gate** green

If any gate fails, iterate:

- Increase the dataset size
- Reduce learning rate
- Try a smaller LoRA rank
- Add more diverse noise classes (record at a different time of day)

## Step 5 — Export and deploy

```bash
rfwhisper train export --ckpt runs/my-qth-v1/best.pt \
  --out ~/.cache/rfwhisper/models/my-qth-v1/ \
  --variants fp32 int8
```

Use it:

```bash
rfwhisper denoise-live --model my-qth-v1 --profile ssb
```

## Step 6 — (Optional) Share it

If your fine-tune beats the stock model on your QTH and passes all gates, please consider contributing it to the community hub:

1. Upload the `.onnx` to a public host (or open a PR with it via Git LFS).
2. Fill out the [model card template](./model-cards).
3. Open a PR in the `rfwhisper-models` repo (or the main repo pre-hub-launch).

We never require you to share. But a fine-tune trained on rural-Wisconsin-solar-inverter-on-40m-evening noise is priceless for the next ham in a similar environment.

## LoRA vs full fine-tune

| Method | When | Wall time | Quality ceiling |
|---|---|---|---|
| **LoRA** (rank 8–16) | Default. Your noise is *a variant* of typical QRM. | Fast | +1–3 dB over stock |
| **Full** | Your noise is dramatically unusual (custom RF environment, specific interferer). | 5–10× slower | Higher, but risks overfitting / regressions |

Start with LoRA. Escalate only if you need to.

## Troubleshooting

- **Regression gates fail mid-training** — lower the learning rate; you're overfitting to the new noise at the cost of signal preservation.
- **Tiny SNR gain improvement** — your stock model was already great at your noise. That's a win.
- **Training diverges** — check the noise recordings for clipping or DC offset; `rfwhisper train record-noise` reports both but it's easy to miss.
