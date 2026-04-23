---
id: intro
title: Introduction
sidebar_position: 1
description: RFWhisper is a real-time, fully local AI noise reduction tool for amateur radio — DeepFilterNet3 + GNU Radio + SoapySDR + ONNX Runtime.
slug: /
---

# Welcome to RFWhisper

> **Real-time AI denoising for ham radio. No cloud. No compromises. Just clean QSOs from the noisy RF aether.**

RFWhisper is a fully local, open-source, real-time ML-powered noise-reduction tool built specifically for amateur radio. It wraps a ham-tuned deep learning denoiser ([DeepFilterNet3](https://github.com/Rikorose/DeepFilterNet) primary, [RNNoise](https://github.com/xiph/rnnoise) fallback) inside a GNU Radio + SoapySDR + ONNX Runtime pipeline so you can pull weak signals out of the modern RFI soup — **without streaming your audio to a cloud service you don't trust**.

Everything runs on your machine. Your shack. Your GPU. Your rules. <span className="rfw-73">73</span>

## What you get

- **Offline WAV denoising** with before/after reports (SNR gain, spectrograms, decoder pass-through counts).
- **Real-time audio** → virtual cable → WSJT-X, fldigi, JS8Call, SDR#, Quisk.
- **GNU Radio flowgraphs** for RTL-SDR, Airspy, Pluto, HackRF, SDRplay (v0.2).
- **Mode-aware profiles**: SSB, CW, FT8/FT4, RTTY, VHF FM (v0.3).
- **Before/after spectrogram UI** with live SNR/latency/CPU telemetry (v0.4).
- **Fine-tuning tools** to adapt the model to *your* noise environment (v0.5).
- **GPLv3** — inspect, modify, fork, redistribute. Forever.

## What you don't get

- A cloud dependency. Ever. Core stays 100% local.
- Silent telemetry. Anything optional is opt-in and disclosed.
- A denoiser that damages your signals. Every model candidate ships only after passing hard CI gates for CW-transient preservation and FT8 decode counts.

## Where to next

- **New here?** → [Why RFWhisper](./why-rfwhisper) explains the real-world noise problem we're solving.
- **Ready to try it?** → [Quick Start](./quickstart/) gets you denoising a WAV in under 5 minutes.
- **Want to test it properly?** → [v0.1 Test Guide](./quickstart/v0_1-test-guide) walks through the full acceptance suite.
- **Contributing?** → [Architecture](./architecture/) and [AGENTS.md](https://github.com/jakenherman/rfwhisper/blob/main/AGENTS.md) are your entry points.

:::tip For hams in a hurry

If you know what a virtual audio cable is, install RFWhisper, route your rig audio through it, and point WSJT-X at the output. You'll probably see the value in under 10 minutes. We'll still be here when you want the deep dive.

:::
