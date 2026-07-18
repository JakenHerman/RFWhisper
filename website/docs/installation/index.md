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

- **Rust** (stable toolchain via [rustup](https://rustup.rs/))
- **Git** with `git-lfs` for samples/models
- A working audio stack on your OS
- (For v0.2+) GNU Radio 3.10.x with `gr-soapy` + `gr-dnn`

## One-liner install (from source, alpha)

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
cargo build --release
./target/release/rfwhisper models fetch
./target/release/rfwhisper --version
```

The build produces a single self-contained binary at `target/release/rfwhisper` — copy it anywhere on your `PATH` (no interpreter or virtualenv needed).

Then jump to [Quick Start](../quickstart/) to denoise your first file.

:::tip Verifying the install

```bash
rfwhisper doctor
```

Runs a diagnostic checklist: build info, ONNX Runtime providers, audio backend, model SHA-256s, and platform-specific quirks. Paste the output into any bug report.

:::
