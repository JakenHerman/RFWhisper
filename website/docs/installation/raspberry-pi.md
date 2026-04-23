---
id: raspberry-pi
title: Raspberry Pi
sidebar_position: 5
description: Install RFWhisper on a Raspberry Pi 5 for field ops (POTA/SOTA/EmComm). RNNoise fallback on Pi 4 and Zero 2 W.
---

# Install on Raspberry Pi

Primary target: **Raspberry Pi 5 (4 GB+)** running Raspberry Pi OS (64-bit) Bookworm. Secondary: Pi 4 with RNNoise only.

## Hardware notes

| Model | Recommended use | Active cooling | Model |
|---|---|---|---|
| **Pi 5 (8 GB)** | Full real-time DFN3 + SDR pipeline | Required | DeepFilterNet3 (FP32) |
| **Pi 5 (4 GB)** | Full real-time DFN3 + SDR pipeline | Required | DeepFilterNet3 (INT8) |
| **Pi 4 (8 GB)** | Audio-only denoise, RNNoise fallback | Recommended | RNNoise-ham |
| **Pi 400** | Same as Pi 4 | Keyboard heatsink helps | RNNoise-ham |
| **Pi Zero 2 W** | Ultra-portable monitor only | Airflow matters | RNNoise-ham (tiny) |

:::tip POTA / SOTA build

A Pi 5 + RTL-SDR v4 + a 10 Ah USB-C PD bank runs RFWhisper + WSJT-X for hours in the field. Add an Argon ONE M.2 case for the NVMe slot and cooling.

:::

## 1. Flash Raspberry Pi OS (64-bit)

Use [Raspberry Pi Imager](https://www.raspberrypi.com/software/) and pick **Raspberry Pi OS (64-bit) Bookworm**. Enable SSH and set a username during the pre-flash customization.

## 2. System update + deps

```bash
sudo apt update && sudo apt full-upgrade -y
sudo apt install -y \
  python3 python3-venv python3-pip \
  git git-lfs \
  libportaudio2 libsndfile1 libasound2-dev \
  ffmpeg build-essential cmake pkg-config
```

## 3. (Optional) GNU Radio

```bash
sudo apt install -y gnuradio gr-soapy soapysdr-module-all soapysdr-tools
```

gr-dnn is not yet packaged for Raspberry Pi OS; we ship an aarch64 wheel in v0.2.

## 4. Install RFWhisper

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
python3 -m venv .venv && source .venv/bin/activate
pip install -U pip wheel

# On Pi 5 prefer DFN3; on Pi 4 / Zero 2 W stick to RNNoise:
pip install -e ".[audio]"
python -m rfwhisper.models.fetch --variant auto    # picks RNNoise on <Pi 5
rfwhisper doctor
```

## 5. Tuning for low latency

### Governor + irqbalance

```bash
# Keep CPU at max while RFWhisper is running
sudo apt install -y cpufrequtils
echo 'GOVERNOR="performance"' | sudo tee /etc/default/cpufrequtils
sudo systemctl restart cpufrequtils

# Spread IRQs off the audio core
sudo apt install -y irqbalance
sudo systemctl enable --now irqbalance
```

### Realtime scheduling

Same as [Linux § 5](./linux#5-realtime-scheduling-recommended) — add yourself to a `realtime` group and set rtprio/memlock limits.

### Audio

Use ALSA directly (not PulseAudio) for lowest latency:

```bash
rfwhisper denoise-live --in hw:0,0 --out hw:1,0 --blocksize 240 --profile ssb
```

## 6. RTL-SDR / SoapySDR modules on aarch64

```bash
sudo apt install -y rtl-sdr soapysdr-module-rtlsdr
# Test:
SoapySDRUtil --find
```

If you see your dongle listed, RFWhisper's flowgraphs will too.

## 7. Expected performance (Pi 5 8 GB, DFN3 INT8)

| Pipeline | CPU | Latency p99 |
|---|---|---|
| Audio-only denoise | ~35 % of 1 core | 70–90 ms |
| RTL-SDR → SSB demod → denoise | ~55 % of 2 cores | 150–220 ms |

(See [Architecture → Latency Budget](../architecture/latency-budget) for the formal budget.)

## Troubleshooting

- **Throttling** (`vcgencmd get_throttled` returns non-zero) — add a proper heatsink/fan and check your power supply delivers ≥ 27 W for the Pi 5.
- **Underruns under load** — lower blocksize limits; try `--blocksize 480` for audio-only on Pi 4.
- **Missing `CoreML` or `CUDA` providers** — expected. RPi runs on the CPU provider + XNNPACK.
- **`ALSA lib pcm.c` warnings at startup** — generally benign; verify audio works end-to-end before investigating.
