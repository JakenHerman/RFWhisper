---
id: index
title: Hardware Compatibility
sidebar_position: 1
description: Supported SDRs, rigs, and virtual audio cables — with on-air notes from real hams.
slug: /hardware/
---

# Hardware Compatibility

RFWhisper aims for **"works with anything SoapySDR supports"** plus common rig USB audio CODECs. Real-world status is tracked in the matrix below and updated from community reports.

| Category | Target v0.1 | Target v0.2 | Target v1.0 |
|---|---|---|---|
| Rig USB audio (any CODEC) | ✅ | ✅ | ✅ |
| Laptop / desktop SDRs | N/A | 3 tested | 8+ tested |
| SBCs (RPi, Jetson) | RPi 5 ✅ | RPi 5 ✅ | RPi 5 + RPi 4 (RNNoise) |

## Sections

- [SDRs](./sdrs) — RTL-SDR, Airspy, HackRF, Pluto, SDRplay, LimeSDR, USRP, KiwiSDR
- [Rigs](./rigs) — IC-7300, IC-705, FTDX10, K3, FT-991, Anan — rig USB CODEC routing
- [Virtual audio cables](./virtual-cables) — VB-Cable / BlackHole / JACK / PipeWire / snd-aloop

## How we keep this honest

- Every SDR in the matrix has a dedicated entry with the exact SoapySDR driver, tested modes, known quirks, and a link to a community on-air recording where possible.
- The matrix is **auto-checked monthly** against GitHub Issues tagged `hardware-report` — if a report contradicts a matrix entry, we flag it.
- Contributing a hardware test is a first-class contribution. See [CONTRIBUTING.md § Hardware Testing](https://github.com/jakenherman/rfwhisper/blob/main/CONTRIBUTING.md#hardware-testing).
