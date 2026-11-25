---
id: macos
title: macOS
sidebar_position: 3
description: Install RFWhisper on macOS (Intel or Apple Silicon).
---

# Install on macOS

Built and unit-tested in CI on Apple Silicon (macos-14). The beta has been run live on an Apple Silicon laptop; Intel Macs should work but are untested.

## 1. Install Homebrew (if needed)

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

## 2. System dependencies

```bash
brew install rustup git
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

Use **BlackHole 2ch** as RFWhisper's output device (`--out "BlackHole"`) and as WSJT-X's input device. Full walkthrough: [Virtual audio cables](../hardware/virtual-cables).

## 4. RFWhisper

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
cargo build --release        # a few minutes the first time: builds DeepFilterNet3 in
sudo install -m755 target/release/rfwhisper /usr/local/bin/
rfwhisper --version          # rfwhisper 0.1.0-beta.1
rfwhisper audio list         # your sound devices
```

The DeepFilterNet3 model is bundled in the binary: there is nothing to download
and no network access at runtime.

## 5. Performance

The beta runs DeepFilterNet3 on the CPU (no CoreML needed). On an Apple Silicon
laptop it uses about 5 % of one core, and `denoise-live` estimates about 75 ms end
to end (30 ms of that is the model's own look-ahead).

## 6. Privacy & mic permissions

macOS will prompt for **microphone access** the first time you run `rfwhisper denoise-live`. If you miss the prompt:

- **System Settings → Privacy & Security → Microphone** — allow Terminal (or your editor).

## Troubleshooting

- **No sound through BlackHole**: make sure RFWhisper's `--out` is BlackHole (or your Multi-Output Device) and the decoder's input is `BlackHole 2ch`. Sample rates don't need to match; RFWhisper resamples.
- **`cargo: command not found` after install** — restart your shell or `source "$HOME/.cargo/env"`.
- **Rosetta**: not required. Build natively on Apple Silicon.
