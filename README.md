# RFWhisper

> **Real-time AI denoising for ham radio. No cloud. No compromises. Just clean QSOs from the noisy RF aether.**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](./LICENSE)
[![Status: Alpha](https://img.shields.io/badge/status-alpha-orange.svg)](./ROADMAP.md)
[![CI](https://github.com/rfwhisper/rfwhisper/actions/workflows/basic-ci.yml/badge.svg)](./.github/workflows/basic-ci.yml)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](./CONTRIBUTING.md)
[![Discord](https://img.shields.io/badge/chat-ham%20%26%20dev-purple.svg)](#community)

```
       .                 .                 .
   .  /|\    _..._     / \     _..._    /|\  .
     /_|_\ .'     '.  / _ \  .'     '. /_|_\
      |O| /   RFW   \/ /_\ \/    73!   \|O|
     [___]'.._____.' (_____) '.._____.'[___]
         RFWhisper — cleaner copy, calmer bands.
```

RFWhisper is a **fully local, open-source, real-time ML-powered noise-reduction tool** built specifically for amateur radio. It wraps a ham-tuned deep learning denoiser (**DeepFilterNet3** primary, **RNNoise** fallback) inside a GNU Radio + SoapySDR + ONNX Runtime pipeline, so you can pull weak signals out of the modern RFI soup without streaming your audio to a cloud service you don't trust.

**Everything runs on your machine.** Your shack. Your GPU. Your rules. 73.

---

## Table of Contents

- [Why RFWhisper?](#why-rfwhisper)
- [Features](#features)
- [How It Works](#how-it-works)
- [Quick Start](#quick-start)
- [Hardware Requirements](#hardware-requirements)
- [Testable Success Examples](#testable-success-examples)
- [Project Status & Roadmap](#project-status--roadmap)
- [Contributing](#contributing)
- [Community](#community)
- [License](#license)
- [73 & Acknowledgments](#73--acknowledgments)

---

## Why RFWhisper?

The amateur bands are noisier than they've ever been. If you've tried to work a POTA station on 40m from a townhouse, chased DX through a neighbour's plasma TV, or decoded FT8 next to an EV charger, you know the modern noise floor is **unrecognizable** compared to a decade ago. The culprits:

- Switch-mode power supplies and LED drivers (broadband hash, 20 kHz spurs)
- Solar inverters and MPPT controllers (rhythmic buzz across HF)
- VDSL / PLC / Ethernet-over-powerline (wideband raised noise floor)
- Plasma TVs, grow lights, touch lamps (narrowband carriers)
- Neighbor electric fences, thermostats, doorbells (pulsing crashes)
- Solar weather, static crashes, atmospheric QRN on low bands

Traditional DSP tools (analog NB/NR, classical Wiener filters, basic spectral subtraction) either:

1. Don't catch modern complex impulsive + stationary mixtures, **or**
2. Wreck the signal, destroy CW attack transients, or introduce "underwater" artifacts that confuse FT8/WSJT-X decoders.

**RFWhisper's approach:** use a deep neural network that was *designed* for speech denoising (DeepFilterNet3) and *fine-tune it on real amateur-radio noise* (recorded powerline buzz, inverter hash, static crashes) so it learns to preserve SSB phonemes, CW keying transients, and FT8 tones while nuking the QRM.

And because it's **local-first** and **open-source (GPLv3)**, you can:

- Run it airgapped on a Raspberry Pi 5 in the field
- Inspect and retrain every model
- Tune it for your exact noise environment
- Use it commercially, modify it, fork it, hack on it — as long as you keep it open.

---

## Features

### Shipping in v0.1 (Audio-Only MVP)

- [x] Real-time audio denoising with DeepFilterNet3 (ONNX Runtime)
- [x] RNNoise fallback for ultra-low-power devices (RPi Zero 2, old laptops)
- [x] Virtual audio cable output (VB-Cable on Windows, BlackHole on macOS, JACK/ALSA loopback on Linux) → drop-in for WSJT-X, fldigi, JS8Call, SDR#, SDRuno, Quisk
- [x] CLI + minimal GUI with before/after A/B toggle
- [x] Latency < 100 ms end-to-end on a modern laptop (target 40–60 ms)
- [x] Before/after WAV recording for sharing and testing
- [x] Cross-platform: Linux, macOS, Windows, Raspberry Pi 5

### Coming in v0.2–v1.0 (see [ROADMAP.md](./ROADMAP.md))

- GNU Radio 3.10 flowgraph with SoapySDR source + gr-dnn ONNX block
- Mode profiles: SSB, CW, FT8/FT4, RTTY, AM/FM, VHF FM
- Adaptive narrowband notch (carriers, birdies) that plays nicely with the NN
- Before/after waterfall + SINAD / SNR-gain / CPU / latency telemetry
- Dataset generator + fine-tuning pipeline for your noise environment
- Signed installers for Win/macOS, `.deb`/`.rpm`/AUR packages, RPi OS image
- Model hub: community-trained variants (HF contester, VHF-FM mobile, 630m etc.)
- GNU Radio 4.0 native port + plugin architecture

---

## How It Works

```
┌─────────────┐   ┌──────────┐   ┌───────────────────┐   ┌──────────┐   ┌────────────┐
│ SDR / Rig   │──▶│ SoapySDR │──▶│ GNU Radio 3.10.x  │──▶│ gr-dnn   │──▶│ Virtual    │
│ RTL, Pluto, │   │ + liquid │   │ demod (SSB/FM/...)│   │ ONNX RT  │   │ audio cable│
│ Airspy, IC- │   │ + VOLK   │   │ → 16k/48k audio   │   │ DFN3/RNN │   │ → WSJT-X,  │
│ 7300 audio  │   │          │   │                   │   │ (CPU/GPU)│   │   fldigi,  │
│             │   │          │   │                   │   │          │   │   headphns │
└─────────────┘   └──────────┘   └───────────────────┘   └──────────┘   └────────────┘
                                                              │
                                                              ▼
                                                 ┌───────────────────────────┐
                                                 │ Telemetry: SNR gain,      │
                                                 │ SINAD, latency, CPU,      │
                                                 │ spectrogram before/after  │
                                                 └───────────────────────────┘
```

**Primary denoiser: DeepFilterNet3.** A two-stage deep-filtering network trained on speech + noise. It beats RNNoise on PESQ and STOI while staying fast enough for realtime on a laptop CPU. We ship a ham-fine-tuned ONNX export and make it easy to swap in your own.

**Fallback: RNNoise.** 40 kHz-ish features, GRU-based, runs on a potato. We ship a ham-retuned build as a backup for RPi Zero-class hardware.

**Why ONNX Runtime?** Cross-platform, great CPU performance with XNNPACK/CoreML/DirectML, optional CUDA/TensorRT/ROCm on beefier rigs, and it lets the community swap in new models without recompiling anything.

---

## Quick Start

> **Status:** v0.1 audio-only MVP. GNU Radio integration lands in v0.2. See [ROADMAP.md](./ROADMAP.md).

### Prerequisites

- Python 3.10–3.12
- A working audio stack (PortAudio / WASAPI / CoreAudio / ALSA or JACK)
- (Optional) A virtual audio cable:
  - **Windows:** [VB-Cable](https://vb-audio.com/Cable/)
  - **macOS:** [BlackHole](https://existential.audio/blackhole/) (`brew install blackhole-2ch`)
  - **Linux:** PipeWire loopback, JACK, or `snd-aloop`

### Install (from source, alpha)

```bash
git clone https://github.com/rfwhisper/rfwhisper.git
cd rfwhisper
python -m venv .venv && source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -e ".[audio]"
# Pull the pre-converted ONNX models (DeepFilterNet3 + ham-tuned RNNoise)
python -m rfwhisper.models.fetch
```

### Denoise a WAV file (offline A/B)

```bash
rfwhisper denoise \
  --input  samples/noisy_40m_ssb.wav \
  --output cleaned.wav \
  --model  deepfilternet3 \
  --report report.json
```

`report.json` contains: SNR gain estimate, average inference time, RTF (real-time factor), and a before/after spectrogram PNG path.

### Real-time from microphone / rig audio

```bash
# List audio devices
rfwhisper audio list

# Pipe rig audio (input 3) → denoiser → virtual cable (output 5) in real time
rfwhisper denoise-live --in 3 --out 5 --model deepfilternet3 --blocksize 480
```

Then point WSJT-X / fldigi / JS8Call at the virtual cable as their input.

### GUI (alpha)

```bash
rfwhisper gui
```

- Pick input + output devices
- Toggle A/B bypass with a big obvious button
- Record raw + cleaned audio simultaneously for sharing
- Live latency/CPU/RTF display

---

## Hardware Requirements

RFWhisper is designed to be **ruthlessly lightweight** so field ops (POTA, SOTA, EmComm) aren't left out.

| Scenario | Minimum | Recommended |
|---|---|---|
| **RNNoise CPU-only** | RPi Zero 2 W, any x86 from 2012+ | Anything newer |
| **DeepFilterNet3 CPU-only** | RPi 5 (4 GB), Intel i5-8xxx, Apple M1 | Ryzen 5600 / i5-12xxx / M2+ |
| **DeepFilterNet3 GPU** | Any CUDA GPU (GTX 1050+), Apple Silicon (CoreML), DirectML on Windows | RTX 3060+ / M2 Pro |
| **Real-time SDR pipeline (v0.2+)** | RTL-SDR v4 + RPi 5 | Airspy HF+ / Pluto+ on a laptop |

**Supported SDRs (planned v0.2+):** RTL-SDR (all variants), Airspy (R2 / Mini / HF+), HackRF, ADALM-Pluto, SDRplay RSP1A/RSPdx (via SoapySDRPlay3), LimeSDR, USRP B-series, KiwiSDR — anything SoapySDR supports. Also: audio from IC-7300 / IC-705 / FT-991 / FTDX10 / K3 over USB CODEC.

---

## Testable Success Examples

Every milestone in this project has an **explicit, measurable success criterion**. If you can't demo it to another ham and have them say "that's clearly better", it doesn't ship. Some concrete examples for v0.1:

### Example 1: Weak 40m SSB under power-line noise

- **Setup:** Record 60 s of a DX station at S3 with S7 powerline buzz. Process with RFWhisper. Play both for 5 volunteer hams, blind.
- **Pass criterion:** ≥ 4 of 5 hams prefer the denoised version AND can copy ≥ 1 additional word per sentence on average.
- **Metric floor:** ≥ +3 dB effective SNR gain (measured via matched-filter correlation with a clean reference), no more than 0.5 point PESQ degradation on reference clean speech (no-op test).

### Example 2: FT8 decode uplift on a noisy 20m sample

- **Setup:** Replay a 15-minute FT8 cycle containing known weak stations through WSJT-X, once raw and once through RFWhisper → virtual cable.
- **Pass criterion:** Decoder recovers **at least as many** stations on the denoised pass. **Zero regressions** on strong-station decodes. Ideally +1 to +3 marginal decodes per cycle.
- **Metric floor:** No increase in false decodes. Latency added by RFWhisper ≤ 100 ms (FT8 tolerates this easily).

### Example 3: CW transient preservation

- **Setup:** Feed a 25 WPM CW recording with atmospheric QRN and static crashes.
- **Pass criterion:** `cw_decoder` (fldigi / CW Skimmer) copy accuracy **does not drop** vs raw, and operator can hear crashes reduced by ≥ 6 dB.
- **Metric floor:** RMS of keying-transient region unchanged within ±1 dB (we are not allowed to soften keying).

### Example 4: Latency budget

- **Setup:** Measure round-trip latency from audio in → denoised audio out.
- **Pass criterion:** < 100 ms p99 on an Intel i5-8xxx / Apple M1 / RPi 5 at 48 kHz. Stretch: < 50 ms by v0.3.

Scripts for all of these live in `tests/audio/` and run in CI. See [ROADMAP.md](./ROADMAP.md) for the full list.

---

## Project Status & Roadmap

RFWhisper is in **alpha**. We are shipping tiny, testable increments. The short version:

| Version | Theme | Status |
|---|---|---|
| **v0.1** | Audio-only denoiser → virtual cable | 🚧 in progress |
| **v0.2** | GNU Radio + SoapySDR flowgraph | ⏳ designed |
| **v0.3** | Mode profiles (SSB/CW/FT8/VHF) + adaptive notch | ⏳ planned |
| **v0.4** | UI: before/after waterfall + metrics | ⏳ planned |
| **v0.5** | Fine-tuning tools + dataset generator | ⏳ planned |
| **v1.0** | Polished: installers, 8+ SDRs tested, model hub | 🎯 target |
| **v1.1+** | Plugins, GR4 native, propagation extras | 💭 exploring |

See [ROADMAP.md](./ROADMAP.md) for exact deliverables, acceptance criteria, and testable success metrics for every version.

---

## Contributing

We love contributions from hams, DSP engineers, ML folks, embedded wizards, UI designers, documentation nerds, and anyone in between. Some specific callouts:

- **Hams with QRM samples:** please upload noisy recordings (any band, any mode) with metadata (rig, antenna, band, noise source if known). These are gold for training.
- **DSP engineers:** VOLK / liquid-dsp / SIMD contributions are welcome, especially for the feature-extraction stage.
- **ML engineers:** help us ship ham-tuned DFN3 / hybrid architectures, and quantized variants (INT8, FP16) for edge.
- **Radio operators on exotic hardware:** if you own a weird SDR, please run the v0.2 flowgraph and file an issue with results.
- **UI / UX designers:** the v0.4 spectrogram UI is wide open for help.

Read [CONTRIBUTING.md](./CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md) first. When working with AI coding assistants, point them at [AGENTS.md](./AGENTS.md) — it has specialized system prompts for DSP, ML, GNU Radio, embedded, UI, and ham-domain work.

**First-time contributor?** Look for issues tagged `good-first-issue` or `help-wanted`.

---

## Community

- **GitHub Discussions:** design debates, RF samples, benchmark reports.
- **Issues:** bugs, feature requests, hardware compatibility reports.
- **Matrix / Discord:** *(TBD — link when community room spins up)*
- **On the air:** if you use RFWhisper for a QSO and it pulls a station out of the mud, drop a note in Discussions — that's the best feedback we get.

---

## License

RFWhisper is licensed under the **GNU General Public License v3.0 or later** — see [LICENSE](./LICENSE).

This means:

- You can use, modify, and redistribute it freely.
- If you ship modifications, you must ship the source.
- No warranty, no liability, de-facto "use at your own risk" — see full text.

We chose GPLv3 specifically to keep the ham radio ecosystem healthy: improvements benefit everyone, and nobody can take the project closed-source and wall it off from the operators who made it possible.

Dependencies retain their own licenses (GNU Radio: GPLv3; SoapySDR: Boost; ONNX Runtime: MIT; DeepFilterNet: MIT/Apache-2.0; RNNoise: BSD-3-Clause; liquid-dsp: MIT; VOLK: GPLv3).

---

## 73 & Acknowledgments

RFWhisper stands on the shoulders of giants:

- **GNU Radio** — the DSP framework this project could not exist without.
- **SoapySDR** — making "works with any SDR" actually true.
- **ONNX Runtime** — honest, fast, cross-platform inference.
- **DeepFilterNet** (Schröter et al.) — the model architecture we're building on.
- **RNNoise** (Jean-Marc Valin / Xiph.Org) — the granddaddy of real-time neural denoising.
- **VOLK / liquid-dsp** — speed when we need it.
- **The amateur radio community** — for 100+ years of hacker culture, open specs, and the patience to teach newcomers.

*To every ham who has ever pulled a signal out of the noise with a shrug and a paper pad: this is for you. We're just automating the superpower you already had.*

**73 de the RFWhisper project.**

*(If this tool helps you make a contact you'd otherwise have missed — POTA, DX, emergency traffic, a friend's ragchew — please tell us. Stories like that are why we're building this.)*
