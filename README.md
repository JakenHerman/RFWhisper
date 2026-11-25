<p align="center">
  <a href="https://rfwhisper.org" title="RFWhisper documentation">
    <img src="website/static/img/logo.svg" width="120" height="120" alt="RFWhisper logo" />
  </a>
</p>

# RFWhisper

> **Real-time AI denoising for ham radio. No cloud. No compromises. Just clean QSOs from the noisy RF aether.**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](./LICENSE)
[![Status: Beta](https://img.shields.io/badge/status-beta-yellow.svg)](./CHANGELOG.md)
[![CI](https://github.com/jakenherman/rfwhisper/actions/workflows/basic-ci.yml/badge.svg)](./.github/workflows/basic-ci.yml)
[![Docs](https://img.shields.io/badge/docs-rfwhisper.org-0d3b66)](https://rfwhisper.org)
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

RFWhisper is a **fully local, open-source, real-time ML-powered noise-reduction tool** built specifically for amateur radio. It runs the **DeepFilterNet3** neural denoiser on your rig's receive audio and hands the cleaned audio to your headphones or a virtual audio cable, so you can pull weak signals out of the modern RFI soup without streaming your audio to a cloud service you don't trust. Later releases add a ham-fine-tuned model, an RNNoise fallback for tiny hardware, and a GNU Radio + SoapySDR pipeline.

**Everything runs on your machine.** Your shack. Your GPU. Your rules. 73.

> ### Beta — v0.1.0-beta.1
>
> The neural denoiser now ships. `rfwhisper denoise` (files) and
> `rfwhisper denoise-live` (rig audio → headphones or virtual cable) run
> **DeepFilterNet3** in real time: about 30 ms of processing delay plus your
> sound card's buffers, typically **about 75 ms end to end** on a laptop, using
> about 5 % of one core.
>
> What "beta" means here:
>
> - **It is the stock DeepFilterNet3 speech model, not yet fine-tuned on ham
>   noise.** It is very good on **SSB / AM / FM voice**. It is a speech model, so
>   it treats CW tones and FT8/RTTY/PSK carriers as noise and removes them. For
>   those modes use `--model passthrough` (or press `b` to bypass) until the
>   v0.3 mode profiles land.
> - It has been measured on synthetic fixtures and a laptop mic. It **has not
>   yet been validated on real on-air ham recordings.** That's the most useful
>   thing a beta tester can do: see [Beta testing](#beta-testing).
> - CLI only. The GUI is planned ([#18](https://github.com/JakenHerman/RFWhisper/issues/18)).

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

### Working today (beta)

- [x] **Real-time denoising with DeepFilterNet3**: rig / mic audio → denoiser → headphones or virtual cable (`rfwhisper denoise-live`)
- [x] **A/B bypass**: press `b` + Enter to hear raw vs. denoised. The two are time-aligned and crossfaded, so you compare the same moment of audio
- [x] **Before/after recording**: `--record-raw` / `--record-denoised` write a sample-aligned pair of WAVs
- [x] Live telemetry once a second: end-to-end latency estimate (with a breakdown), processing time p50/p99, CPU load, dropouts
- [x] Works with mismatched sound cards (e.g. a 44.1 kHz USB codec feeding a 48 kHz virtual cable). Picks devices by number or name, and lets you choose which input channel carries the receive audio
- [x] Offline WAV denoise (`rfwhisper denoise`) with a JSON metrics report and a self-contained before/after spectrogram HTML page
- [x] `--model passthrough` for CW and digital modes (no processing)
- [x] Deterministic test-signal synthesis (`rfwhisper samples synth`): SSB / CW / FT8 / VHF-FM signals with powerline / inverter / VDSL / QRN noise
- [x] Measurement harness: matched-filter SNR gain, keying-onset RMS, RTF, and a streaming latency probe (`rfwhisper bench`)
- [x] Cross-platform: Linux, macOS, Windows (Raspberry Pi 5 expected to work, not yet verified)

### Still to come for v0.1 final

- [ ] Validation on real on-air recordings: SNR gain (A1), FT8 decode non-regression (A2), CW transients (A3). See [#47](https://github.com/JakenHerman/RFWhisper/issues/47) / [#68](https://github.com/JakenHerman/RFWhisper/issues/68)
- [ ] Hardware loopback latency measurement (A4) — [#15](https://github.com/JakenHerman/RFWhisper/issues/15) / [#22](https://github.com/JakenHerman/RFWhisper/issues/22)
- [ ] RNNoise fallback for ultra-low-power devices (RPi Zero 2, old laptops) — [#11](https://github.com/JakenHerman/RFWhisper/issues/11)
- [ ] Minimal GUI with A/B toggle — [#18](https://github.com/JakenHerman/RFWhisper/issues/18) (`rfwhisper gui` currently prints a pointer)
- [ ] Raspberry Pi 5 verified
- [ ] Ham-noise fine-tuned model (v0.5 pipeline; the beta uses stock DeepFilterNet3)

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

The diagram is the v0.2+ target architecture. Today the chain is
`WAV or audio device → resample to 48 kHz → DeepFilterNet3 → resample → WAV or
audio device`; the SDR and GNU Radio legs are v0.2
([#29](https://github.com/JakenHerman/RFWhisper/issues/29)–[#38](https://github.com/JakenHerman/RFWhisper/issues/38)).

**Primary denoiser: DeepFilterNet3 (shipping).** A two-stage deep-filtering
network for full-band 48 kHz audio. It beats RNNoise on PESQ and STOI and is fast
enough for real time on a laptop CPU (about 0.1–0.5 ms per 10 ms frame on an Apple
Silicon laptop). The beta runs upstream's own Rust inference engine (`deep_filter`, on
the pure-Rust `tract` runtime) with the stock DeepFilterNet3 weights bundled in
the binary: nothing to download, no network at runtime. A ham-fine-tuned model
comes with the v0.5 training pipeline.

**Planned fallback: RNNoise.** GRU-based, runs on a potato. Intended as the
backup for RPi Zero-class hardware
([#11](https://github.com/JakenHerman/RFWhisper/issues/11)).

**Also included:** `passthrough` (no processing, for CW / digital modes and A/B
reference) and `spectral_stub` (a classical spectral-subtraction test path used by
CI and by `--no-default-features` builds).

**Why upstream's Rust engine instead of ONNX Runtime?** DeepFilterNet's own
realtime reference implementation *is* that crate, so we inherit its exact
feature extraction and tuned model and skip the export-parity risk of a
third-party ONNX conversion. The ONNX model registry and execution-provider
selection stay in the codebase for custom and community models later.

**Why Rust?** The runtime (DSP, realtime audio, CLI) is a single static binary — no interpreter, no virtualenv, no GC pauses in the audio path. `cargo build --release` and you can drop it on a Pi in a POTA tent. Model *training* stays in Python/PyTorch where the ML ecosystem lives; the two meet at the model boundary.

---

## Quick Start

> **Status:** v0.1 beta, audio only. GNU Radio integration lands in v0.2. See [CHANGELOG.md](./CHANGELOG.md) and [ROADMAP.md](./ROADMAP.md).

### Prerequisites

- The Rust toolchain (stable, via [rustup](https://rustup.rs/))
- A working audio stack (WASAPI / CoreAudio / ALSA or JACK; `libasound2-dev` to build on Linux)
- (Optional) A virtual audio cable:
  - **Windows:** [VB-Cable](https://vb-audio.com/Cable/)
  - **macOS:** [BlackHole](https://existential.audio/blackhole/) (`brew install blackhole-2ch`)
  - **Linux:** PipeWire loopback, JACK, or `snd-aloop`

### Install (from source)

```bash
git clone https://github.com/jakenherman/rfwhisper.git
cd rfwhisper
cargo build --release          # single binary at target/release/rfwhisper
```

The first build takes a few minutes: it compiles the DeepFilterNet3 inference
engine and bundles the model into the binary. Nothing is downloaded at runtime.
Add `target/release` to your `PATH`, or copy `target/release/rfwhisper` somewhere
on it.

`cargo build --release --no-default-features` builds a lean binary without the
neural denoiser (CI and embedding only; `deepfilternet3` then falls back to a
classical stub and says so).

### Make yourself a noisy signal

No downloads and no sample pack: the fixtures are generated from a seed, so these
commands produce byte-identical audio on any machine.

```bash
rfwhisper samples list          # available signals, noise types, and presets

# An S3 SSB signal buried under an S7 powerline buzz (-24 dB SNR).
# --out is the noisy mix; --clean-out is the reference it was mixed from.
rfwhisper samples synth --preset ssb_powerline_s3_s7 \
  --out ssb.wav --clean-out ssb.clean.wav
```

### Denoise it (offline A/B)

```bash
rfwhisper denoise \
  --input       ssb.wav \
  --output      cleaned.wav \
  --reference   ssb.clean.wav \
  --report      report.json \
  --spectrogram report.html
```

`report.json` carries the measured SNR gain against the reference, inference time,
and RTF; `report.html` is a self-contained before/after spectrogram page. Your own
recordings work too: any mono WAV, any common sample rate (drop `--reference` if
you have no clean copy).

> Heads-up: DeepFilterNet3 is a *speech* model and the synthetic "SSB" fixture is
> a formant model, not a real voice, so synthetic SNR numbers understate it (see
> [#117](https://github.com/JakenHerman/RFWhisper/issues/117)). Judge it on real
> recordings.

### Real-time from rig audio

```bash
rfwhisper audio list            # numbered list of sound devices

# Rig USB codec → denoiser → virtual cable (devices by number or by name)
rfwhisper denoise-live --in "USB Audio CODEC" --out "CABLE Input"

# Or straight to your headphones, recording a before/after pair as you go
rfwhisper denoise-live --in 3 --record-raw raw.wav --record-denoised clean.wav
```

While it runs: type `b` + Enter to toggle the A/B bypass, `q` + Enter (or
Ctrl-C) to stop. A status line every second shows the latency estimate and any
dropouts. Then point WSJT-X / fldigi / JS8Call at the virtual cable as their input
(for digital modes add `--model passthrough`; see the beta notes above).

Useful flags: `--in-channel 1` if your receive audio is on the right channel,
`--bypass` to start in raw mode, `--duration 600` to stop on its own, `--quiet`
to hide the status line. Virtual cable setup per OS:
[rfwhisper.org/docs/hardware/virtual-cables](https://rfwhisper.org/docs/hardware/virtual-cables).

### GUI

Not built yet — `rfwhisper gui` prints a pointer to `denoise-live`. Device picker,
A/B bypass, dual-stream recording, and live telemetry are all in `denoise-live`
today; the window around them is
[#18](https://github.com/JakenHerman/RFWhisper/issues/18) and the v0.4 GUI epic
([#70](https://github.com/JakenHerman/RFWhisper/issues/70)–[#78](https://github.com/JakenHerman/RFWhisper/issues/78)).

### Beta testing

The most valuable thing you can send is a **real on-air recording** with RFWhisper
on and off:

```bash
rfwhisper denoise-live --in <rig> --record-raw raw.wav --record-denoised denoised.wav
```

Let it run through a few overs of a weak SSB station (toggle `b` to listen to
both), then file an issue with the two WAVs (or a link), your rig, band, mode,
noise source if you know it, OS and CPU, and the last status line it printed.
Tell us what it did *wrong*, too: warbly "underwater" artifacts, chopped
syllables, a band that sounds dead. See
[samples/README.md](./samples/README.md) for the recording spec.

---

## Hardware Requirements

RFWhisper is designed to be **ruthlessly lightweight** so field ops (POTA, SOTA, EmComm) aren't left out.

| Scenario | Minimum | Recommended |
|---|---|---|
| **RNNoise CPU-only** *(planned, #11)* | RPi Zero 2 W, any x86 from 2012+ | Anything newer |
| **DeepFilterNet3 CPU-only** | RPi 5 (4 GB), Intel i5-8xxx, Apple M1 | Ryzen 5600 / i5-12xxx / M2+ |
| **DeepFilterNet3 GPU** *(planned; the beta is CPU-only and does not need it)* | Any CUDA GPU (GTX 1050+), Apple Silicon (CoreML), DirectML on Windows | RTX 3060+ / M2 Pro |
| **Real-time SDR pipeline (v0.2+)** | RTL-SDR v4 + RPi 5 | Airspy HF+ / Pluto+ on a laptop |

**Supported SDRs (planned v0.2+):** RTL-SDR (all variants), Airspy (R2 / Mini / HF+), HackRF, ADALM-Pluto, SDRplay RSP1A/RSPdx (via SoapySDRPlay3), LimeSDR, USRP B-series, KiwiSDR — anything SoapySDR supports. Also: audio from IC-7300 / IC-705 / FT-991 / FTDX10 / K3 over USB CODEC.

---

## Testable Success Examples

Every milestone in this project has an **explicit, measurable success criterion**. If you can't demo it to another ham and have them say "that's clearly better", it doesn't ship. Some concrete examples for v0.1.

**These are the definition of done for v0.1 final, not claims about the beta.** The harness that measures them exists and the real denoiser now runs. A5 (RTF < 0.5) passes, and the pipeline's own latency is about 40 ms before sound-card buffers. The quality criteria still need real on-air recordings to measure honestly (synthetic fixtures understate a speech model; see [#117](https://github.com/JakenHerman/RFWhisper/issues/117)).

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

The metrics behind these live in `src/dsp/metrics.rs`, the fixtures they run against in `src/samples/`, and the tests in `tests/`. Gate tests are `#[ignore]`-marked and run with `cargo test --release -- --ignored gate_`. See [ROADMAP.md](./ROADMAP.md) for the full list.

---

## Project Status & Roadmap

RFWhisper is in **beta** (v0.1.0-beta.1). We are shipping tiny, testable increments. The short version:

| Version | Theme | Status |
|---|---|---|
| **v0.1** | Audio-only denoiser → virtual cable | 🧪 beta |
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

Dependencies retain their own licenses (DeepFilterNet + its model: MIT/Apache-2.0; tract: MIT/Apache-2.0; cpal: Apache-2.0; GNU Radio: GPLv3; SoapySDR: Boost; ONNX Runtime: MIT; RNNoise: BSD-3-Clause; liquid-dsp: MIT; VOLK: GPLv3).

---

## 73 & Acknowledgments

RFWhisper stands on the shoulders of giants:

- **GNU Radio** — the DSP framework this project could not exist without.
- **SoapySDR** — making "works with any SDR" actually true.
- **ONNX Runtime** — honest, fast, cross-platform inference.
- **DeepFilterNet** (Schröter et al.) — the model, and the Rust inference engine the beta runs on.
- **RNNoise** (Jean-Marc Valin / Xiph.Org) — the granddaddy of real-time neural denoising.
- **VOLK / liquid-dsp** — speed when we need it.
- **The amateur radio community** — for 100+ years of hacker culture, open specs, and the patience to teach newcomers.

*To every ham who has ever pulled a signal out of the noise with a shrug and a paper pad: this is for you. We're just automating the superpower you already had.*

**73 de the RFWhisper project.**

*(If this tool helps you make a contact you'd otherwise have missed — POTA, DX, emergency traffic, a friend's ragchew — please tell us. Stories like that are why we're building this.)*
