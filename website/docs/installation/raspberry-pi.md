---
id: raspberry-pi
title: Raspberry Pi
sidebar_position: 5
description: Install RFWhisper on a Raspberry Pi 5 for field ops (POTA/SOTA/EmComm). Pi 5 is the target; not yet verified in the beta.
---

# Install on Raspberry Pi

:::caution Not yet verified in the beta
The Pi 5 is a target platform, but nobody has run the beta on one yet. The
build should work (it's ordinary aarch64 Linux); performance is unmeasured. If
you try it, please post the `denoise-live` status line in an issue.
:::

Primary target: **Raspberry Pi 5 (4 GB+)** running Raspberry Pi OS (64-bit) Bookworm. The Pi 4 / Zero 2 W are meant for the RNNoise fallback, which is not built yet ([#11](https://github.com/JakenHerman/RFWhisper/issues/11)).

## Hardware notes

| Model | Intended use | Active cooling | Denoiser |
|---|---|---|---|
| **Pi 5 (4–8 GB)** | Real-time DFN3 (audio; SDR pipeline from v0.2) | Required | DeepFilterNet3 |
| **Pi 4 (8 GB)** | Audio-only denoise | Recommended | RNNoise *(planned, #11)* |
| **Pi 400** | Same as Pi 4 | Keyboard heatsink helps | RNNoise *(planned)* |
| **Pi Zero 2 W** | Ultra-portable monitor only | Airflow matters | RNNoise *(planned)* |

:::tip POTA / SOTA build

A Pi 5 + RTL-SDR v4 + a 10 Ah USB-C PD bank runs RFWhisper + WSJT-X for hours in the field. Add an Argon ONE M.2 case for the NVMe slot and cooling.

:::

## 1. Flash Raspberry Pi OS (64-bit)

Use [Raspberry Pi Imager](https://www.raspberrypi.com/software/) and pick **Raspberry Pi OS (64-bit) Bookworm**. Enable SSH and set a username during the pre-flash customization.

## 2. System update + deps

```bash
sudo apt update && sudo apt full-upgrade -y
sudo apt install -y \
  git libasound2-dev build-essential pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # Rust toolchain
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
cargo build --release        # native aarch64 build; the first compile takes a while
sudo install -m755 target/release/rfwhisper /usr/local/bin/
rfwhisper --version
rfwhisper audio list
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

### Audio

Pick devices by number or name from `rfwhisper audio list`. On a headless Pi
without PipeWire, ALSA device names look like `hw:CARD=CODEC,DEV=0`:

```bash
rfwhisper denoise-live --in "CARD=CODEC" --out "CARD=Loopback,DEV=0"
```

To run it unattended (no keyboard), start it from a service or `tmux`; stdin
at end-of-file just disables the `b` / `q` keys, and SIGTERM stops it cleanly.

## 6. RTL-SDR / SoapySDR modules on aarch64

```bash
sudo apt install -y rtl-sdr soapysdr-module-rtlsdr
# Test:
SoapySDRUtil --find
```

If you see your dongle listed, RFWhisper's flowgraphs will too.

## 7. Expected performance

Not measured yet. The A5 target is RTF < 0.5 and the A4 target is < 100 ms p99
end to end; DeepFilterNet3 alone adds a fixed 30 ms. `rfwhisper bench rtf
<file.wav>` and the `denoise-live` status line give you the numbers on your own
Pi; please share them. See
[Architecture → Latency Budget](../architecture/latency-budget) for the formal
budget.

## Troubleshooting

- **Throttling** (`vcgencmd get_throttled` returns non-zero) — add a proper heatsink/fan and check your power supply delivers ≥ 27 W for the Pi 5.
- **Underruns under load**: check for throttling first, then try `--blocksize 960`.
- **`ALSA lib pcm.c` warnings at startup** — generally benign; verify audio works end-to-end before investigating.
