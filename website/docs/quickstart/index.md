---
id: index
title: Quick Start
sidebar_position: 1
description: Build RFWhisper, denoise a file, then run it live on your rig audio — no downloads.
slug: /quickstart/
---

# Quick Start

From `git clone` to denoised rig audio in a few minutes. The DeepFilterNet3 model
is bundled in the binary, so there is no model download, and nothing touches the
network at runtime.

:::info Beta — read this first
This is **v0.1.0-beta.1**. The neural denoiser (DeepFilterNet3) ships and runs in
real time. Two things to know:

- It is the **stock speech model**, not yet fine-tuned on ham noise. Use it for
  **SSB / AM / FM voice**. It removes CW tones and FT8/RTTY/PSK carriers along
  with the noise, so for those modes use `--model passthrough` (or bypass) until
  the v0.3 mode profiles ship.
- It hasn't been validated on real on-air recordings yet. If you try it on the
  air, please [send us a before/after pair](#6-help-test-the-beta).
:::

## 1. Build

You need the Rust toolchain ([rustup](https://rustup.rs/)). On Linux, install the
ALSA headers first (`sudo apt install libasound2-dev` or your distro's equivalent).
See [Installation](../installation/index.md) for per-OS details.

```bash
git clone https://github.com/JakenHerman/RFWhisper.git
cd RFWhisper
cargo build --release          # single binary at target/release/rfwhisper
```

The first build takes a few minutes: it compiles the DeepFilterNet3 inference
engine and bundles the model. Put `target/release` on your `PATH` (or copy the
binary somewhere that is).

## 2. Find your sound devices

```bash
rfwhisper audio list
```

```text
audio host: CoreAudio
  0  MacBook Pro Microphone
       in: 1 ch @ 48000 Hz   out: -  [default in]
  1  MacBook Pro Speakers
       in: -   out: 2 ch @ 48000 Hz  [default out]
  2  USB Audio CODEC
       in: 2 ch @ 48000 Hz   out: 2 ch @ 48000 Hz
  3  BlackHole 2ch
       in: 2 ch @ 48000 Hz   out: 2 ch @ 48000 Hz
```

Your rig's USB sound card ("USB Audio CODEC" on most Icom / Yaesu / Kenwood rigs)
is the input. The output is your headphones, or a
[virtual audio cable](../hardware/virtual-cables.md) if you want to feed WSJT-X,
fldigi, or JS8Call.

## 3. Denoise live

```bash
# Rig audio → denoiser → your default speakers / headphones
rfwhisper denoise-live --in "USB Audio CODEC"
```

`--in` and `--out` take either the number from `audio list` or part of the
device's name. Once it's running:

| Type | Does |
|---|---|
| `b` + Enter | **A/B toggle**: raw ↔ denoised. Time-aligned and crossfaded, so you hear the same moment both ways |
| `q` + Enter or Ctrl-C | Stop |

Once a second it prints a status line:

```text
[   12s] DENOISE deepfilternet3 | latency ~73 ms (devices 21 + buffers 22 + dsp 31) | proc p50 0.5 ms p99 0.9 ms | load 5% | in-drops 0 underruns 0 drift-drops 0
```

- **latency**: estimated time from antenna audio in to denoised audio out. The
  v0.1 budget is under 100 ms.
- **load**: share of one CPU core the denoiser uses.
- **in-drops / underruns**: should stay at 0. If they climb, see
  [Troubleshooting](#troubleshooting).

### Feed a digital-mode or logging app

```bash
# Rig → denoiser → virtual cable; set WSJT-X / fldigi input to the cable
rfwhisper denoise-live --in "USB Audio CODEC" --out "BlackHole"
```

On Windows the cable's playback side is called `CABLE Input`; on Linux, a
PipeWire null sink or `snd-aloop` device. Setup per OS:
[Virtual audio cables](../hardware/virtual-cables.md).

:::caution FT8, FT4, RTTY, PSK, and CW
The beta's speech model removes these tones. Run digital modes with
`--model passthrough` (no processing) for now.
:::

### Useful flags

| Flag | What it does |
|---|---|
| `--in-channel 1` | Receive audio is on the right channel (some codecs split main/sub receiver across L/R) |
| `--bypass` | Start in raw (bypassed) mode |
| `--record-raw raw.wav --record-denoised clean.wav` | Record a sample-aligned before/after pair (48 kHz, 32-bit float) |
| `--duration 600` | Stop on its own after 10 minutes |
| `--quiet` | Hide the once-a-second status line |
| `--model passthrough` | No processing (digital modes, CW, A/B reference) |

## 4. Denoise a file

Any mono WAV, at any common sample rate:

```bash
rfwhisper denoise -i noisy.wav -o cleaned.wav --spectrogram report.html
```

`report.html` is a **self-contained** before/after page: SNR tiles, side-by-side
spectrograms on a shared scale, and a median-spectrum chart. It opens in any
browser, offline. Add `--report report.json` for a machine-readable summary, and
`--reference clean.wav` if you have the clean original (the report then includes
the measured SNR gain).

## 5. Make a test signal (optional)

No recordings handy? The fixtures are generated from a seed, so these commands
produce byte-identical audio on any machine:

```bash
rfwhisper samples list          # signals, noise types, and named presets

# An S3 SSB signal buried under an S7 powerline buzz (-24 dB SNR)
rfwhisper samples synth --preset ssb_powerline_s3_s7 \
  --out ssb.wav --clean-out ssb.clean.wav

rfwhisper denoise -i ssb.wav -o cleaned.wav \
  --reference ssb.clean.wav --report report.json --spectrogram report.html
```

:::note Synthetic "speech" understates a speech model
The synthetic SSB fixture is a formant model, not a human voice, and DeepFilterNet3
partly removes it as non-speech, so its measured gain on these fixtures is low or
even negative. The fixtures are right for timing, latency, and pipeline tests. To
judge quality, use real voice recordings.
:::

## 6. Help test the beta

The most useful thing you can send us is a real on-air before/after pair:

```bash
rfwhisper denoise-live --in "USB Audio CODEC" \
  --record-raw raw.wav --record-denoised denoised.wav
```

Listen through a few overs of a weak SSB station (toggle `b` to compare), then
[open an issue](https://github.com/JakenHerman/RFWhisper/issues/new) with the two
files (or a link), your rig, band, mode, noise source if known, OS and CPU, and
the last status line. Tell us what it got *wrong* too: warbly "underwater"
artifacts, clipped syllables, a band that sounds dead. The recording spec is in
[`samples/README.md`](https://github.com/JakenHerman/RFWhisper/tree/master/samples).

## What exists today

| Command | Status |
|---|---|
| `denoise-live` (realtime, A/B, recording, telemetry) | ✅ beta |
| `denoise` (offline WAV → WAV, JSON + HTML report) | ✅ works |
| `--model deepfilternet3` (default) | ✅ real denoiser, bundled |
| `--model passthrough` | ✅ no processing |
| `samples synth` / `samples list` | ✅ works |
| `bench latency` / `bench rtf` | ✅ works |
| `gui` | 🚧 prints a pointer; minimal GUI is [#18](https://github.com/JakenHerman/RFWhisper/issues/18) |

## Troubleshooting

| Symptom | Try |
|---|---|
| `no input device whose name contains …` | Run `rfwhisper audio list` and copy part of the exact name, or use its number |
| Silence on macOS | Allow microphone access for your terminal: System Settings → Privacy & Security → Microphone |
| `could not open … device` | Another app may hold it in exclusive mode (Windows: Sound settings → device → Advanced → untick "Allow applications to take exclusive control") |
| `underruns` climbing | Close CPU-heavy apps; try `--blocksize 960` |
| Receive audio only on one side | `--in-channel 1` |
| Warning about `SpectralStubEngine` | The binary was built with `--no-default-features`; rebuild with `cargo build --release` |

## Next: prove it properly

Want to *measure* it rather than eyeball it? See the
[v0.1 Test Guide](./v0_1-test-guide.md) for the acceptance gates and how to run them.
