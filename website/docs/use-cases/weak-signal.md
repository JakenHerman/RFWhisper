---
id: weak-signal
title: Weak-Signal Work
sidebar_position: 3
description: EME, 630m, microwave, and other environments where every dB counts.
---

# Weak-Signal Work

EME, 630m, 2200m, 6m sporadic E, microwave — the modes where you live 3 dB above the noise floor and every photon matters. RFWhisper isn't a miracle, but a ham-tuned DNN that doesn't damage signal structure is a meaningful addition to a weak-signal operator's toolkit.

## Why weak-signal benefits most

Classical NR is built around the assumption that noise is stationary and the signal is loud. Weak-signal modes break both assumptions:

- The **signal is barely above the noise**. Any gating threshold swallows it.
- The **noise is often non-stationary** — ionospheric propagation flutter, man-made QRM drift, ambient hash.
- **Waveforms are sacred** — Q65, JT9, JT65, WSPR decoders rely on tone phase/amplitude continuity. A denoiser that artifacts is worse than no denoiser.

RFWhisper's hard gates (CW transient, FT8 decode counts) exist specifically because we designed it against the temptation to over-process. On JT-family modes we default to gentler blend factors and longer frames.

## Recommended profiles

| Mode | Profile | Notes |
|---|---|---|
| JT65 (HF / VHF / EME) | `ft8` (close enough for v0.1) | v0.3 adds a dedicated `jt65` profile |
| Q65 (EME / MS / tropo) | `ft8` → `jt65` (v0.3) | Same NN, tuned aggressiveness |
| JT9 | `ft8` | Narrow, same approach |
| WSPR | `ft8` | Extremely narrow; verify decoder via v0.1 regression gate |
| 630m / 2200m CW | `cw` | Crash reduction helps a lot |
| 2m SSB EME (audio post-demod) | `vhf-ssb` (v0.3) | — |

## EME / meteor scatter

- Feed RFWhisper the **rig audio output** after demod (not raw IQ).
- Keep the rig's own NR **off** — RFWhisper replaces it, doesn't augment it.
- WSJT-X sees the virtual cable as a standard input. The non-regression gates guarantee decoder parity.
- Burst modes (MSK144, Q65-F) benefit from **longer pre-filtering** (use the default profile; don't override frame size manually unless you know what you're doing).

## 630m / 2200m (LF / MF)

Noise is different here — seasonal atmospheric QRN dominates, not man-made QRM. A CW-tuned profile with aggressive crash reduction helps a lot without harming keying transients.

```bash
rfwhisper denoise-live --model deepfilternet3 --profile cw \
  --telemetry ./630m-session.jsonl
```

## Microwave / SHF

At microwave frequencies you're typically working narrow modes (CW, SSB, JT4, JT65). Signal chains are long and picky; insert RFWhisper **after** the demodulator, **before** the virtual cable feeding your decoder. Same setup as HF.

## 6m sporadic E

- Band opens fast, closes fast. You don't want to be fiddling with aggressive denoising when a mult rolls by.
- Use the `ssb` or `ft8` profile, A/B toggle on space bar, and focus on the radio.

## Weak-signal contributions we'd love

- **EME recordings** before/after RFWhisper with known-good reference decodes.
- **630m winter** recordings of the seasonal atmospheric QRN — this is hard to synthesize and invaluable for training.
- **WSPR long-term spots** before/after with sequential WSPRnet logs.

See [CONTRIBUTING.md § Submitting RF/Audio Samples](https://github.com/jakenherman/rfwhisper/blob/main/CONTRIBUTING.md#submitting-rfaudio-samples).

## The operator's honest take

A +3 dB denoiser on a weak-signal station is not going to change what's physically copyable in theory. It changes:

- Operator **fatigue** on long sessions.
- **Decoder margin** when the signal is right at the edge.
- **Time-to-decide** whether a signal is real or imagined.

Those are real, measurable gains. <span className="rfw-73">73</span>
