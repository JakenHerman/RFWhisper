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
brew install python@3.12 git git-lfs portaudio libsndfile ffmpeg cmake pkg-config
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
python3.12 -m venv .venv && source .venv/bin/activate
pip install -U pip wheel
pip install -e ".[audio]"
python -m rfwhisper.models.fetch
rfwhisper doctor
```

## 5. Apple Silicon: CoreML acceleration

`onnxruntime` on Apple Silicon includes the CoreML execution provider automatically. RFWhisper will prefer it when available:

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

- **`OSError: PortAudio library not found`** — `brew reinstall portaudio` and ensure your shell picks up `/opt/homebrew/bin` on the PATH (Apple Silicon) or `/usr/local/bin` (Intel).
- **No sound through BlackHole** — verify the Multi-Output Device is selected as system output; check that input/output sample rates match (48 kHz is our default).
- **`arm64` vs `x86_64` pip conflicts** — create your venv with the native Python (`which python3.12` should end in `/opt/homebrew/` on Apple Silicon).
- **Rosetta** — not required. Do not install x86_64 Python on Apple Silicon; you'll lose CoreML.
