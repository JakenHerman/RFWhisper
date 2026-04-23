---
id: rigs
title: Rig USB CODEC Routing
sidebar_position: 3
description: Using RFWhisper with rig USB audio CODECs (IC-7300, IC-705, FTDX10, K3, FT-991, Anan).
---

# Rig USB CODEC Routing

Most modern HF rigs expose their TX/RX audio as a USB audio CODEC. RFWhisper treats that CODEC as any other audio input — no SDR required. This is the simplest path to a v0.1 on-air demo.

## Quick setup

```bash
# 1. Identify your rig's CODEC name in the device list
rfwhisper audio list
# → "USB Audio CODEC"        (Icom, most)
# → "SignaLink USB"           (SignaLink interface)
# → "PR-40 USB Mic"           (not a rig; skip)

# 2. Run denoise-live from rig CODEC → virtual cable
rfwhisper denoise-live --in "USB Audio CODEC" --out "BlackHole 2ch" --profile ssb

# 3. Point WSJT-X at the virtual cable
```

## Per-rig notes

### Icom IC-7300

- Set **MENU → SET → Connectors → USB SEND** to your preferred keying method.
- Set the CODEC **Sample Rate** to 48 kHz in the rig menu.
- Use **USB Audio CODEC** as RFWhisper's input.

### Icom IC-705

- Identical to IC-7300 for audio routing.
- Bluetooth/WiFi streaming is not recommended for low-latency use.

### Yaesu FTDX10 / FT-991 / FT-991A

- In the rig menu set **USB MOD** and **AF OUT SEL** per your preference; this project only needs the AF OUT to be routed to USB audio.
- Baud rate for CAT (separate from audio) doesn't matter for RFWhisper unless you're using Hamlib for mode-follow (v0.3).

### Elecraft K3 / K3S / KX3

- Line-out into a USB sound card is the typical chain if you're not using the K3S's internal USB.
- For the KX3 use a USB audio adapter between the headphone jack and your computer.

### Apache Labs ANAN (SDR rig)

- Use the ANAN's built-in [Thetis / PowerSDR mRX] audio output piped into a virtual cable, then RFWhisper reads from that.

### Flex 6000 series

- Flex has its own DAX virtual audio routing; point RFWhisper at a DAX RX slice output, then route RFWhisper's output to a second virtual cable that WSJT-X listens on.

### SignaLink USB / rigExpert / other interfaces

- These are just USB audio CODECs from the computer's perspective. Same setup as the rigs above.

## Rig control (Hamlib)

v0.3 adds a Hamlib hook so RFWhisper can auto-select a profile based on your rig's current mode (SSB / CW / DATA-U / etc.). This is optional — manual `--profile` always wins.

## Common gotchas

- **Two devices share the same name** — run `rfwhisper audio list --json` and pass the numeric index instead of the name.
- **Double denoising** — if you already have a classical NR engaged on the rig, RFWhisper may fight it. Turn rig NR off and let RFWhisper do the work.
- **Level mismatch** — RFWhisper's soft limiter catches overshoot but you should still set a sensible rig AF level (S-meter mid-scale on noise, not clipping on strong signals).
