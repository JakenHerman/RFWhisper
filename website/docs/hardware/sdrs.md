---
id: sdrs
title: Supported SDRs
sidebar_position: 2
description: Per-device compatibility matrix with on-air notes.
---

# Supported SDRs

Status legend:

- ✅ **Tested on-air** — at least one community contributor has a recorded QSO or reception report.
- 🧪 **Tested flowgraph** — flowgraph runs end-to-end, no on-air QSO yet submitted.
- 📝 **Planned** — on the v0.2 / v1.0 roadmap but not yet validated.
- ❓ **Needs help** — the SDR should work via SoapySDR; nobody's confirmed yet.

| SDR | Driver | HF | VHF | UHF | Status |
|---|---|---|---|---|---|
| RTL-SDR v3 / v4 | `rtlsdr` (SoapyRTLSDR) | via upconverter / direct-sampling-mod | ✅ | ✅ | ✅ (v0.2) |
| Airspy R2 | `airspy` | via upconverter | ✅ | ✅ | 🧪 |
| Airspy Mini | `airspy` | via upconverter | ✅ | ✅ | 🧪 |
| Airspy HF+ Discovery | `airspyhf` | ✅ | ✅ (30 MHz–260 MHz) | — | ✅ |
| HackRF One | `hackrf` | ✅ | ✅ | ✅ | 🧪 |
| ADALM-Pluto | `plutosdr` | ≥ 325 MHz stock / mod below | ✅ | ✅ | 🧪 |
| SDRplay RSP1A | `sdrplay` (SoapySDRPlay3) | ✅ | ✅ | ✅ | 🧪 |
| SDRplay RSPdx | `sdrplay` | ✅ | ✅ | — | 📝 |
| LimeSDR Mini 2.0 | `lime` | ≥ 10 MHz | ✅ | ✅ | ❓ |
| USRP B200mini | `uhd` | ✅ | ✅ | ✅ | ❓ |
| KiwiSDR (remote) | `kiwisdr` | ✅ | — | — | 📝 |
| RSPduo | `sdrplay` | ✅ | ✅ | ✅ | 📝 |
| Ettus B210 | `uhd` | ✅ (with ext. filters) | ✅ | ✅ | 📝 |

## Per-device notes

### RTL-SDR v4

- Most affordable entry point. Default flowgraph `flowgraphs/rtl_ssb_hf.grc`.
- For HF use: enable direct-sampling mode or add an upconverter (Ham It Up, SpyVerter, etc.).
- Common setup: RTL-SDR v4 + Ham It Up Plus + LNA → RFWhisper @ 14 MHz USB.

### Airspy HF+ Discovery

- Better HF dynamic range than RTL-SDR; highly recommended if you work DX or contests.
- Lower native sample rate (768 ksps) means less CPU for decimation.

### ADALM-Pluto

- Stock firmware tunes 325 MHz–3.8 GHz; [community mods](https://wiki.analog.com/university/tools/pluto/hacking/frequency) enable full HF. Follow local regulations.
- Very popular for VHF/UHF weak-signal experimentation.

### SDRplay RSP series

- Use SoapySDRPlay3 (not the legacy SDRPlay module).
- Enable broadcast notches in the API preset; helps the NN focus on actual noise rather than fighting an overloaded front-end.

### HackRF One

- Half-duplex. Wideband (up to 20 Msps) but 8-bit ADC — narrow applications like contest SSB work better than wideband monitoring.

## Reporting a new SDR

1. Run the nearest matching flowgraph.
2. Open an Issue with the `hardware-report` label.
3. Include: make/model/firmware, sample rates tested, a 30 s before/after recording, and `rfwhisper doctor` output.

We'll update the matrix and credit you.

## SDRs that almost certainly work

If it has a SoapySDR driver listed at <https://github.com/pothosware/SoapySDR/wiki>, RFWhisper can use it. The only real question is whether we've tuned a flowgraph for it. Community flowgraph contributions are extremely welcome — see [Architecture → Flowgraphs](../architecture/flowgraphs).
