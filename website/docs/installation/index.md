---
id: index
title: Installation Overview
sidebar_position: 1
description: Install RFWhisper on Linux, macOS, Windows, or Raspberry Pi.
slug: /installation/
---

# Installation

RFWhisper runs on **Linux, macOS, Windows, and Raspberry Pi 5**. Pick your platform below.

| Platform | Minimum | Recommended | Notes |
|---|---|---|---|
| [Linux](./linux) | Any x86_64/aarch64 from 2015+, ALSA or PipeWire | Ubuntu 22.04+ / Fedora 38+ | Best DX for hacking |
| [macOS](./macos) | Intel or Apple Silicon, macOS 12+ | Apple Silicon M1+ | CoreML provider gives you a free perf boost |
| [Windows](./windows) | Windows 10/11 x64 | Windows 11 with DirectML | VB-Cable required for virtual routing |
| [Raspberry Pi](./raspberry-pi) | Pi 5 (4 GB) + active cooling | Pi 5 8 GB + NVMe HAT | RNNoise fallback for Pi 4 / Zero 2 W |

## Shared prerequisites

- **Python 3.10 / 3.11 / 3.12** (3.12 recommended)
- **Git** with `git-lfs` for samples/models
- A working audio stack on your OS
- (For v0.2+) GNU Radio 3.10.x with `gr-soapy` + `gr-dnn`

## One-liner install (from source, alpha)

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
python -m venv .venv && source .venv/bin/activate
pip install -e ".[audio]"
python -m rfwhisper.models.fetch
rfwhisper --version
```

Then jump to [Quick Start](../quickstart/) to denoise your first file.

:::tip Verifying the install

```bash
rfwhisper doctor
```

Runs a diagnostic checklist: Python version, ONNX Runtime providers, audio backend, model SHA-256s, and platform-specific quirks. Paste the output into any bug report.

:::
