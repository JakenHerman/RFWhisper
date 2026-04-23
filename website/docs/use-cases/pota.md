---
id: pota
title: POTA / SOTA Field Ops
sidebar_position: 1
description: Using RFWhisper from a parking lot, picnic table, or mountaintop — low-power, low-noise, field-ready.
---

# POTA / SOTA Field Ops

Parks On The Air and Summits On The Air are where RFWhisper earns its keep. You're usually operating QRP (5 W) into a compromise antenna, chasing weak stations, and you've dragged a laptop up the hill so every dB of improvement matters.

## Typical field setup

| Component | Example | Notes |
|---|---|---|
| Rig | IC-705 / KX3 / FT-891 / X6100 | USB CODEC handles audio routing |
| Antenna | EFHW / vertical / loop | Compromise is the norm |
| Computer | Laptop or **Raspberry Pi 5** | RPi 5 with a small 7" screen is ideal |
| Power | 10–20 Ah LiFePO4 + USB-PD bank | RPi 5 draws ~5 W running RFWhisper + WSJT-X |
| Virtual cable | PipeWire / BlackHole / VB-Cable | Per your OS |
| Model | `deepfilternet3@int8` or `rnnoise-ham` | INT8 saves ~30% CPU |
| Profile | `ssb` or `ft8` | Auto-select with Hamlib in v0.3 |

## Pi 5 field recipe

```bash
# POTA kit: RPi 5 + IC-705 USB + 7" screen + LiFePO4
rfwhisper denoise-live \
  --in  "USB Audio CODEC" \
  --out "loopback" \
  --model deepfilternet3 \
  --model-variant int8 \
  --profile auto       # reads rig mode via Hamlib (v0.3)
```

In WSJT-X: **Audio → Input** = `loopback`.

## What to actually expect

- **FT8 on 20m at midday** — typically +1 to +3 marginal decodes per cycle, zero false decodes. Great day.
- **40m SSB evening contact** — stations near your local S-meter noise floor become copyable. You'll notice the difference on a 599-vs-449 signal report.
- **CW in the wild** — RFWhisper is conservative on CW by default (profile `cw`). Crashes from distant storms come down; dits and dahs stay sharp.
- **VHF FM simplex** — scratchy signals ride up. Deemphasis runs before the NN so you don't amplify hiss into the NN's features.

## Power budget (Pi 5 + RFWhisper + WSJT-X)

| Load | Power |
|---|---|
| Pi 5 idle | ~2.5 W |
| Pi 5 + WSJT-X | ~3.5 W |
| Pi 5 + WSJT-X + RFWhisper (INT8) | ~5.0 W |
| + 7" HDMI touchscreen | +2–3 W |

A 20 Ah 12 V LiFePO4 bank + a 12 V → USB-PD step-down runs the Pi for most of a typical POTA session even after powering the rig.

## Field-tested kit list

*This section is populated by community contributions. Submit your kit list via [Discussions → Field Ops](https://github.com/jakenherman/rfwhisper/discussions/categories/field-ops).*

- **[community slot]** — POTA activator, rig, antenna, computer, result
- **[community slot]** — SOTA activator, rig, antenna, computer, result

## Tips specific to field ops

1. **Run on battery**, not grid, if at all possible — portable QTHs are much quieter than home, and you want the NN learning what's real noise vs fake.
2. **Set blocksize to 960** on slower SBCs to give yourself margin on uneven CPU frequency scaling outdoors.
3. **Record before/after clips in the field** (`rfwhisper gui` has a record button) — they're the best content for the community model hub.
4. **Update** `rfwhisper models fetch` before you leave — no internet in the park.

## Heat

Raspberry Pi 5 will thermal-throttle in direct sun without airflow. Active cooling (Argon ONE, Pimoroni fan SHIM, or just a 5 V fan glued to a heatsink) is worth the weight.

<span className="rfw-73">73</span> and enjoy the quiet bands.
