---
id: intro
title: Introduction
sidebar_position: 1
description: RFWhisper is a real-time, fully local AI noise reduction tool for amateur radio, running DeepFilterNet3 on your own CPU.
slug: /
---

# Welcome to RFWhisper

> **Real-time AI denoising for ham radio. No cloud. No compromises. Just clean QSOs from the noisy RF aether.**

RFWhisper is a fully local, open-source, real-time ML-powered noise-reduction tool built specifically for amateur radio. It runs the [DeepFilterNet3](https://github.com/Rikorose/DeepFilterNet) neural denoiser on your rig's receive audio and sends the cleaned audio to your headphones or a virtual cable, so you can pull weak signals out of the modern RFI soup — **without streaming your audio to a cloud service you don't trust**. Later releases add a ham-fine-tuned model, an [RNNoise](https://github.com/xiph/rnnoise) fallback for tiny hardware, and GNU Radio + SoapySDR flowgraphs.

:::info v0.1 beta
The beta is out: real-time DeepFilterNet3 with A/B bypass, before/after recording,
and live latency telemetry, at about 75 ms end to end on a laptop. It's the stock
speech model, so it's for **voice** (SSB / AM / FM) for now; use
`--model passthrough` for CW and digital modes. See the
[Quick Start](./quickstart/index.md) and the [changelog](https://github.com/JakenHerman/RFWhisper/blob/master/CHANGELOG.md).
:::

Everything runs on your machine. Your shack. Your CPU. Your rules. <span className="rfw-73">73</span>

## What you get

- **Real-time audio** → denoiser → headphones or virtual cable → WSJT-X, fldigi, JS8Call, SDR#, Quisk. A/B bypass and before/after recording built in. *(beta)*
- **Offline WAV denoising** with before/after reports (SNR gain, spectrograms). *(beta)*
- **GNU Radio flowgraphs** for RTL-SDR, Airspy, Pluto, HackRF, SDRplay (v0.2).
- **Mode-aware profiles**: SSB, CW, FT8/FT4, RTTY, VHF FM (v0.3).
- **Before/after spectrogram UI** with live SNR/latency/CPU telemetry (v0.4).
- **Fine-tuning tools** to adapt the model to *your* noise environment (v0.5).
- **GPLv3** — inspect, modify, fork, redistribute. Forever.

## What you don't get

- A cloud dependency. Ever. Core stays 100% local.
- Silent telemetry. Anything optional is opt-in and disclosed.
- A denoiser that quietly damages your signals. The goal for v0.1 final is that no model becomes a default without passing hard gates for CW-transient preservation and FT8 decode counts. The beta is honest about where it isn't there yet: its speech model is for voice, and it says so.

## Where to next

- **New here?** → [Why RFWhisper](./why-rfwhisper.md) explains the real-world noise problem we're solving.
- **Ready to try it?** → [Quick Start](./quickstart/index.md) gets you denoising rig audio in a few minutes.
- **Want to test it properly?** → [v0.1 Test Guide](./quickstart/v0_1-test-guide.md) walks through the full acceptance suite.
- **Contributing?** → [Architecture](./architecture/index.md) and [AGENTS.md](https://github.com/jakenherman/rfwhisper/blob/master/AGENTS.md) are your entry points.

:::tip For hams in a hurry

If you know what a virtual audio cable is, install RFWhisper, route your rig audio through it, and point WSJT-X at the output. You'll probably see the value in under 10 minutes. We'll still be here when you want the deep dive.

:::
