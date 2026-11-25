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
| [Linux](./linux.md) | Any x86_64/aarch64 from 2015+, ALSA or PipeWire | Ubuntu 22.04+ / Fedora 38+ | Best DX for hacking |
| [macOS](./macos.md) | Intel or Apple Silicon, macOS 12+ | Apple Silicon M1+ | Beta verified live on Apple Silicon |
| [Windows](./windows.md) | Windows 10/11 x64 | Windows 11 | VB-Cable for virtual routing |
| [Raspberry Pi](./raspberry-pi.md) | Pi 5 (4 GB) + active cooling | Pi 5 8 GB + NVMe HAT | Not yet verified in the beta |

## Shared prerequisites

- **Rust** (stable toolchain via [rustup](https://rustup.rs/))
- **Git** (plus `git-lfs` only if you want the real-audio sample recordings)
- A working audio stack on your OS
- (For v0.2+) GNU Radio 3.10.x with `gr-soapy` + `gr-dnn`

## Install from source (beta)

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
cargo build --release
./target/release/rfwhisper --version
./target/release/rfwhisper audio list
```

The build produces a single self-contained binary at `target/release/rfwhisper`,
with the DeepFilterNet3 model inside. Copy it anywhere on your `PATH` (no
interpreter, virtualenv, or model download needed).

Then jump to [Quick Start](../quickstart/index.md) to run it on your rig audio.

:::tip What to paste into a bug report

```bash
rfwhisper --version
rfwhisper audio list
```

plus your OS, CPU, and the last status line `denoise-live` printed.

:::
