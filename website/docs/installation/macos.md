---
id: macos
title: macOS
sidebar_position: 3
description: Install RFWhisper on macOS (Intel or Apple Silicon) with CoreML acceleration.
---

# Install on macOS

Tested on macOS 13 (Ventura), 14 (Sonoma), and 15 (Sequoia) on Intel and Apple Silicon.

## 1. Install Homebrew (if needed)

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

## 2. System dependencies

```bash
brew install rustup git git-lfs ffmpeg pkg-config
rustup-init -y                # Rust stable toolchain
brew install blackhole-2ch    # virtual audio cable
```

For v0.2+ GNU Radio flowgraphs:

```bash
brew install gnuradio soapysdr
# gr-dnn ships as a wheel in v0.2; see Architecture → Flowgraphs
```

## 3. Virtual cable setup

[BlackHole](https://existential.audio/blackhole/) is the de-facto free virtual cable on macOS.

1. After `brew install blackhole-2ch`, open **Audio MIDI Setup** (`⌘+Space → "Audio MIDI Setup"`).
2. Click **+** → **Create Multi-Output Device**.
3. Check both **BlackHole 2ch** and your **MacBook / external speakers**.
4. Rename it to something obvious like *"RFW-Listen"*.

Use **BlackHole 2ch** as RFWhisper's output device and as WSJT-X's input device.

## 4. RFWhisper

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
cargo build --release
sudo install -m755 target/release/rfwhisper /usr/local/bin/
rfwhisper models fetch
rfwhisper doctor
```

## 5. Apple Silicon: CoreML acceleration

When the ONNX (DFN3) backend is enabled, RFWhisper prefers the CoreML execution provider on Apple Silicon automatically:

```bash
rfwhisper info providers
# CoreMLExecutionProvider   (preferred on M-series)
# CPUExecutionProvider      (fallback)
```

This typically halves inference latency vs CPU-only for DeepFilterNet3.

## 6. Privacy & mic permissions

macOS will prompt for **microphone access** the first time you run `rfwhisper denoise-live`. If you miss the prompt:

- **System Settings → Privacy & Security → Microphone** — allow Terminal (or your editor).

## Troubleshooting

- **No sound through BlackHole** — verify the Multi-Output Device is selected as system output; check that input/output sample rates match (48 kHz is our default).
- **`cargo: command not found` after install** — restart your shell or `source "$HOME/.cargo/env"`.
- **Rosetta** — not required. Build natively on Apple Silicon; you'll keep CoreML eligibility.
