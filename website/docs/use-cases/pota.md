---
id: pota
title: POTA / SOTA Field Ops
sidebar_position: 1
description: Using RFWhisper from a parking lot, picnic table, or mountaintop — low-power, low-noise, field-ready.
---

# POTA / SOTA Field Ops

:::caution Beta: voice only for now
The beta runs the stock DeepFilterNet3 *speech* model. It is good on SSB, AM, and
FM voice, but it removes CW tones and FT8/FT4/RTTY/PSK carriers along with the
noise. Use `--model passthrough` for those modes until the v0.3 mode profiles
ship. Commands marked *(v0.3)* or *(planned)* below don't exist yet.
:::

Parks On The Air and Summits On The Air are where RFWhisper earns its keep. You're usually operating QRP (5 W) into a compromise antenna, chasing weak stations, and you've dragged a laptop up the hill so every dB of improvement matters.

## Typical field setup

| Component | Example | Notes |
|---|---|---|
| Rig | IC-705 / KX3 / FT-891 / X6100 | USB CODEC handles audio routing |
| Antenna | EFHW / vertical / loop | Compromise is the norm |
| Computer | Laptop or **Raspberry Pi 5** | RPi 5 with a small 7" screen is ideal |
| Power | 10–20 Ah LiFePO4 + USB-PD bank | RPi 5 draws ~5 W running RFWhisper + WSJT-X |
| Virtual cable | PipeWire / BlackHole / VB-Cable | Per your OS |
| Model | `deepfilternet3` (voice), `passthrough` (digital) | INT8 and RNNoise variants are planned |
| Profile | *(v0.3)* `ssb` / `ft8` | Auto-select with Hamlib in v0.3 |

## Pi 5 field recipe

```bash
# POTA kit: RPi 5 + IC-705 USB + 7" screen + LiFePO4
rfwhisper denoise-live \
  --in  "USB Audio CODEC" \
  --out "Loopback" \
  --record-raw raw.wav --record-denoised denoised.wav
# For FT8 / FT4 in the beta, add: --model passthrough
```

In WSJT-X: **Audio → Input** = the matching loopback capture device (see
[Virtual audio cables](../hardware/virtual-cables)). Hamlib-driven profile
auto-selection is planned for v0.3.

## What to actually expect

These are the v0.1-final goals; the beta is only partly there.

- **40m SSB evening contact**: the beta's sweet spot. Hiss, buzz, and hash under
  a voice come down hard; weak voices near the noise floor get easier to copy.
  Tell us if it does: on-air reports are what validate this.
- **FT8 on 20m at midday**: the goal is +1 to +3 marginal decodes per cycle with
  zero false decodes (gate A2). The beta's speech model removes FT8 tones, so
  use `--model passthrough` until the FT8 profile lands.
- **CW in the wild**: the goal is crashes down, keying untouched (gate A3). Same
  advice for the beta: `--model passthrough`.
- **VHF FM simplex**: voice, so the beta helps. De-emphasis-aware handling is
  planned.

## Power budget (Pi 5 + RFWhisper + WSJT-X)

| Load | Power |
|---|---|
| Pi 5 idle | ~2.5 W |
| Pi 5 + WSJT-X | ~3.5 W |
| Pi 5 + WSJT-X + RFWhisper | not yet measured |
| + 7" HDMI touchscreen | +2–3 W |

A 20 Ah 12 V LiFePO4 bank + a 12 V → USB-PD step-down runs the Pi for most of a typical POTA session even after powering the rig.

## Field-tested kit list

*This section is populated by community contributions. Submit your kit list via [Discussions → Field Ops](https://github.com/jakenherman/rfwhisper/discussions/categories/field-ops).*

- **[community slot]** — POTA activator, rig, antenna, computer, result
- **[community slot]** — SOTA activator, rig, antenna, computer, result

## Tips specific to field ops

1. **Run on battery**, not grid, if at all possible — portable QTHs are much quieter than home, and you want the NN learning what's real noise vs fake.
2. **Set blocksize to 960** on slower SBCs to give yourself margin on uneven CPU frequency scaling outdoors.
3. **Record before/after clips in the field** (`--record-raw raw.wav --record-denoised denoised.wav`). They're the best content for the community model hub.
4. **No internet needed in the park.** The model is built into the binary; just make sure you've built (or copied) the current release before you leave.

## Heat

Raspberry Pi 5 will thermal-throttle in direct sun without airflow. Active cooling (Argon ONE, Pimoroni fan SHIM, or just a 5 V fan glued to a heatsink) is worth the weight.

<span className="rfw-73">73</span> and enjoy the quiet bands.
